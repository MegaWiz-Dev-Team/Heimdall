use axum::{routing::get, Json, Router};
use serde::Serialize;
use sysinfo::System;

use crate::AppState;

#[derive(Serialize)]
pub struct GpuStatusResponse {
    pub vendor: String,
    pub model: String,
    pub vram_total_mb: u64,
    pub vram_used_mb: u64,
    pub utilization_pct: f64,
}

async fn get_gpu_status() -> Json<GpuStatusResponse> {
    // We only need memory statistics since Apple Silicon uses Unified Memory
    let mut sys = System::new();
    sys.refresh_memory();

    let total_bytes = sys.total_memory();
    let used_bytes = sys.used_memory();

    let vram_total_mb = total_bytes / 1024 / 1024;
    let vram_used_mb = used_bytes / 1024 / 1024;
    
    let utilization_pct = if total_bytes > 0 {
        (used_bytes as f64 / total_bytes as f64) * 100.0
    } else {
        0.0
    };

    Json(GpuStatusResponse {
        vendor: "Apple".to_string(),
        model: "Apple Silicon (Unified)".to_string(),
        vram_total_mb,
        vram_used_mb,
        utilization_pct: (utilization_pct * 100.0).round() / 100.0, // rounded to 2 decimals
    })
}

pub fn routes() -> Router<AppState> {
    Router::new().route("/api/gpu", get(get_gpu_status))
}
