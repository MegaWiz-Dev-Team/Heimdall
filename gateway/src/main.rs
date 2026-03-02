mod auth;
mod config;
mod health;
mod metrics_handler;
mod proxy;

use axum::{
    Router,
    middleware,
};
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use config::AppConfig;

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<AppConfig>,
    pub http_client: reqwest::Client,
}

#[tokio::main]
async fn main() {
    // Load .env from project root
    let _ = dotenvy::from_path("../.env");
    let _ = dotenvy::dotenv();

    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "llm_gateway=info,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load config
    let config = AppConfig::from_env();
    tracing::info!(
        "LLM Gateway starting — backend: {}:{}",
        config.backend_host,
        config.backend_port
    );

    // Setup Prometheus metrics
    let prometheus_handle = metrics_handler::setup_metrics();

    // Build HTTP client for proxying
    let http_client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(300)) // 5 min for long generations
        .build()
        .expect("Failed to build HTTP client");

    let state = AppState {
        config: Arc::new(config.clone()),
        http_client,
    };

    // Build router
    let app = Router::new()
        // Health & metrics
        .merge(health::routes())
        .merge(metrics_handler::routes(prometheus_handle))
        // OpenAI-compatible API proxy
        .merge(proxy::routes())
        // Middleware
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth::auth_middleware,
        ))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state);

    // Bind and serve
    let addr = format!("{}:{}", config.host, config.gateway_port);
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("Failed to bind address");

    tracing::info!("🚀 LLM Gateway listening on {}", addr);
    tracing::info!(
        "   Proxying to http://{}:{}",
        config.backend_host,
        config.backend_port
    );

    axum::serve(listener, app)
        .await
        .expect("Server error");
}
