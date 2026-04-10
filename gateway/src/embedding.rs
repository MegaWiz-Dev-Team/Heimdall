/// Native Rust Embedding & Reranking Engine for Heimdall.
///
/// Uses `fastembed` (ONNX Runtime + CoreML on Apple Silicon) to provide
/// OpenAI-compatible `/v1/embeddings` and `/v1/rerank` endpoints without
/// requiring a Python sidecar.

use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::post,
    Json, Router,
};
use fastembed::{
    EmbeddingModel, InitOptions, TextEmbedding,
    RerankerModel, TextRerank, RerankInitOptions, RerankResult,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::OnceCell;

use crate::AppState;

// ─── Model Singletons ───────────────────────────────────────────────────────

/// Lazily-initialized models (loaded on first request).
static EMBEDDING_ENGINE: OnceCell<tokio::sync::Mutex<TextEmbedding>> = OnceCell::const_new();
static RERANKER_ENGINE: OnceCell<tokio::sync::Mutex<TextRerank>> = OnceCell::const_new();

async fn get_embedding_engine() -> Result<&'static tokio::sync::Mutex<TextEmbedding>, String> {
    EMBEDDING_ENGINE
        .get_or_try_init(|| async {
            tracing::info!("🔄 Loading native embedding model: BAAI/bge-m3 (ONNX)...");
            let start = std::time::Instant::now();
            let model = TextEmbedding::try_new(
                InitOptions::new(EmbeddingModel::BGEM3)
                    .with_show_download_progress(true),
            )
            .map_err(|e| format!("Failed to load embedding model: {}", e))?;
            tracing::info!(
                "✅ Embedding model loaded in {:.1}s",
                start.elapsed().as_secs_f64()
            );
            Ok(tokio::sync::Mutex::new(model))
        })
        .await
        .map_err(|e: String| e)
}

async fn get_reranker_engine() -> Result<&'static tokio::sync::Mutex<TextRerank>, String> {
    RERANKER_ENGINE
        .get_or_try_init(|| async {
            tracing::info!("🔄 Loading native reranker model (ONNX)...");
            let start = std::time::Instant::now();
            let model = TextRerank::try_new(
                RerankInitOptions::new(RerankerModel::BGERerankerBase)
                    .with_show_download_progress(true),
            )
            .map_err(|e| format!("Failed to load reranker model: {}", e))?;
            tracing::info!(
                "✅ Reranker model loaded in {:.1}s",
                start.elapsed().as_secs_f64()
            );
            Ok(tokio::sync::Mutex::new(model))
        })
        .await
        .map_err(|e: String| e)
}

// ─── Request / Response Types (OpenAI-compatible) ───────────────────────────

#[derive(Deserialize)]
pub struct EmbeddingRequest {
    pub model: Option<String>,
    pub input: serde_json::Value, // String or Vec<String>
}

#[derive(Serialize)]
pub struct EmbeddingResponse {
    pub object: String,
    pub data: Vec<EmbeddingData>,
    pub model: String,
    pub usage: EmbeddingUsage,
}

#[derive(Serialize)]
pub struct EmbeddingData {
    pub object: String,
    pub index: usize,
    pub embedding: Vec<f32>,
}

#[derive(Serialize)]
pub struct EmbeddingUsage {
    pub prompt_tokens: usize,
    pub total_tokens: usize,
}

#[derive(Deserialize)]
pub struct RerankRequest {
    pub query: String,
    pub texts: Vec<String>,
    pub model: Option<String>,
}

#[derive(Serialize)]
pub struct RerankResultItem {
    pub index: usize,
    pub score: f32,
}

// ─── Handlers ───────────────────────────────────────────────────────────────

async fn handle_embeddings(
    State(_state): State<AppState>,
    Json(payload): Json<EmbeddingRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let engine = get_embedding_engine().await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": {"message": e, "type": "model_error"}})),
        )
    })?;

    // Parse input: can be a single string or array of strings
    let texts: Vec<String> = match &payload.input {
        serde_json::Value::String(s) => vec![s.clone()],
        serde_json::Value::Array(arr) => arr
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect(),
        _ => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": {"message": "input must be a string or array of strings"}})),
            ))
        }
    };

    if texts.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": {"message": "input is empty"}})),
        ));
    }

    let start = std::time::Instant::now();
    let mut engine_guard = engine.lock().await;
    let embeddings = engine_guard.embed(texts.clone(), None).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": {"message": format!("Embedding failed: {}", e)}})),
        )
    })?;
    drop(engine_guard);
    let elapsed = start.elapsed();

    let token_count: usize = texts.iter().map(|t| t.split_whitespace().count()).sum();

    let data: Vec<EmbeddingData> = embeddings
        .into_iter()
        .enumerate()
        .map(|(i, emb)| EmbeddingData {
            object: "embedding".to_string(),
            index: i,
            embedding: emb,
        })
        .collect();

    tracing::info!(
        "🧮 Embedded {} texts in {:.3}s (native ONNX)",
        texts.len(),
        elapsed.as_secs_f64()
    );

    let model_name = payload.model.unwrap_or_else(|| "BAAI/bge-m3".to_string());

    Ok(Json(EmbeddingResponse {
        object: "list".to_string(),
        data,
        model: model_name,
        usage: EmbeddingUsage {
            prompt_tokens: token_count,
            total_tokens: token_count,
        },
    }))
}

async fn handle_rerank(
    State(_state): State<AppState>,
    Json(payload): Json<RerankRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let engine = get_reranker_engine().await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": {"message": e}})),
        )
    })?;

    if payload.texts.is_empty() || payload.query.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": {"message": "Missing query or texts"}})),
        ));
    }

    let start = std::time::Instant::now();
    let doc_strs: Vec<&str> = payload.texts.iter().map(|s| s.as_str()).collect();
    
    let mut engine_guard = engine.lock().await;
    let results = engine_guard
        .rerank(payload.query.as_str(), &doc_strs, false, None)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": {"message": format!("Rerank failed: {}", e)}})),
            )
        })?;
    drop(engine_guard);

    let elapsed = start.elapsed();

    let items: Vec<RerankResultItem> = results
        .into_iter()
        .map(|r| RerankResultItem {
            index: r.index,
            score: r.score as f32,
        })
        .collect();

    tracing::info!(
        "🔀 Reranked {} texts in {:.3}s (native ONNX)",
        payload.texts.len(),
        elapsed.as_secs_f64()
    );

    Ok(Json(items))
}

// ─── Routes ─────────────────────────────────────────────────────────────────

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/v1/embeddings", post(handle_embeddings))
        .route("/v1/rerank", post(handle_rerank))
        .route("/rerank", post(handle_rerank))
}
