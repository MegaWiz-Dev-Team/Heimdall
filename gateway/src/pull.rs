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
    /// Optional git revision (branch, tag, or commit SHA) to pin the download to.
    /// Recommended for `MegawizCo/*` production models so a swapped upstream
    /// cannot be silently pulled.
    #[serde(default)]
    pub revision: Option<String>,
}

#[derive(Serialize)]
pub struct PullResponse {
    pub status: String,
    pub message: Option<String>,
}

pub fn routes() -> Router<AppState> {
    Router::new().route("/pull", post(pull_model))
}

/// Validates a Hugging Face repo id of the form `namespace/name`.
/// Only ASCII alphanumerics and `.`, `_`, `-` are allowed in each segment,
/// with exactly one `/` separator. Rejects anything that could be abused to
/// break out of the download invocation.
fn is_valid_repo_id(id: &str) -> bool {
    let mut parts = id.split('/');
    match (parts.next(), parts.next(), parts.next()) {
        (Some(ns), Some(name), None) => is_valid_segment(ns) && is_valid_segment(name),
        _ => false,
    }
}

fn is_valid_segment(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 96
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
}

/// Validates a git revision (branch, tag, or commit SHA).
fn is_valid_revision(rev: &str) -> bool {
    !rev.is_empty()
        && rev.len() <= 128
        && rev
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-' | b'/'))
}

pub async fn pull_model(
    State(_state): State<AppState>,
    Json(payload): Json<PullRequest>,
) -> Result<Json<PullResponse>, (axum::http::StatusCode, Json<PullResponse>)> {
    // Reject anything that isn't a well-formed `namespace/name` HF repo id.
    // Defence-in-depth: the model name is also passed as an argv (never
    // interpolated into executed code), so this is a second line, not the only one.
    if !is_valid_repo_id(&payload.model) {
        error!("Rejected pull for malformed model id: {:?}", payload.model);
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            Json(PullResponse {
                status: "error".into(),
                message: Some("Invalid model id (expected `namespace/name`)".into()),
            }),
        ));
    }
    if let Some(rev) = payload.revision.as_deref() {
        if !is_valid_revision(rev) {
            error!("Rejected pull for malformed revision: {:?}", rev);
            return Err((
                axum::http::StatusCode::BAD_REQUEST,
                Json(PullResponse {
                    status: "error".into(),
                    message: Some("Invalid revision".into()),
                }),
            ));
        }
    }

    match payload.revision.as_deref() {
        Some(rev) => info!("Starting model pull: {} @ {}", payload.model, rev),
        None => info!("Starting model pull: {} (no revision pin)", payload.model),
    }

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

    // The model id and revision are passed as argv, NOT interpolated into the
    // script body, so a hostile value is inert data and cannot execute code.
    const DOWNLOAD_SCRIPT: &str = "import sys\n\
from huggingface_hub import snapshot_download\n\
model = sys.argv[1]\n\
revision = sys.argv[2] if len(sys.argv) > 2 and sys.argv[2] else None\n\
snapshot_download(model, revision=revision)\n";

    let mut command = tokio::process::Command::new(python_cmd);
    command.arg("-c").arg(DOWNLOAD_SCRIPT).arg(&payload.model);
    if let Some(rev) = payload.revision.as_deref() {
        command.arg(rev);
    }

    let child = command
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
