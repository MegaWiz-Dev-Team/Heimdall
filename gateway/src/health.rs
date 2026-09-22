/// Health check endpoints.

use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    routing::get,
    Router,
};
use serde::Serialize;

use crate::AppState;

/// Budget for the backend probe inside GET /health.
///
/// The shared `http_client` carries a 300 s timeout sized for long
/// generations. A health probe must not inherit that: when mlx_lm is paged
/// out or GIL-starved mid-generation, /v1/models can take many seconds and
/// the watchdog's 4 s curl gives up first — reporting the *gateway* as down
/// while it is serving fine (INC-2026-09-22). Answer "degraded" fast instead.
pub const BACKEND_PROBE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(2);

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub gateway: String,
    pub backend: BackendHealth,
    pub timestamp: String,
}

#[derive(Serialize)]
pub struct BackendHealth {
    pub url: String,
    pub status: String,
    pub models: Option<serde_json::Value>,
}

/// GET /health — Check gateway + backend health
async fn health_check(State(state): State<AppState>) -> (StatusCode, Json<HealthResponse>) {
    let backend_url = state.config.backend_url();
    let models_url = format!("{}/v1/models", backend_url);

    // Try to reach the backend
    let (backend_status, models) = match state
        .http_client
        .get(&models_url)
        .timeout(BACKEND_PROBE_TIMEOUT)
        .send()
        .await
    {
        Ok(resp) if resp.status().is_success() => {
            let body = resp.json::<serde_json::Value>().await.ok();
            ("healthy".to_string(), body)
        }
        Ok(resp) => (format!("unhealthy (status: {})", resp.status()), None),
        Err(e) => (format!("unreachable ({})", e), None),
    };

    let overall_status = if backend_status == "healthy" {
        "healthy"
    } else {
        "degraded"
    };

    let status_code = if overall_status == "healthy" {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    let response = HealthResponse {
        status: overall_status.to_string(),
        gateway: "healthy".to_string(),
        backend: BackendHealth {
            url: backend_url,
            status: backend_status,
            models,
        },
        timestamp: chrono::Utc::now().to_rfc3339(),
    };

    (status_code, Json(response))
}

/// GET /ready — Simple readiness probe
async fn readiness() -> StatusCode {
    StatusCode::OK
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/health", get(health_check))
        .route("/ready", get(readiness))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppConfig;
    use std::sync::Arc;
    use std::time::Instant;
    use tokio::net::TcpListener;

    fn state_with_backend_port(port: u16) -> AppState {
        AppState {
            config: Arc::new(AppConfig {
                host: "0.0.0.0".into(),
                gateway_port: 3000,
                backend_host: "127.0.0.1".into(),
                backend_port: port,
                embedding_host: "127.0.0.1".into(),
                embedding_port: 8001,
                vlm_q4_port: 8082,
                vlm_q8_port: 8083,
                api_keys: vec![],
                auth_enabled: false,
                llm_model: "".into(),
                project_dir: "../".into(),
                openrouter_api_key: None,
                openrouter_base_url: "https://openrouter.ai/api/v1".into(),
                gemini_api_key: None,
                gemini_base_url: "https://generativelanguage.googleapis.com/v1beta/openai".into(),
                openai_api_key: None,
                openai_base_url: "https://api.openai.com/v1".into(),
                yggdrasil_issuer: None,
                jwt_audience: None,
            }),
            // Same 300 s client production uses — the probe must not inherit it.
            http_client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(300))
                .build()
                .unwrap(),
            active_model: Arc::new(std::sync::RwLock::new(String::new())),
            swap_lock: Arc::new(tokio::sync::Mutex::new(())),
            admission: Arc::new(crate::admission::Admission::from_env()),
            tenant_cfg: None,
            jwt_validator: None,
        }
    }

    /// A backend that accepts the TCP connection but never answers — what a
    /// paged-out mlx_lm looks like from the gateway's side.
    #[tokio::test]
    async fn health_reports_degraded_within_probe_budget_when_backend_hangs() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let _hold = tokio::spawn(async move {
            let mut held = Vec::new();
            loop {
                if let Ok((sock, _)) = listener.accept().await {
                    held.push(sock); // keep it open, say nothing
                }
            }
        });

        let started = Instant::now();
        let (status, Json(body)) = health_check(State(state_with_backend_port(port))).await;
        let elapsed = started.elapsed();

        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(body.status, "degraded");
        assert_eq!(body.gateway, "healthy");
        assert!(
            body.backend.status.starts_with("unreachable"),
            "backend status was {:?}",
            body.backend.status
        );
        assert!(
            elapsed < BACKEND_PROBE_TIMEOUT + std::time::Duration::from_secs(2),
            "health took {elapsed:?}, should be bounded by the {BACKEND_PROBE_TIMEOUT:?} probe budget"
        );
    }

    #[tokio::test]
    async fn health_is_healthy_when_backend_answers() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let _srv = tokio::spawn(async move {
            use tokio::io::{AsyncReadExt, AsyncWriteExt};
            loop {
                if let Ok((mut sock, _)) = listener.accept().await {
                    let mut buf = [0u8; 1024];
                    let _ = sock.read(&mut buf).await;
                    let body = r#"{"object":"list","data":[]}"#;
                    let resp = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        body.len(),
                        body
                    );
                    let _ = sock.write_all(resp.as_bytes()).await;
                }
            }
        });

        let (status, Json(body)) = health_check(State(state_with_backend_port(port))).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body.status, "healthy");
        assert!(body.backend.models.is_some());
    }
}
