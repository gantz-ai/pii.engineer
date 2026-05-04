use std::sync::Arc;

use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use pii_engineer_core::lang::has_chinese;
use pii_engineer_core::labels::canonicalize;
use pii_engineer_core::pipeline::default_labels;
use pii_engineer_core::{run_pipeline, ChineseNer, Entity, GlinerSpanModel, PipelineConfig};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::state::AppState;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/", get(index))
        .route("/api/health", get(health))
        .route("/api/detect", post(detect))
        .fallback(not_found)
        .with_state(state)
}

async fn index() -> impl IntoResponse {
    let path = std::env::var("PII_ENGINEER_STATIC_DIR").unwrap_or_else(|_| "static".into());
    match tokio::fs::read_to_string(format!("{path}/index.html")).await {
        Ok(s) => axum::response::Html(s).into_response(),
        Err(_) => (
            StatusCode::NOT_FOUND,
            Json(json!({"error":"static_missing"})),
        )
            .into_response(),
    }
}


async fn not_found() -> impl IntoResponse {
    let path = std::env::var("PII_ENGINEER_STATIC_DIR").unwrap_or_else(|_| "static".into());
    match tokio::fs::read_to_string(format!("{path}/404.html")).await {
        Ok(s) => (StatusCode::NOT_FOUND, axum::response::Html(s)).into_response(),
        Err(_) => (StatusCode::NOT_FOUND, "404 Not Found").into_response(),
    }
}

async fn health(State(state): State<AppState>) -> impl IntoResponse {
    Json(json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
        "gliner_loaded": state.gliner.is_some(),
        "chinese_loaded": state.chinese.is_some(),
    }))
}

// ── Types ──────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct DetectRequest {
    text: String,
    #[serde(default)]
    labels: Option<Vec<String>>,
    #[serde(default)]
    boost: Vec<String>,
}

#[derive(Serialize, Clone)]
struct DetectEntity {
    #[serde(rename = "type")]
    type_: String,
    value: String,
    start: usize,
    end: usize,
    score: f32,
    needs_review: bool,
}

// ── NER helper ───────────────────────────────────────────────────────


struct NerResult {
    det_entities: Vec<DetectEntity>,
    redacted: String,
}

#[allow(clippy::too_many_arguments)]
fn run_ner(
    gliner: &GlinerSpanModel,
    chinese: &Option<Arc<ChineseNer>>,
    text: &str,
    labels: &[String],
    boost: &[String],
    cfg: &PipelineConfig,
    raw_threshold: f32,
    auto_redact: f32,
) -> pii_engineer_core::Result<NerResult> {
    use pii_engineer_core::labels::LABEL_DESCRIPTIONS;
    let mut raw: Vec<Entity> = if boost.is_empty() {
        gliner.detect(text, labels)?
    } else {
        let descs: Vec<(&str, &str)> = LABEL_DESCRIPTIONS
            .iter()
            .filter(|(l, _)| boost.iter().any(|b| b == l))
            .copied()
            .collect();
        if descs.is_empty() {
            gliner.detect(text, labels)?
        } else {
            gliner.detect_with_descriptions(text, labels, &descs)?
        }
    };
    if has_chinese(text) {
        if let Some(cn) = chinese.as_ref() {
            raw.extend(cn.predict(text, raw_threshold)?);
        }
    }
    let entities = run_pipeline(raw, text, cfg);
    let det_entities: Vec<DetectEntity> = entities
        .iter()
        .map(|e| {
            let canonical = canonicalize(&e.label)
                .map(|s| s.to_string())
                .unwrap_or(e.label.clone());
            DetectEntity {
                start: e.start,
                end: e.end,
                value: e.text.clone(),
                type_: canonical,
                score: e.score,
                needs_review: e.score < auto_redact,
            }
        })
        .collect();
    let redacted = build_redacted(text, &det_entities, false);
    Ok(NerResult {
        det_entities,
        redacted,
    })
}

// ── Detect (NER only) ────────────────────────────────────────────────

async fn detect(
    State(state): State<AppState>,
    Json(req): Json<DetectRequest>,
) -> Response {
    let Some(gliner) = state.gliner.clone() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({"error": "model_unavailable"})),
        )
            .into_response();
    };

    if req.text.len() > state.settings.max_text_length {
        return (
            StatusCode::PAYLOAD_TOO_LARGE,
            Json(json!({"error": "text_too_long", "limit": state.settings.max_text_length})),
        )
            .into_response();
    }

    let text = req.text.clone();
    let labels: Vec<String> = req.labels.unwrap_or_else(default_labels);
    let boost = req.boost;
    let cfg = PipelineConfig {
        review_threshold: state.settings.review_threshold,
        label_thresholds: state.settings.label_thresholds.clone(),
    };
    let chinese = state.chinese.clone();
    let raw_threshold = state.settings.raw_threshold;
    let auto_redact = state.settings.auto_redact_threshold;

    match tokio::task::spawn_blocking(move || {
        run_ner(&gliner, &chinese, &text, &labels, &boost, &cfg, raw_threshold, auto_redact)
    })
    .await
    {
        Ok(Ok(ner)) => Json(json!({
            "entities": ner.det_entities,
            "redacted": ner.redacted,
            "original": req.text,
        }))
        .into_response(),
        Ok(Err(e)) => {
            tracing::error!("detect error: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": format!("{e}")})),
            )
                .into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("{e}")})),
        )
            .into_response(),
    }
}

// ── Redaction helpers ──────────────────────────────────────────────────

fn build_redacted(text: &str, entities: &[DetectEntity], redact_all: bool) -> String {
    let mut selected: Vec<&DetectEntity> = if redact_all {
        entities.iter().collect()
    } else {
        entities.iter().filter(|e| !e.needs_review).collect()
    };
    selected.sort_by_key(|e| e.start);
    let mut out = String::with_capacity(text.len());
    let mut cursor = 0usize;
    for e in selected {
        if e.start < cursor || e.end > text.len() {
            continue;
        }
        out.push_str(&text[cursor..e.start]);
        out.push('[');
        out.push_str(&e.type_.to_uppercase());
        out.push(']');
        cursor = e.end;
    }
    if cursor < text.len() {
        out.push_str(&text[cursor..]);
    }
    out
}
