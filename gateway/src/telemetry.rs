/// Centralized Telemetry & Cost Tracking for Heimdall Gateway.
///
/// Extracts token usage from LLM responses (OpenAI-compatible format)
/// and emits Prometheus metrics for monitoring and billing.

use serde::{Deserialize, Serialize};

// ─── Telemetry Data ─────────────────────────────────────────────────────────

/// Structured telemetry record for a single LLM request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestTelemetry {
    pub provider: String,
    pub model: String,
    pub endpoint_type: String, // "chat", "embedding", "rerank"
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub thinking_tokens: u32,
    pub latency_ms: u64,
    pub status: u16,
}

// ─── Token Extraction ───────────────────────────────────────────────────────

/// Extract token usage from an OpenAI-compatible response JSON.
///
/// Supports multiple formats:
/// - Standard OpenAI: `usage.completion_tokens_details.reasoning_tokens`
/// - Gemini/flat: `usage.reasoning_tokens`
/// - Embedding: `usage.prompt_tokens` + `usage.total_tokens`
pub fn extract_token_usage(json: &serde_json::Value) -> (u32, u32, u32) {
    let usage = match json.get("usage") {
        Some(u) => u,
        None => return (0, 0, 0),
    };

    let prompt_tokens = usage
        .get("prompt_tokens")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u32;

    let completion_tokens = usage
        .get("completion_tokens")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u32;

    // Reasoning/thinking tokens — try nested first, then flat
    let thinking_tokens = usage
        .get("completion_tokens_details")
        .and_then(|d| d.get("reasoning_tokens"))
        .and_then(|v| v.as_u64())
        .or_else(|| {
            usage
                .get("reasoning_tokens")
                .and_then(|v| v.as_u64())
        })
        .unwrap_or(0) as u32;

    (prompt_tokens, completion_tokens, thinking_tokens)
}

// ─── Metrics Recording ──────────────────────────────────────────────────────

/// Record telemetry from a completed LLM response.
///
/// Emits Prometheus metrics and structured tracing logs.
pub fn record_from_response(
    provider: &str,
    model: &str,
    endpoint_type: &str,
    response_json: &serde_json::Value,
    latency_ms: u64,
    status: u16,
) {
    let (prompt_tokens, completion_tokens, thinking_tokens) =
        extract_token_usage(response_json);

    let telemetry = RequestTelemetry {
        provider: provider.to_string(),
        model: model.to_string(),
        endpoint_type: endpoint_type.to_string(),
        prompt_tokens,
        completion_tokens,
        thinking_tokens,
        latency_ms,
        status,
    };

    // Emit Prometheus metrics
    let provider_label = provider.to_string();
    let model_label = model.to_string();
    let endpoint_label = endpoint_type.to_string();

    if prompt_tokens > 0 || completion_tokens > 0 {
        metrics::counter!(
            "heimdall_tokens_total",
            "provider" => provider_label.clone(),
            "model" => model_label.clone(),
            "type" => "prompt",
            "endpoint_type" => endpoint_label.clone()
        )
        .increment(prompt_tokens as u64);

        metrics::counter!(
            "heimdall_tokens_total",
            "provider" => provider_label.clone(),
            "model" => model_label.clone(),
            "type" => "completion",
            "endpoint_type" => endpoint_label.clone()
        )
        .increment(completion_tokens as u64);

        if thinking_tokens > 0 {
            metrics::counter!(
                "heimdall_tokens_total",
                "provider" => provider_label.clone(),
                "model" => model_label.clone(),
                "type" => "thinking",
                "endpoint_type" => endpoint_label.clone()
            )
            .increment(thinking_tokens as u64);
        }
    }

    metrics::histogram!(
        "heimdall_request_duration_seconds",
        "provider" => provider_label.clone(),
        "endpoint_type" => endpoint_label.clone()
    )
    .record(latency_ms as f64 / 1000.0);

    metrics::counter!(
        "heimdall_requests_total",
        "provider" => provider_label.clone(),
        "endpoint_type" => endpoint_label.clone(),
        "status" => status.to_string()
    )
    .increment(1);

    // Structured tracing for audit log
    tracing::info!(
        provider = %telemetry.provider,
        model = %telemetry.model,
        endpoint_type = %telemetry.endpoint_type,
        prompt_tokens = telemetry.prompt_tokens,
        completion_tokens = telemetry.completion_tokens,
        thinking_tokens = telemetry.thinking_tokens,
        latency_ms = telemetry.latency_ms,
        status = telemetry.status,
        "📊 Request telemetry recorded"
    );
}

/// Record telemetry for embedding requests (simpler — no completion tokens).
pub fn record_embedding(
    provider: &str,
    model: &str,
    token_count: usize,
    latency_ms: u64,
    status: u16,
) {
    let provider_label = provider.to_string();

    metrics::counter!(
        "heimdall_tokens_total",
        "provider" => provider_label.clone(),
        "model" => model.to_string(),
        "type" => "prompt",
        "endpoint_type" => "embedding".to_string()
    )
    .increment(token_count as u64);

    metrics::histogram!(
        "heimdall_request_duration_seconds",
        "provider" => provider_label.clone(),
        "endpoint_type" => "embedding".to_string()
    )
    .record(latency_ms as f64 / 1000.0);

    metrics::counter!(
        "heimdall_requests_total",
        "provider" => provider_label,
        "endpoint_type" => "embedding".to_string(),
        "status" => status.to_string()
    )
    .increment(1);

    tracing::info!(
        provider = %provider,
        model = %model,
        tokens = token_count,
        latency_ms = latency_ms,
        "📊 Embedding telemetry recorded"
    );
}

/// Record telemetry for rerank requests.
pub fn record_rerank(
    provider: &str,
    model: &str,
    doc_count: usize,
    latency_ms: u64,
    status: u16,
) {
    let provider_label = provider.to_string();

    metrics::histogram!(
        "heimdall_request_duration_seconds",
        "provider" => provider_label.clone(),
        "endpoint_type" => "rerank".to_string()
    )
    .record(latency_ms as f64 / 1000.0);

    metrics::counter!(
        "heimdall_requests_total",
        "provider" => provider_label,
        "endpoint_type" => "rerank".to_string(),
        "status" => status.to_string()
    )
    .increment(1);

    tracing::info!(
        provider = %provider,
        model = %model,
        documents = doc_count,
        latency_ms = latency_ms,
        "📊 Rerank telemetry recorded"
    );
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_token_usage_openai_format() {
        let resp = serde_json::json!({
            "choices": [{"message": {"content": "hello"}}],
            "usage": {
                "prompt_tokens": 10,
                "completion_tokens": 20,
                "completion_tokens_details": {
                    "reasoning_tokens": 5
                }
            }
        });
        let (p, c, t) = extract_token_usage(&resp);
        assert_eq!(p, 10);
        assert_eq!(c, 20);
        assert_eq!(t, 5);
    }

    #[test]
    fn test_extract_token_usage_gemini_flat() {
        let resp = serde_json::json!({
            "usage": {
                "prompt_tokens": 50,
                "completion_tokens": 100,
                "reasoning_tokens": 30
            }
        });
        let (p, c, t) = extract_token_usage(&resp);
        assert_eq!(p, 50);
        assert_eq!(c, 100);
        assert_eq!(t, 30);
    }

    #[test]
    fn test_extract_token_usage_no_usage() {
        let resp = serde_json::json!({"choices": [{"message": {"content": "hi"}}]});
        let (p, c, t) = extract_token_usage(&resp);
        assert_eq!(p, 0);
        assert_eq!(c, 0);
        assert_eq!(t, 0);
    }

    #[test]
    fn test_extract_token_usage_embedding_format() {
        let resp = serde_json::json!({
            "usage": {
                "prompt_tokens": 42,
                "total_tokens": 42
            }
        });
        let (p, c, t) = extract_token_usage(&resp);
        assert_eq!(p, 42);
        assert_eq!(c, 0);
        assert_eq!(t, 0);
    }
}
