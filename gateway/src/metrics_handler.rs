/// Prometheus metrics endpoint.

use axum::{routing::get, Router};
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};

use crate::AppState;

/// Initialize Prometheus metrics recorder.
pub fn setup_metrics() -> PrometheusHandle {
    let builder = PrometheusBuilder::new();
    builder
        .install_recorder()
        .expect("Failed to install Prometheus recorder")
}

pub fn routes(handle: PrometheusHandle) -> Router<AppState> {
    Router::new().route(
        "/metrics",
        get(move || {
            let h = handle.clone();
            async move { h.render() }
        }),
    )
}
