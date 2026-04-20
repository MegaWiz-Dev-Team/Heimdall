use axum::{
    extract::State,
    routing::post,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use tracing::{info, error};

use crate::AppState;

#[derive(Deserialize)]
pub struct PullRequest {
    pub model: String,
}

#[derive(Serialize)]
pub struct PullResponse {
    pub status: String,
    pub message: Option<String>,
}

pub fn routes() -> Router<AppState> {
    Router::new().route("/pull", post(pull_model))
}

pub async fn pull_model(
    State(_state): State<AppState>,
    Json(payload): Json<PullRequest>,
) -> Result<Json<PullResponse>, (axum::http::StatusCode, Json<PullResponse>)> {
    info!("Starting model pull via huggingface-cli: {}", payload.model);

    // Find the python executable in the local virtual environment
    let python_path = std::env::current_dir()
        .unwrap_or_else(|_| std::path::PathBuf::from("."))
        .parent()
        .unwrap()
        .join(".venv/bin/python");

    let python_cmd = if python_path.exists() {
        python_path
    } else {
        std::path::PathBuf::from("python3")
    };

    let download_script = format!(
        "from huggingface_hub import snapshot_download; snapshot_download('{}')",
        payload.model
    );

    let child = tokio::process::Command::new(python_cmd)
        .arg("-c")
        .arg(download_script)
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()
        .await;

    match child {
        Ok(status) if status.success() => {
            info!("Model pull successful: {}", payload.model);
            Ok(Json(PullResponse {
                status: "success".into(),
                message: Some(format!("Successfully pulled {}", payload.model)),
            }))
        }
        Ok(status) => {
            error!("Model pull failed: exited with status {}", status);
            Err((
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(PullResponse {
                    status: "error".into(),
                    message: Some("Failed to download model".into()),
                }),
            ))
        }
        Err(e) => {
            error!("Failed to spawn huggingface-cli: {}", e);
            Err((
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(PullResponse {
                    status: "error".into(),
                    message: Some("Could not invoke huggingface-cli".into()),
                }),
            ))
        }
    }
}
