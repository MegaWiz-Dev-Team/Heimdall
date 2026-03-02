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
    /// API keys (comma-separated)
    pub api_keys: Vec<String>,
    /// Whether auth is enabled
    pub auth_enabled: bool,
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
                .unwrap_or_else(|_| "8000".into())
                .parse()
                .expect("BACKEND_PORT must be a number"),
            api_keys,
            auth_enabled,
        }
    }

    /// Get the full backend URL
    pub fn backend_url(&self) -> String {
        format!("http://{}:{}", self.backend_host, self.backend_port)
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
        assert_eq!(config.backend_port, 8000);
        assert!(!config.auth_enabled);
        assert!(config.api_keys.is_empty());
    }

    #[test]
    fn test_config_with_api_keys() {
        std::env::set_var("API_KEYS", "key1,key2,key3");
        let config = AppConfig::from_env();
        assert_eq!(config.api_keys.len(), 3);
        assert!(config.auth_enabled);
        std::env::remove_var("API_KEYS");
    }

    #[test]
    fn test_backend_url() {
        std::env::remove_var("BACKEND_HOST");
        std::env::remove_var("BACKEND_PORT");
        let config = AppConfig::from_env();
        assert_eq!(config.backend_url(), "http://127.0.0.1:8000");
    }
}
