/// Reverse proxy to the LLM backend engine.
/// Forwards all /v1/* requests to the vllm-mlx backend,
/// supporting both regular JSON responses and SSE streaming.

use axum::{
    body::Body,
    extract::State,
    http::{HeaderMap, Method, StatusCode, Uri},
    response::Response,
    routing::{any, get},
    Router,
};
use bytes::Bytes;
use futures::StreamExt;

use crate::AppState;

/// Proxy handler — forwards request to backend and streams response back.
async fn proxy_handler(
    State(state): State<AppState>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    body: Body,
) -> Result<Response, StatusCode> {
    let start = std::time::Instant::now();
    let path = uri.path_and_query().map(|pq| pq.as_str()).unwrap_or("/");
    
    // Choose target backend based on path
    let target_base_url = if path.starts_with("/v1/embeddings") || path.starts_with("/v1/rerank") || path.starts_with("/rerank") {
        state.config.embedding_url()
    } else {
        state.config.backend_url()
    };
    
    // Forward the path as-is to the chosen backend.
    let backend_url = format!("{}{}", target_base_url, path);

    tracing::info!("{} {} → {}", method, uri, backend_url);

    // Collect request body
    let body_bytes = match axum::body::to_bytes(body, 10 * 1024 * 1024).await {
        Ok(b) => b,
        Err(e) => {
            tracing::error!("Failed to read request body: {}", e);
            return Err(StatusCode::BAD_REQUEST);
        }
    };

    // Build the proxied request
    let mut req_builder = state
        .http_client
        .request(method.clone(), &backend_url)
        .body(body_bytes.clone());

    // Forward relevant headers
    for (key, value) in headers.iter() {
        let key_str = key.as_str();
        if key_str != "host" && key_str != "authorization" {
            req_builder = req_builder.header(key.clone(), value.clone());
        }
    }

    // Send request to backend
    let backend_resp = match req_builder.send().await {
        Ok(resp) => resp,
        Err(e) => {
            tracing::error!("Backend request failed: {}", e);
            metrics::counter!("proxy_errors_total").increment(1);
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

    // Check if this is a streaming response (SSE)
    let is_streaming = content_type.contains("text/event-stream");

    // Record metrics
    let elapsed = start.elapsed();
    metrics::counter!("proxy_requests_total").increment(1);
    metrics::histogram!("proxy_request_duration_seconds").record(elapsed.as_secs_f64());

    if is_streaming {
        // Stream SSE response
        tracing::info!("Streaming SSE response ({}ms to first byte)", elapsed.as_millis());

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

        // Forward response headers
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
            "{} {} → {} ({}ms, {} bytes)",
            method,
            uri,
            status,
            elapsed.as_millis(),
            response_body.len()
        );

        let mut response = Response::builder().status(status.as_u16());

        for (key, value) in resp_headers.iter() {
            response = response.header(key.clone(), value.clone());
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
        // Root fallback
        .route(
            "/",
            get(|| async {
                serde_json::json!({
                    "name": "Heimdall",
                    "description": "Guardian of the LLM realm",
                    "version": env!("CARGO_PKG_VERSION"),
                    "endpoints": {
                        "api": "/v1/",
                        "health": "/health",
                        "metrics": "/metrics"
                    }
                })
                .to_string()
            }),
        )
}
