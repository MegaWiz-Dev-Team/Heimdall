mod auth;
mod embedding;
mod config;
mod health;
mod metrics_handler;
mod proxy;
mod router;
mod telemetry;
mod gpu;
mod pull;
use axum::{
    Router,
    middleware,
    routing::get,
    response::Html,
};
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use opentelemetry_otlp::WithExportConfig;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use config::AppConfig;

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<AppConfig>,
    pub http_client: reqwest::Client,
    pub active_model: Arc<std::sync::RwLock<String>>,
    pub swap_lock: Arc<tokio::sync::Mutex<()>>,
}

#[tokio::main]
async fn main() {
    // Load .env from project root
    let _ = dotenvy::from_path("../.env");
    let _ = dotenvy::dotenv();

    // Initialize OpenTelemetry Tracing
    let tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint("http://otel-collector.infra.svc:4317"),
        )
        .install_batch(opentelemetry_sdk::runtime::Tokio)
        .expect("Failed to initialize OTLP Tracer");

    let telemetry_layer = tracing_opentelemetry::layer().with_tracer(tracer);

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "heimdall_gateway=info,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .with(telemetry_layer)
        .init();

    // Load config
    let config = AppConfig::from_env();
    tracing::info!(
        "⚡ Heimdall starting — backend: {}:{}",
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

    let active_model = Arc::new(std::sync::RwLock::new(config.llm_model.clone()));
    let swap_lock = Arc::new(tokio::sync::Mutex::new(()));

    let state = AppState {
        config: Arc::new(config.clone()),
        http_client,
        active_model,
        swap_lock,
    };

    // Build router
    let app = Router::new()
        // Health & metrics
        .merge(health::routes())
        .merge(metrics_handler::routes(prometheus_handle))
        .merge(gpu::routes())
        // Native Rust embedding & reranking (handled before proxy)
        .merge(embedding::routes())
        // Model pull mechanism
        .merge(pull::routes())
        // OpenAI-compatible API proxy (chat/completions, models, etc.)
        .merge(proxy::routes())
        // Root HTML Dashboard
        .route("/", get(|| async { Html(include_str!("index.html")) }))
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

    tracing::info!("🛡️ Heimdall Gateway listening on {}", addr);
    tracing::info!(
        "   Proxying to http://{}:{}",
        config.backend_host,
        config.backend_port
    );

    axum::serve(listener, app)
        .await
        .expect("Server error");
}
