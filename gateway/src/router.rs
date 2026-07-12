/// Smart Provider Router for Heimdall Gateway.
///
/// Inspects the incoming `model` field and resolves the correct backend:
/// - No prefix → Local MLX/llama.cpp backend
/// - `openrouter/...` → OpenRouter API
/// - `gemini/...` → Google Gemini (OpenAI-compatible endpoint)
/// - `openai/...` → OpenAI API
///
/// Supports `X-Provider-Key` header for per-tenant API key override.

use axum::http::StatusCode;

use crate::config::AppConfig;

// ─── Resolved Route ─────────────────────────────────────────────────────────

/// The result of route resolution: either forward locally or to an external provider.
#[derive(Debug, Clone)]
pub enum ResolvedRoute {
    /// Forward to local MLX/llama.cpp backend (text LLM, default port 8081).
    Local { model: String },
    /// Forward to a local MLX-VLM server on a different port.
    /// Used for vision-language OCR models (Typhoon-OCR-MLX) — see Sprint 51
    /// where we ship `mlx_vlm.server` on 8082/8083 alongside the text-LLM
    /// backend. The proxy logic mirrors `Local` but talks to a different
    /// port so the text + vision servers can run in parallel.
    LocalVlm { model: String, port: u16 },
    /// Forward to an external OpenAI-compatible API.
    External {
        provider: String,
        model: String,
        base_url: String,
        api_key: String,
    },
}

// ─── Route Resolution ───────────────────────────────────────────────────────

/// Resolve an incoming model name to a concrete route.
///
/// # Key precedence for external providers
/// 1. `provider_key_header` (`X-Provider-Key`) — per-tenant override from Mimir.
/// 2. Heimdall's own config key (from `.env`).
/// 3. If neither is set → return 401 Unauthorized.
///
/// # Model prefix conventions
/// - `openrouter/anthropic/claude-3.5-sonnet` → OpenRouter with model `anthropic/claude-3.5-sonnet`
/// - `gemini/gemini-2.5-flash` → Google Gemini with model `gemini-2.5-flash`
/// - `openai/gpt-4o` → OpenAI with model `gpt-4o`
/// - `mlx-community/Qwen3.5-35B-A3B-4bit` → Local backend (no prefix)
/// - `BAAI/bge-m3` → Local embedding (no prefix)
pub fn resolve_route(
    model: &str,
    config: &AppConfig,
    provider_key_header: Option<&str>,
) -> Result<ResolvedRoute, (StatusCode, String)> {
    // ── OpenRouter ──────────────────────────────────────────────────────
    if let Some(rest) = model.strip_prefix("openrouter/") {
        let api_key = resolve_api_key(
            provider_key_header,
            config.openrouter_api_key.as_deref(),
            "openrouter",
        )?;
        return Ok(ResolvedRoute::External {
            provider: "openrouter".to_string(),
            model: rest.to_string(),
            base_url: config.openrouter_base_url.clone(),
            api_key,
        });
    }

    // ── Google Gemini ───────────────────────────────────────────────────
    if let Some(rest) = model.strip_prefix("gemini/") {
        let api_key = resolve_api_key(
            provider_key_header,
            config.gemini_api_key.as_deref(),
            "gemini",
        )?;
        return Ok(ResolvedRoute::External {
            provider: "gemini".to_string(),
            model: rest.to_string(),
            base_url: config.gemini_base_url.clone(),
            api_key,
        });
    }

    // ── OpenAI ──────────────────────────────────────────────────────────
    if let Some(rest) = model.strip_prefix("openai/") {
        let api_key = resolve_api_key(
            provider_key_header,
            config.openai_api_key.as_deref(),
            "openai",
        )?;
        return Ok(ResolvedRoute::External {
            provider: "openai".to_string(),
            model: rest.to_string(),
            base_url: config.openai_base_url.clone(),
            api_key,
        });
    }

    // ── Local VLM (Sprint 51 — Typhoon-OCR MLX served via mlx_vlm.server) ─
    // Two ports live alongside the text-LLM backend on 8081:
    //   - q4 (default, 3.5GB peak, fast): VLM_Q4_PORT (8082)
    //   - q8 (escalation, 5GB peak, lower CER ceiling): VLM_Q8_PORT (8083)
    // Match is suffix-based so future quants (e.g. q6) drop in by adding
    // a port and the matching arm.
    if model.ends_with("-mlx-q4") && model.starts_with("MegawizCo/") {
        return Ok(ResolvedRoute::LocalVlm {
            model: model.to_string(),
            port: config.vlm_q4_port,
        });
    }
    if model.ends_with("-mlx-q8") && model.starts_with("MegawizCo/") {
        return Ok(ResolvedRoute::LocalVlm {
            model: model.to_string(),
            port: config.vlm_q8_port,
        });
    }

    // ── Local (default) ─────────────────────────────────────────────────
    Ok(ResolvedRoute::Local {
        model: model.to_string(),
    })
}

/// Resolve the API key for an external provider.
///
/// Priority: X-Provider-Key header → config env key → error.
fn resolve_api_key(
    header_key: Option<&str>,
    config_key: Option<&str>,
    provider_name: &str,
) -> Result<String, (StatusCode, String)> {
    // 1. Tenant-specific key from header takes precedence
    if let Some(key) = header_key {
        if !key.is_empty() {
            return Ok(key.to_string());
        }
    }

    // 2. Fallback to Heimdall's centralized key from .env
    if let Some(key) = config_key {
        if !key.is_empty() {
            return Ok(key.to_string());
        }
    }

    // 3. No key available
    Err((
        StatusCode::UNAUTHORIZED,
        format!(
            "No API key available for provider '{}'. Set {}_API_KEY in Heimdall's .env or pass X-Provider-Key header.",
            provider_name,
            provider_name.to_uppercase()
        ),
    ))
}

/// Extract the `X-Provider-Key` header value from request headers.
pub fn extract_provider_key(headers: &axum::http::HeaderMap) -> Option<String> {
    headers
        .get("x-provider-key")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> AppConfig {
        AppConfig {
            host: "0.0.0.0".into(),
            gateway_port: 8080,
            backend_host: "127.0.0.1".into(),
            backend_port: 8081,
            embedding_host: "127.0.0.1".into(),
            embedding_port: 8001,
            vlm_q4_port: 8082,
            vlm_q8_port: 8083,
            local_slots: vec![],
            api_keys: vec![],
            auth_enabled: false,
            llm_model: "mlx-community/Qwen3.5-35B-A3B-4bit".into(),
            project_dir: "../".into(),
            // External providers
            openrouter_api_key: Some("sk-or-test-key".into()),
            openrouter_base_url: "https://openrouter.ai/api/v1".into(),
            gemini_api_key: Some("AIzaSy-test-key".into()),
            gemini_base_url: "https://generativelanguage.googleapis.com/v1beta/openai".into(),
            openai_api_key: Some("sk-proj-test-key".into()),
            openai_base_url: "https://api.openai.com/v1".into(),
            yggdrasil_issuer: None,
            jwt_audience: None,
        }
    }

    fn test_config_no_keys() -> AppConfig {
        AppConfig {
            openrouter_api_key: None,
            gemini_api_key: None,
            openai_api_key: None,
            ..test_config()
        }
    }

    // ── Local routing ───────────────────────────────────────────────────

    #[test]
    fn test_local_model_routes_locally() {
        let config = test_config();
        let result = resolve_route("mlx-community/Qwen3.5-35B-A3B-4bit", &config, None).unwrap();
        match result {
            ResolvedRoute::Local { model } => {
                assert_eq!(model, "mlx-community/Qwen3.5-35B-A3B-4bit");
            }
            _ => panic!("Expected Local route"),
        }
    }

    #[test]
    fn test_embedding_model_routes_locally() {
        let config = test_config();
        let result = resolve_route("BAAI/bge-m3", &config, None).unwrap();
        match result {
            ResolvedRoute::Local { model } => assert_eq!(model, "BAAI/bge-m3"),
            _ => panic!("Expected Local route for embedding model"),
        }
    }

    #[test]
    fn test_reranker_model_routes_locally() {
        let config = test_config();
        let result = resolve_route("BAAI/bge-reranker-v2-m3", &config, None).unwrap();
        match result {
            ResolvedRoute::Local { model } => assert_eq!(model, "BAAI/bge-reranker-v2-m3"),
            _ => panic!("Expected Local route for reranker model"),
        }
    }

    // ── OpenRouter routing ──────────────────────────────────────────────

    #[test]
    fn test_openrouter_prefix_routes_externally() {
        let config = test_config();
        let result =
            resolve_route("openrouter/anthropic/claude-3.5-sonnet", &config, None).unwrap();
        match result {
            ResolvedRoute::External {
                provider,
                model,
                base_url,
                api_key,
            } => {
                assert_eq!(provider, "openrouter");
                assert_eq!(model, "anthropic/claude-3.5-sonnet");
                assert_eq!(base_url, "https://openrouter.ai/api/v1");
                assert_eq!(api_key, "sk-or-test-key");
            }
            _ => panic!("Expected External route for OpenRouter"),
        }
    }

    // ── Gemini routing ──────────────────────────────────────────────────

    #[test]
    fn test_gemini_prefix_routes_externally() {
        let config = test_config();
        let result = resolve_route("gemini/gemini-2.5-flash", &config, None).unwrap();
        match result {
            ResolvedRoute::External {
                provider,
                model,
                api_key,
                ..
            } => {
                assert_eq!(provider, "gemini");
                assert_eq!(model, "gemini-2.5-flash");
                assert_eq!(api_key, "AIzaSy-test-key");
            }
            _ => panic!("Expected External route for Gemini"),
        }
    }

    // ── OpenAI routing ──────────────────────────────────────────────────

    #[test]
    fn test_openai_prefix_routes_externally() {
        let config = test_config();
        let result = resolve_route("openai/gpt-4o", &config, None).unwrap();
        match result {
            ResolvedRoute::External {
                provider,
                model,
                base_url,
                api_key,
            } => {
                assert_eq!(provider, "openai");
                assert_eq!(model, "gpt-4o");
                assert_eq!(base_url, "https://api.openai.com/v1");
                assert_eq!(api_key, "sk-proj-test-key");
            }
            _ => panic!("Expected External route for OpenAI"),
        }
    }

    // ── X-Provider-Key header precedence ────────────────────────────────

    #[test]
    fn test_provider_key_header_takes_precedence() {
        let config = test_config();
        let result = resolve_route(
            "gemini/gemini-2.5-flash",
            &config,
            Some("tenant-specific-key-123"),
        )
        .unwrap();
        match result {
            ResolvedRoute::External { api_key, .. } => {
                assert_eq!(api_key, "tenant-specific-key-123");
            }
            _ => panic!("Expected External route"),
        }
    }

    #[test]
    fn test_provider_key_header_works_without_config_key() {
        let config = test_config_no_keys();
        let result = resolve_route(
            "openrouter/anthropic/claude-3.5-sonnet",
            &config,
            Some("tenant-only-key"),
        )
        .unwrap();
        match result {
            ResolvedRoute::External { api_key, .. } => {
                assert_eq!(api_key, "tenant-only-key");
            }
            _ => panic!("Expected External route with tenant key"),
        }
    }

    // ── Error cases ─────────────────────────────────────────────────────

    #[test]
    fn test_external_route_without_any_key_fails() {
        let config = test_config_no_keys();
        let result = resolve_route("gemini/gemini-2.5-flash", &config, None);
        assert!(result.is_err());
        let (status, msg) = result.unwrap_err();
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert!(msg.contains("gemini"));
    }

    #[test]
    fn test_empty_provider_key_header_falls_back_to_config() {
        let config = test_config();
        let result = resolve_route("gemini/gemini-2.5-flash", &config, Some("")).unwrap();
        match result {
            ResolvedRoute::External { api_key, .. } => {
                assert_eq!(api_key, "AIzaSy-test-key"); // falls back to config
            }
            _ => panic!("Expected External route"),
        }
    }

    // ── extract_provider_key ────────────────────────────────────────────

    #[test]
    fn test_extract_provider_key_present() {
        let mut headers = axum::http::HeaderMap::new();
        headers.insert("x-provider-key", "my-secret-key".parse().unwrap());
        assert_eq!(
            extract_provider_key(&headers),
            Some("my-secret-key".to_string())
        );
    }

    #[test]
    fn test_extract_provider_key_missing() {
        let headers = axum::http::HeaderMap::new();
        assert_eq!(extract_provider_key(&headers), None);
    }
}
