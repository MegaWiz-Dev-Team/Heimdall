/// Reverse proxy to the LLM backend engine.
/// Forwards all /v1/* requests to the vllm-mlx backend,
/// supporting both regular JSON responses and SSE streaming.
///
/// Enhanced with Smart Router: inspects the `model` field to decide
/// whether to forward locally or to an external provider (OpenRouter, Gemini, OpenAI).

use axum::{
    body::Body,
    extract::State,
    http::{HeaderMap, Method, StatusCode, Uri},
    response::{Html, Response},
    routing::{any, get},
    Router,
};
use bytes::Bytes;
use futures::StreamExt;

use crate::router::{self, ResolvedRoute};
use crate::telemetry;
use crate::AppState;

/// Proxy handler — routes request based on model prefix, then forwards and streams response.
#[tracing::instrument(skip(state, headers, body), fields(method = %method, uri = %uri, model = tracing::field::Empty))]
async fn proxy_handler(
    State(state): State<AppState>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    body: Body,
) -> Result<Response, StatusCode> {
    let start = std::time::Instant::now();
    let path_owned = uri.path_and_query().map(|pq| pq.as_str().to_string()).unwrap_or_else(|| "/".to_string());
    let path = path_owned.as_str();

    // Collect request body
    let body_bytes = match axum::body::to_bytes(body, 10 * 1024 * 1024).await {
        Ok(b) => b,
        Err(e) => {
            tracing::error!("Failed to read request body: {}", e);
            return Err(StatusCode::BAD_REQUEST);
        }
    };

    // ── Route Resolution ────────────────────────────────────────────────
    // Only attempt routing for chat/completions endpoints
    let is_completion_path = path == "/chat/completions"
        || path == "/completions"
        || path == "/v1/chat/completions"
        || path == "/v1/completions";

    // Extract requested model
    let requested_model = if is_completion_path {
        serde_json::from_slice::<serde_json::Value>(&body_bytes)
            .ok()
            .and_then(|json| json.get("model").and_then(|m| m.as_str()).map(String::from))
    } else {
        None
    };

    if let Some(ref model) = requested_model {
        tracing::Span::current().record("model", model.as_str());
    }

    // Extract tenant-specific provider key from header
    let provider_key = router::extract_provider_key(&headers);

    // Resolve route
    let route = if let Some(ref model) = requested_model {
        match router::resolve_route(model, &state.config, provider_key.as_deref()) {
            Ok(r) => r,
            Err((status, msg)) => {
                tracing::warn!("Route resolution failed: {}", msg);
                return Ok(Response::builder()
                    .status(status)
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({"error": {"message": msg}}).to_string(),
                    ))
                    .unwrap());
            }
        }
    } else {
        // Non-completion paths (e.g. /v1/models) → always local
        ResolvedRoute::Local {
            model: String::new(),
        }
    };

    match route {
        ResolvedRoute::Local { ref model } => {
            handle_local_proxy(state, method, uri, headers, body_bytes, path, model, start).await
        }
        ResolvedRoute::External {
            ref provider,
            ref model,
            ref base_url,
            ref api_key,
        } => {
            handle_external_proxy(
                state,
                method,
                headers,
                body_bytes,
                path,
                provider,
                model,
                base_url,
                api_key,
                &requested_model.unwrap_or_default(),
                start,
            )
            .await
        }
    }
}

/// Forward request to the local MLX/llama.cpp backend (existing behavior).
async fn handle_local_proxy(
    state: AppState,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    body_bytes: Bytes,
    path: &str,
    model: &str,
    start: std::time::Instant,
) -> Result<Response, StatusCode> {
    let target_base_url = state.config.backend_url();
    let backend_url = format!("{}{}", target_base_url, path);

    tracing::info!("{} {} → {} (local)", method, uri, backend_url);

    // Hot-swap logic: check if the request asks for a different MLX model
    let is_completion_path = path == "/chat/completions"
        || path == "/completions"
        || path == "/v1/chat/completions"
        || path == "/v1/completions";

    if is_completion_path && !model.is_empty() {
        let current_model = {
            let lock = state.active_model.read().unwrap();
            (*lock).clone()
        };

        if model != current_model && !model.is_empty() {
            tracing::info!(
                "🔄 Model swap requested: '{}' != '{}'",
                model,
                current_model
            );
            let _guard = state.swap_lock.lock().await;

            // Double-check inside lock
            let current_model = {
                let lock = state.active_model.read().unwrap();
                (*lock).clone()
            };

            if model != current_model {
                tracing::warn!("Executing hot-swap to {}...", model);
                let output = tokio::process::Command::new("./scripts/hotswap.sh")
                    .arg(model)
                    .current_dir(&state.config.project_dir)
                    .output()
                    .await;

                match output {
                    Ok(out) if out.status.success() => {
                        tracing::info!("✅ Hot-swap successful!");
                        let mut lock = state.active_model.write().unwrap();
                        *lock = model.to_string();
                    }
                    Ok(out) => {
                        let stderr = String::from_utf8_lossy(&out.stderr);
                        let stdout = String::from_utf8_lossy(&out.stdout);
                        tracing::error!(
                            "❌ Hot-swap script failed: {}\nStdout: {}",
                            stderr,
                            stdout
                        );
                        return Err(StatusCode::INTERNAL_SERVER_ERROR);
                    }
                    Err(e) => {
                        tracing::error!("❌ Failed to execute hot-swap script: {}", e);
                        return Err(StatusCode::INTERNAL_SERVER_ERROR);
                    }
                }
            }
        }
    }

    // Forward to local backend
    forward_request(
        &state,
        method,
        uri,
        headers,
        body_bytes,
        &backend_url,
        None, // No auth override for local
        "local",
        model,
        start,
    )
    .await
}

/// Forward request to an external provider (OpenRouter, Gemini, OpenAI).
async fn handle_external_proxy(
    state: AppState,
    method: Method,
    headers: HeaderMap,
    body_bytes: Bytes,
    path: &str,
    provider: &str,
    model: &str,
    base_url: &str,
    api_key: &str,
    _original_model: &str,
    start: std::time::Instant,
) -> Result<Response, StatusCode> {
    // Rewrite the model field in the JSON body — strip the provider prefix
    let rewritten_body = if let Ok(mut json) =
        serde_json::from_slice::<serde_json::Value>(&body_bytes)
    {
        if let Some(obj) = json.as_object_mut() {
            obj.insert("model".to_string(), serde_json::Value::String(model.to_string()));
        }
        serde_json::to_vec(&json).unwrap_or_else(|_| body_bytes.to_vec())
    } else {
        body_bytes.to_vec()
    };

    // Determine the correct path for the external provider
    let external_path = if path.contains("chat/completions") {
        "/chat/completions"
    } else if path.contains("completions") {
        "/completions"
    } else {
        path
    };

    let external_url = format!(
        "{}{}",
        base_url.trim_end_matches('/'),
        external_path
    );

    tracing::info!(
        "🌐 {} → {} (provider={}, model={})",
        method,
        external_url,
        provider,
        model
    );

    let uri_for_log: Uri = external_url.parse().unwrap_or_default();

    forward_request(
        &state,
        method,
        uri_for_log,
        headers,
        Bytes::from(rewritten_body),
        &external_url,
        Some(api_key),
        provider,
        model,
        start,
    )
    .await
}

/// Shared request forwarding logic — handles both local and external targets.
async fn forward_request(
    state: &AppState,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    body_bytes: Bytes,
    target_url: &str,
    auth_override: Option<&str>,
    provider: &str,
    model: &str,
    start: std::time::Instant,
) -> Result<Response, StatusCode> {
    // Build the proxied request
    let mut req_builder = state
        .http_client
        .request(method.clone(), target_url)
        .body(body_bytes.clone());

    // Forward relevant headers
    for (key, value) in headers.iter() {
        let key_str = key.as_str();
        // Skip hop-by-hop headers and auth (we'll set our own)
        if key_str != "host"
            && key_str != "authorization"
            && key_str != "x-provider-key"
            && key_str != "content-length"
        {
            req_builder = req_builder.header(key.clone(), value.clone());
        }
    }

    // Set authorization
    if let Some(api_key) = auth_override {
        req_builder = req_builder.header("Authorization", format!("Bearer {}", api_key));
    }

    // Ensure content-type is set
    req_builder = req_builder.header("Content-Type", "application/json");

    // Send request to backend
    let backend_resp = match req_builder.send().await {
        Ok(resp) => resp,
        Err(e) => {
            tracing::error!("Backend request failed (provider={}): {}", provider, e);
            metrics::counter!("proxy_errors_total", "provider" => provider.to_string())
                .increment(1);
            return Err(StatusCode::BAD_GATEWAY);
        }
    };

    let status = backend_resp.status();
    let resp_headers = backend_resp.headers().clone();
    let content_type = resp_headers
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    let is_streaming = content_type.contains("text/event-stream");

    // Record base metrics
    let elapsed = start.elapsed();
    metrics::counter!("proxy_requests_total", "provider" => provider.to_string()).increment(1);
    metrics::histogram!("proxy_request_duration_seconds", "provider" => provider.to_string())
        .record(elapsed.as_secs_f64());

    if is_streaming {
        // Stream SSE response
        tracing::info!(
            "Streaming SSE response (provider={}, {}ms to first byte)",
            provider,
            elapsed.as_millis()
        );

        let stream = backend_resp.bytes_stream().map(|chunk| {
            chunk
                .map(|b| axum::body::Bytes::from(b.to_vec()))
                .map_err(|e| {
                    tracing::error!("Stream error: {}", e);
                    std::io::Error::new(std::io::ErrorKind::Other, e)
                })
        });

        let body = Body::from_stream(stream);

        let mut response = Response::builder().status(status.as_u16());

        for (key, value) in resp_headers.iter() {
            let key_str = key.as_str();
            if key_str != "transfer-encoding" {
                response = response.header(key.clone(), value.clone());
            }
        }

        Ok(response.body(body).unwrap())
    } else {
        // Regular JSON response
        let response_body = backend_resp.bytes().await.map_err(|e| {
            tracing::error!("Failed to read backend response: {}", e);
            StatusCode::BAD_GATEWAY
        })?;

        tracing::info!(
            "{} {} → {} (provider={}, {}ms, {} bytes)",
            method,
            uri,
            status,
            provider,
            elapsed.as_millis(),
            response_body.len()
        );

        // Record telemetry from response body (token usage, etc.)
        if let Ok(json) = serde_json::from_slice::<serde_json::Value>(&response_body) {
            telemetry::record_from_response(
                provider,
                model,
                "chat",
                &json,
                elapsed.as_millis() as u64,
                status.as_u16(),
            );
        }

        let mut response = Response::builder().status(status.as_u16());

        for (key, value) in resp_headers.iter() {
            let key_str = key.as_str();
            if key_str != "transfer-encoding" 
                && key_str != "content-encoding" 
                && key_str != "content-length" 
            {
                response = response.header(key.clone(), value.clone());
            }
        }

        Ok(response
            .body(Body::from(Bytes::from(response_body.to_vec())))
            .unwrap())
    }
}

pub fn routes() -> Router<AppState> {
    Router::new()
        // OpenAI-compatible endpoints
        .route("/v1/{*path}", any(proxy_handler))
}
