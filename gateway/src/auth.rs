/// Authentication middleware. Dual-mode:
///   1. Static API_KEYS (legacy bearer compare, sub-µs)
///   2. Yggdrasil JWT (RS256 via JWKS, ~0.1–0.3ms)
///
/// Heuristic: bearer tokens starting with `ey` (the base64 of a JWT header)
/// are routed to JWT validation; everything else falls back to static-key
/// compare. This lets both modes coexist without per-request config.

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
/// If neither static keys nor JWT mode are configured, all requests pass through.
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

    let jwt_enabled = state.jwt_validator.is_some();

    // If neither auth mode is configured, skip auth entirely
    if !state.config.auth_enabled && !jwt_enabled {
        return Ok(next.run(request).await);
    }

    // Extract Bearer token.
    //
    // Parse per RFC 6750/7235: scheme name is CASE-INSENSITIVE
    // ("Bearer" / "bearer" / "BEARER" all valid). We also `.trim()` so
    // "Bearer    abc.def.ghi   " parses the same as "Bearer abc.def.ghi".
    // This mirrors the Mimir-side parser at
    // mimir-core-ai/src/middleware/dual_mode_auth.rs so the two services
    // accept the same set of tokens — important for cross-service
    // contract tests and for clients that hit both gateway and backend.
    let auth_header = request
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok());

    let token = match auth_header.and_then(parse_bearer_token) {
        Some(t) if !t.is_empty() => t,
        _ => {
            metrics::counter!("auth_failure_total", "mode" => "missing").increment(1);
            return Err(StatusCode::UNAUTHORIZED);
        }
    };

    // Route by token shape: JWTs always start with "ey" (b64 of `{"alg"`...).
    // Anything else falls through to static-key compare.
    if jwt_enabled && token.starts_with("ey") {
        match state
            .jwt_validator
            .as_ref()
            .unwrap()
            .validate(token)
            .await
        {
            Ok(claims) => {
                metrics::counter!("auth_success_total", "mode" => "jwt").increment(1);
                tracing::info!(
                    auth_mode = "jwt",
                    sub = %claims.sub,
                    tenant = claims.tenant_id.as_deref().unwrap_or("-"),
                    scope = claims.scope.as_deref().unwrap_or("-"),
                    "auth.success"
                );
                // Surface claims to downstream handlers via request extensions.
                // axum extractor: `Extension<crate::auth_jwt::Claims>`.
                // Cross-service contract (Bifrost/Mimir/Eir/Hermodr): use the
                // same Claims type so a single TenantContext extractor works.
                let mut request = request;
                request.extensions_mut().insert(claims);
                return Ok(next.run(request).await);
            }
            Err(e) => {
                metrics::counter!("auth_failure_total", "mode" => "jwt").increment(1);
                tracing::warn!(auth_mode = "jwt", error = %e, "auth.failure");
                return Err(StatusCode::UNAUTHORIZED);
            }
        }
    }

    // Static-key fallback
    if state.config.auth_enabled && state.config.api_keys.contains(&token.to_string()) {
        metrics::counter!("auth_success_total", "mode" => "static").increment(1);
        Ok(next.run(request).await)
    } else {
        metrics::counter!("auth_failure_total", "mode" => "static").increment(1);
        tracing::warn!(auth_mode = "static", "auth.failure");
        Err(StatusCode::UNAUTHORIZED)
    }
}

/// Parse `Authorization: <scheme> <token>` per RFC 6750/7235.
///
/// Returns `Some(token)` iff the scheme is "Bearer" (case-insensitive) and
/// at least one space follows it. Returns `None` for any other shape so the
/// caller emits a single 401 path. Token is trimmed of surrounding
/// whitespace.
fn parse_bearer_token(header: &str) -> Option<&str> {
    let (scheme, rest) = header.split_once(' ')?;
    if scheme.eq_ignore_ascii_case("Bearer") {
        Some(rest.trim())
    } else {
        None
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
                vlm_q4_port: 8082,
                vlm_q8_port: 8083,
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
                yggdrasil_issuer: None,
                jwt_audience: None,
            }),
            http_client: reqwest::Client::new(),
            active_model: Arc::new(std::sync::RwLock::new(String::new())),
            swap_lock: Arc::new(tokio::sync::Mutex::new(())),
            admission: Arc::new(crate::admission::Admission::from_env()),
            tenant_cfg: None, // tests don't need tenant lookup
            jwt_validator: None, // static-key tests only
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
