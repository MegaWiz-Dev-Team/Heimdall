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
    let models_url = format!("{}/models", backend_url);

    // Try to reach the backend
    let (backend_status, models) = match state.http_client.get(&models_url).send().await {
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
