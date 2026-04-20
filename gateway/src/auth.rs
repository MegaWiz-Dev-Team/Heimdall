/// API Key authentication middleware.

use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};

use crate::AppState;

/// Middleware that checks for valid API key in the Authorization header.
/// Skips auth for /health and /metrics endpoints.
/// If no API keys are configured, all requests pass through.
pub async fn auth_middleware(
    State(state): State<AppState>,
    request: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    // Skip auth for UI and health endpoints
    let path = request.uri().path();
    if path == "/" || path == "/health" || path == "/metrics" || path == "/ready" || path == "/api/gpu" {
        return Ok(next.run(request).await);
    }

    // If no API keys configured, skip auth
    if !state.config.auth_enabled {
        return Ok(next.run(request).await);
    }

    // Extract Bearer token
    let auth_header = request
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok());

    match auth_header {
        Some(header) if header.starts_with("Bearer ") => {
            let token = &header[7..];
            if state.config.api_keys.contains(&token.to_string()) {
                metrics::counter!("auth_success_total").increment(1);
                Ok(next.run(request).await)
            } else {
                metrics::counter!("auth_failure_total").increment(1);
                tracing::warn!("Invalid API key attempt");
                Err(StatusCode::UNAUTHORIZED)
            }
        }
        _ => {
            metrics::counter!("auth_failure_total").increment(1);
            Err(StatusCode::UNAUTHORIZED)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppConfig;
    use axum::{body::Body, http::Request, routing::get, Router, middleware};
    use std::sync::Arc;
    use tower::ServiceExt;

    fn test_state(keys: Vec<String>) -> AppState {
        AppState {
            config: Arc::new(AppConfig {
                host: "0.0.0.0".into(),
                gateway_port: 3000,
                backend_host: "127.0.0.1".into(),
                backend_port: 8000,
                embedding_host: "127.0.0.1".into(),
                embedding_port: 8001,
                api_keys: keys.clone(),
                auth_enabled: !keys.is_empty(),
                llm_model: "".into(),
                project_dir: "../".into(),
                openrouter_api_key: None,
                openrouter_base_url: "https://openrouter.ai/api/v1".into(),
                gemini_api_key: None,
                gemini_base_url: "https://generativelanguage.googleapis.com/v1beta/openai".into(),
                openai_api_key: None,
                openai_base_url: "https://api.openai.com/v1".into(),
            }),
            http_client: reqwest::Client::new(),
            active_model: Arc::new(std::sync::RwLock::new(String::new())),
            swap_lock: Arc::new(tokio::sync::Mutex::new(())),
        }
    }

    async fn ok_handler() -> &'static str {
        "ok"
    }

    fn build_app(state: AppState) -> Router {
        Router::new()
            .route("/test", get(ok_handler))
            .route("/health", get(ok_handler))
            .layer(middleware::from_fn_with_state(state.clone(), auth_middleware))
            .with_state(state)
    }

    #[tokio::test]
    async fn test_no_auth_configured_passes_all() {
        let app = build_app(test_state(vec![]));

        let response = app
            .oneshot(Request::builder().uri("/test").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_valid_api_key_passes() {
        let app = build_app(test_state(vec!["secret-key".into()]));

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/test")
                    .header("authorization", "Bearer secret-key")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_invalid_api_key_rejected() {
        let app = build_app(test_state(vec!["secret-key".into()]));

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/test")
                    .header("authorization", "Bearer wrong-key")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_missing_auth_header_rejected() {
        let app = build_app(test_state(vec!["secret-key".into()]));

        let response = app
            .oneshot(Request::builder().uri("/test").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_health_endpoint_bypasses_auth() {
        let app = build_app(test_state(vec!["secret-key".into()]));

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }
}
