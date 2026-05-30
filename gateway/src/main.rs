mod auth;
mod auth_jwt;
mod embedding;
mod config;
mod health;
mod metrics_handler;
mod proxy;
mod router;
mod telemetry;
mod gpu;
mod pull;
mod skuggi;
mod tenant_config;
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
    /// 🌑 Skuggi tenant config cache. `None` when MARIADB_URL is unset
    /// (dev/test mode); proxy falls back to env-var SKUGGI_MODE.
    pub tenant_cfg: Option<Arc<tenant_config::TenantConfigCache>>,
    /// 🌳 Yggdrasil JWT validator. `None` when YGGDRASIL_ISSUER/JWT_AUDIENCE
    /// are unset; auth falls back to static API_KEYS only.
    pub jwt_validator: Option<Arc<auth_jwt::JwtValidator>>,
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

    // 🌑 Skuggi rule set. Loaded eagerly so the active detector count is
    // logged at boot and a bad `SKUGGI_RULES_PATH` surfaces here rather than
    // on the first proxied request. Falls back to the builtin set when the
    // env var is unset (dev/public builds); commercial/on-prem boxes point it
    // at a private skuggi-rules.toml. See ADR-023 / OPEN_CORE_POLICY.md.
    {
        let rule_count = skuggi_core::init_default_rules();
        match std::env::var("SKUGGI_RULES_PATH") {
            Ok(p) if !p.trim().is_empty() => {
                tracing::info!("🌑 Skuggi rule set loaded from {p} ({rule_count} detectors)");
            }
            _ => {
                tracing::info!("🌑 Skuggi using builtin rule set ({rule_count} detectors)");
            }
        }
    }

    // 🌑 Skuggi tenant config cache. Optional — Heimdall starts fine
    // without MariaDB (e.g. dev / unit-test environments); the proxy
    // falls back to env-var SKUGGI_MODE in that case.
    let tenant_cfg = match std::env::var("MARIADB_URL") {
        Ok(url) if !url.is_empty() => {
            match sqlx::mysql::MySqlPoolOptions::new()
                .max_connections(5)
                .acquire_timeout(std::time::Duration::from_secs(3))
                .connect(&url)
                .await
            {
                Ok(pool) => {
                    tracing::info!("🌑 Skuggi tenant config cache connected to MariaDB");
                    Some(Arc::new(tenant_config::TenantConfigCache::new(pool)))
                }
                Err(e) => {
                    tracing::warn!(
                        "🌑 Skuggi MariaDB connect failed ({}); falling back to env-var SKUGGI_MODE",
                        e
                    );
                    None
                }
            }
        }
        _ => {
            tracing::info!("🌑 Skuggi MARIADB_URL unset; using env-var SKUGGI_MODE fallback only");
            None
        }
    };

    let jwt_validator = if config.jwt_enabled() {
        let issuer = config.yggdrasil_issuer.clone().unwrap();
        let audience = config.jwt_audience.clone().unwrap();
        tracing::info!(
            issuer = %issuer,
            audience = %audience,
            "🌳 Yggdrasil JWT validator enabled"
        );
        Some(Arc::new(auth_jwt::JwtValidator::new(issuer, audience)))
    } else {
        tracing::info!("🌳 Yggdrasil JWT disabled (YGGDRASIL_ISSUER/JWT_AUDIENCE unset); static API_KEYS only");
        None
    };

    let state = AppState {
        config: Arc::new(config.clone()),
        http_client,
        active_model,
        swap_lock,
        tenant_cfg,
        jwt_validator,
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
