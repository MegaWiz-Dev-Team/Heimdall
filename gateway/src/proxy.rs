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
#[tracing::instrument(skip(state, headers, body), fields(method = %method, uri = %uri, model = tracing::field::Empty, caller = tracing::field::Empty, request_id = tracing::field::Empty))]
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

    // Record caller attribution on the exported span so downstream consumers
    // (heimdall-trace / Laminar) can filter by originating service (e.g.
    // `caller=iris`) and join all spans of one pipeline run by `request_id`.
    if let Some(caller) = headers.get("x-asgard-caller").and_then(|v| v.to_str().ok()) {
        tracing::Span::current().record("caller", caller);
    }
    if let Some(req_id) = headers.get("x-request-id").and_then(|v| v.to_str().ok()) {
        tracing::Span::current().record("request_id", req_id);
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
        ResolvedRoute::LocalVlm { ref model, port } => {
            handle_local_vlm_proxy(state, method, uri, headers, body_bytes, path, model, port, start).await
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

    // Inject `chat_template_kwargs.enable_thinking: false` so thinking-mode
    // chat templates (Qwen3, Gemma3+ with a4b variants) emit the answer in
    // `message.content` instead of `message.reasoning`. Caller-supplied
    // `chat_template_kwargs.enable_thinking` always wins; non-JSON or
    // non-chat payloads pass through untouched.
    let body_bytes = if is_completion_path {
        inject_disable_thinking_default(body_bytes)
    } else {
        body_bytes
    };

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

/// Inject `chat_template_kwargs.enable_thinking: false` into a JSON
/// chat-completion body if (and only if) the caller hasn't already set it.
///
/// Returns the original bytes unchanged when the body isn't parseable JSON,
/// isn't a JSON object, or already carries an explicit `enable_thinking`.
fn inject_disable_thinking_default(body_bytes: Bytes) -> Bytes {
    let Ok(mut json) = serde_json::from_slice::<serde_json::Value>(&body_bytes) else {
        return body_bytes;
    };
    let Some(obj) = json.as_object_mut() else {
        return body_bytes;
    };
    // Only act on chat-completion shape (must carry `messages`).
    if !obj.contains_key("messages") {
        return body_bytes;
    }

    let kwargs = obj
        .entry("chat_template_kwargs".to_string())
        .or_insert_with(|| serde_json::json!({}));
    let Some(kwargs_obj) = kwargs.as_object_mut() else {
        // Caller passed a non-object — leave their value alone.
        return serde_json::to_vec(&json).map(Bytes::from).unwrap_or(body_bytes);
    };
    if !kwargs_obj.contains_key("enable_thinking") {
        kwargs_obj.insert(
            "enable_thinking".to_string(),
            serde_json::Value::Bool(false),
        );
    }

    serde_json::to_vec(&json).map(Bytes::from).unwrap_or(body_bytes)
}

/// Forward request to a local MLX-VLM server on a custom port (Sprint 51).
///
/// Skips hot-swap (each VLM server is pinned to one model — q4 or q8 — and
/// re-loading on each request would defeat the parallel-port pattern).
async fn handle_local_vlm_proxy(
    state: AppState,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    body_bytes: Bytes,
    path: &str,
    model: &str,
    port: u16,
    start: std::time::Instant,
) -> Result<Response, StatusCode> {
    let target_base_url = format!("http://{}:{}", state.config.backend_host, port);
    let backend_url = format!("{}{}", target_base_url, path);

    tracing::info!("{} {} → {} (local-vlm, model={})", method, uri, backend_url, model);

    forward_request(
        &state,
        method,
        uri,
        headers,
        body_bytes,
        &backend_url,
        None,
        "local-vlm",
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
    // Rewrite the model field + run 🌑 Skuggi PII redaction (Sprint 50b).
    //
    // Mode resolution order (first match wins):
    //   1. `tenant_configs.pii_mode` for the tenant id in `X-Tenant-Id`
    //      header (cached 60s) — production path
    //   2. `SKUGGI_MODE` env var — fallback for tests / when MariaDB
    //      is unreachable / no tenant header
    //   3. default `mask-and-send` — never silently disable redaction
    //
    // Mode dispatch:
    //   - off            → skip redaction entirely
    //   - detect-only    → run detection, log, pass ORIGINAL payload
    //   - mask-and-send  → run detection, log, send REDACTED payload  ◄── default
    //   - block-on-pii   → run detection, log; if any PII found return 422
    use crate::tenant_config::{insert_audit, AuditEvent, PiiMode};
    let tenant_id = headers
        .get("x-tenant-id")
        .and_then(|v| v.to_str().ok())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());
    let request_id = headers
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let pii_mode = if let (Some(cache), Some(tid)) = (state.tenant_cfg.as_ref(), tenant_id.as_ref()) {
        cache.get_pii_mode(tid).await
    } else {
        // Fallback: env var (back-compat with v0 partial scaffolding)
        match std::env::var("SKUGGI_MODE").as_deref() {
            Ok("off") => PiiMode::Off,
            Ok(s) => PiiMode::from_db(s),
            Err(_) => PiiMode::MaskAndSend, // safe default
        }
    };

    let skuggi_started = std::time::Instant::now();

    // First parse + run redaction; we may need the detection list before
    // deciding whether to send the redacted body, the original body, or
    // reject with 422.
    let parsed_json = serde_json::from_slice::<serde_json::Value>(&body_bytes).ok();
    let (rewritten_body, skuggi_detections, skuggi_pii_total) = if let Some(mut json) = parsed_json {
        if let Some(obj) = json.as_object_mut() {
            obj.insert("model".to_string(), serde_json::Value::String(model.to_string()));
        }

        match pii_mode {
            PiiMode::Off => {
                let body_bytes_out = serde_json::to_vec(&json).unwrap_or_else(|_| body_bytes.to_vec());
                (body_bytes_out, Vec::new(), 0usize)
            }
            PiiMode::DetectOnly => {
                // Detect on a clone; log; send the model-rewritten ORIGINAL.
                let mut probe = json.clone();
                let dets = crate::skuggi::redact_chat_body(&mut probe);
                let pii_total: usize = dets.iter().map(|d| d.count).sum();
                if pii_total > 0 {
                    let summary: Vec<String> = dets.iter()
                        .map(|d| format!("{}={}", d.category, d.count))
                        .collect();
                    tracing::info!(
                        "🌑 skuggi[detect-only] tenant={:?} provider={} model={} detections={} (NOT REDACTED)",
                        tenant_id, provider, model, summary.join(",")
                    );
                }
                let body_bytes_out = serde_json::to_vec(&json).unwrap_or_else(|_| body_bytes.to_vec());
                (body_bytes_out, dets, pii_total)
            }
            PiiMode::MaskAndSend => {
                let dets = crate::skuggi::redact_chat_body(&mut json);
                let pii_total: usize = dets.iter().map(|d| d.count).sum();
                if pii_total > 0 {
                    let summary: Vec<String> = dets.iter()
                        .map(|d| format!("{}={}", d.category, d.count))
                        .collect();
                    tracing::info!(
                        "🌑 skuggi[mask-and-send] tenant={:?} provider={} model={} detections={}",
                        tenant_id, provider, model, summary.join(",")
                    );
                }
                let body_bytes_out = serde_json::to_vec(&json).unwrap_or_else(|_| body_bytes.to_vec());
                (body_bytes_out, dets, pii_total)
            }
            PiiMode::BlockOnPii => {
                // Detect on a clone so the original survives if we decide to forward
                let mut probe = json.clone();
                let dets = crate::skuggi::redact_chat_body(&mut probe);
                let pii_total: usize = dets.iter().map(|d| d.count).sum();
                if pii_total > 0 {
                    let summary: Vec<String> = dets.iter()
                        .map(|d| format!("{}={}", d.category, d.count))
                        .collect();
                    tracing::warn!(
                        "🌑 skuggi[block-on-pii] BLOCKED tenant={:?} provider={} model={} detections={}",
                        tenant_id, provider, model, summary.join(",")
                    );
                    metrics::counter!("skuggi_blocked_total").increment(1);

                    // Audit even when blocked — compliance needs proof
                    // we caught it. Fire-and-forget so the 422 reply
                    // isn't held up by the DB.
                    if let (Some(cache), Some(tid)) = (state.tenant_cfg.as_ref(), tenant_id.as_ref()) {
                        let pool = cache.pool().clone();
                        let evt = AuditEvent {
                            tenant_id: tid.clone(),
                            request_id: request_id.clone(),
                            provider: provider.to_string(),
                            model: model.to_string(),
                            mode: pii_mode,
                            detections: dets,
                            pii_total_count: pii_total,
                            blocked: true,
                            payload_bytes: body_bytes.len(),
                            redacted_bytes: 0, // payload never sent
                            duration: skuggi_started.elapsed(),
                            detection_tier: "tier1",
                        };
                        tokio::spawn(async move { insert_audit(&pool, evt).await });
                    }
                    return Err(StatusCode::UNPROCESSABLE_ENTITY);
                }
                let body_bytes_out = serde_json::to_vec(&json).unwrap_or_else(|_| body_bytes.to_vec());
                (body_bytes_out, dets, pii_total)
            }
        }
    } else {
        // Body wasn't parseable JSON — pass through unchanged. Skuggi
        // doesn't operate on opaque payloads (image-uploads, multipart,
        // etc.); those are handled by the Syn OCR layer (`surface=image`).
        (body_bytes.to_vec(), Vec::new(), 0usize)
    };

    // Sprint 50b W2: Tier 2 NER (PyThaiNLP) — only when Tier 1 found
    // little/nothing AND payload is long Thai text. Cheap heuristic
    // gate keeps the 50-100ms NER call off the hot path for ~80% of
    // requests. Sidecar URL via PYTHAINLP_URL env; absent or
    // unreachable → fall back to Tier 1 result.
    let mut tier_used = "tier1";
    let mut combined_detections = skuggi_detections;
    let mut combined_pii_total = skuggi_pii_total;
    if pii_mode != PiiMode::Off {
        let raw_text = std::str::from_utf8(&body_bytes).unwrap_or("");
        if crate::skuggi::should_fire_tier2(raw_text, combined_pii_total) {
            match crate::skuggi::tier2_detect(&state.http_client, raw_text).await {
                Ok(Some(t2)) => {
                    let t2_total: usize = t2.detections.iter().map(|d| d.count).sum();
                    if t2_total > 0 {
                        for det in t2.detections {
                            // Map sidecar's stringly category to a stable &'static
                            // — anything outside the known set falls back to
                            // "thai_other" so audit rows never carry untrusted
                            // user-controlled strings into static memory.
                            let cat: &'static str = match det.category.as_str() {
                                "thai_person_name" => "thai_person_name",
                                "thai_address"     => "thai_address",
                                "thai_org"         => "thai_org",
                                _                  => "thai_other",
                            };
                            combined_detections.push(crate::skuggi::Detection {
                                category: cat,
                                count: det.count,
                            });
                        }
                        combined_pii_total += t2_total;
                        tier_used = "tier1+tier2";
                        tracing::info!(
                            "🌑 skuggi tier2 found {} additional PII items tenant={:?}",
                            t2_total, tenant_id
                        );
                    }
                }
                Ok(None) => {} // sidecar not configured
                Err(e) => tracing::warn!("🌑 skuggi tier2 call failed: {}", e),
            }
        }
    }

    let skuggi_duration = skuggi_started.elapsed();

    // Day-3 audit: every cloud-bound chat call logs one row for the
    // happy path (block-on-pii has its own pre-return insert above).
    // Skip audit when we have no tenant/cache context — env-var fallback
    // mode is for dev/test only and has no compliance requirement.
    if let (Some(cache), Some(tid)) = (state.tenant_cfg.as_ref(), tenant_id.as_ref()) {
        let pool = cache.pool().clone();
        let evt = AuditEvent {
            tenant_id: tid.clone(),
            request_id: request_id.clone(),
            provider: provider.to_string(),
            model: model.to_string(),
            mode: pii_mode,
            detections: combined_detections,
            pii_total_count: combined_pii_total,
            blocked: false,
            payload_bytes: body_bytes.len(),
            redacted_bytes: rewritten_body.len(),
            duration: skuggi_duration,
            detection_tier: tier_used,
        };
        tokio::spawn(async move { insert_audit(&pool, evt).await });
    }

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
