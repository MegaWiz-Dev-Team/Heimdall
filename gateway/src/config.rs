/// Application configuration loaded from environment variables.

#[derive(Debug, Clone)]
pub struct AppConfig {
    /// Gateway listen host
    pub host: String,
    /// Gateway listen port
    pub gateway_port: u16,
    /// Backend engine host
    pub backend_host: String,
    /// Backend engine port
    pub backend_port: u16,
    /// Embedding server host
    pub embedding_host: String,
    /// Embedding server port
    pub embedding_port: u16,
    /// VLM backend port for MegawizCo/typhoon-ocr-*-mlx-q4 (Sprint 51)
    pub vlm_q4_port: u16,
    /// VLM backend port for MegawizCo/typhoon-ocr-*-mlx-q8
    pub vlm_q8_port: u16,
    /// API keys (comma-separated)
    pub api_keys: Vec<String>,
    /// Whether auth is enabled
    pub auth_enabled: bool,
    /// Initially loaded LLM model
    pub llm_model: String,
    /// Project root directory
    pub project_dir: String,

    // ── External Provider Keys (Centralized Secret Store) ──────────────
    /// OpenRouter API key (fallback if X-Provider-Key header is not set)
    pub openrouter_api_key: Option<String>,
    /// OpenRouter base URL
    pub openrouter_base_url: String,
    /// Google Gemini API key
    pub gemini_api_key: Option<String>,
    /// Gemini OpenAI-compatible base URL
    pub gemini_base_url: String,
    /// OpenAI API key
    pub openai_api_key: Option<String>,
    /// OpenAI base URL
    pub openai_base_url: String,

    // ── Yggdrasil JWT Auth (Sprint 52) ─────────────────────────────────
    /// OIDC issuer URL (Yggdrasil/Zitadel). Set to enable JWT validation
    /// alongside static API_KEYS. Empty = JWT mode off (legacy only).
    pub yggdrasil_issuer: Option<String>,
    /// Expected `aud` claim in incoming JWTs (typically "heimdall").
    pub jwt_audience: Option<String>,
}

impl AppConfig {
    pub fn from_env() -> Self {
        let api_keys_str = std::env::var("API_KEYS").unwrap_or_default();
        let api_keys: Vec<String> = api_keys_str
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        let auth_enabled = !api_keys.is_empty();

        Self {
            host: std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".into()),
            gateway_port: std::env::var("GATEWAY_PORT")
                .unwrap_or_else(|_| "3000".into())
                .parse()
                .expect("GATEWAY_PORT must be a number"),
            backend_host: std::env::var("BACKEND_HOST")
                .unwrap_or_else(|_| "127.0.0.1".into()),
            backend_port: std::env::var("BACKEND_PORT")
                .unwrap_or_else(|_| "8081".into())
                .parse()
                .expect("BACKEND_PORT must be a number"),
            embedding_host: std::env::var("EMBEDDING_HOST")
                .unwrap_or_else(|_| "127.0.0.1".into()),
            embedding_port: std::env::var("EMBEDDING_PORT")
                .unwrap_or_else(|_| "8001".into())
                .parse()
                .expect("EMBEDDING_PORT must be a number"),
            vlm_q4_port: std::env::var("VLM_Q4_PORT")
                .unwrap_or_else(|_| "8082".into())
                .parse()
                .expect("VLM_Q4_PORT must be a number"),
            vlm_q8_port: std::env::var("VLM_Q8_PORT")
                .unwrap_or_else(|_| "8083".into())
                .parse()
                .expect("VLM_Q8_PORT must be a number"),
            api_keys,
            auth_enabled,
            llm_model: std::env::var("LLM_MODEL").unwrap_or_else(|_| "".into()),
            project_dir: std::env::var("PROJECT_DIR").unwrap_or_else(|_| "../".into()),

            // External providers
            openrouter_api_key: std::env::var("OPENROUTER_API_KEY").ok().filter(|s| !s.is_empty()),
            openrouter_base_url: std::env::var("OPENROUTER_BASE_URL")
                .unwrap_or_else(|_| "https://openrouter.ai/api/v1".into()),
            gemini_api_key: std::env::var("GEMINI_API_KEY").ok().filter(|s| !s.is_empty()),
            gemini_base_url: std::env::var("GEMINI_BASE_URL")
                .unwrap_or_else(|_| "https://generativelanguage.googleapis.com/v1beta/openai".into()),
            openai_api_key: std::env::var("OPENAI_API_KEY").ok().filter(|s| !s.is_empty()),
            openai_base_url: std::env::var("OPENAI_BASE_URL")
                .unwrap_or_else(|_| "https://api.openai.com/v1".into()),

            yggdrasil_issuer: std::env::var("YGGDRASIL_ISSUER")
                .ok()
                .filter(|s| !s.is_empty()),
            jwt_audience: std::env::var("JWT_AUDIENCE").ok().filter(|s| !s.is_empty()),
        }
    }

    /// JWT validation is enabled iff both issuer and audience are configured.
    pub fn jwt_enabled(&self) -> bool {
        self.yggdrasil_issuer.is_some() && self.jwt_audience.is_some()
    }

    /// Get the full backend URL
    pub fn backend_url(&self) -> String {
        format!("http://{}:{}", self.backend_host, self.backend_port)
    }

    /// Get the full embedding server URL
    pub fn embedding_url(&self) -> String {
        format!("http://{}:{}", self.embedding_host, self.embedding_port)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        // Clear env vars to test defaults
        std::env::remove_var("HOST");
        std::env::remove_var("GATEWAY_PORT");
        std::env::remove_var("BACKEND_HOST");
        std::env::remove_var("BACKEND_PORT");
        std::env::remove_var("API_KEYS");

        let config = AppConfig::from_env();
        assert_eq!(config.host, "0.0.0.0");
        assert_eq!(config.gateway_port, 3000);
        assert_eq!(config.backend_host, "127.0.0.1");
        assert_eq!(config.backend_port, 8081);
    }

    #[test]
    fn test_api_keys_parsing() {
        // Test the parsing logic directly without env vars
        let keys_str = "key1,key2,key3";
        let keys: Vec<String> = keys_str
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        assert_eq!(keys.len(), 3);
        assert_eq!(keys[0], "key1");
        assert_eq!(keys[2], "key3");
    }

    #[test]
    fn test_empty_api_keys() {
        let keys_str = "";
        let keys: Vec<String> = keys_str
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        assert!(keys.is_empty());
        assert!(!(!keys.is_empty())); // auth_enabled = false
    }

    #[test]
    fn test_backend_url() {
        std::env::remove_var("BACKEND_HOST");
        std::env::remove_var("BACKEND_PORT");
        let config = AppConfig::from_env();
        assert_eq!(config.backend_url(), "http://127.0.0.1:8081");
    }
}
