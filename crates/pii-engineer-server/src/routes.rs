use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use pii_engineer_core::lang::has_chinese;
use pii_engineer_core::labels::canonicalize;
use pii_engineer_core::pipeline::default_labels;
use pii_engineer_core::{run_pipeline, ChineseNer, Entity, GlinerSpanModel, PipelineConfig};
use rust_embed::Embed;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::state::AppState;

#[derive(Embed)]
#[folder = "../../static/"]
struct StaticAssets;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/", get(index))
        .route("/api/health", get(health))
        .route("/api/detect", post(detect))
        .route("/blog", get(blog))
        .route("/blog/en", get(blog))
        .route("/blog/zh", get(blog))
        .route("/blog/vi", get(blog))
        .route("/blog/post/:lang/:slug", get(blog))
        .route("/benchmarks", get(benchmarks))
        .route("/docs.html", get(docs))
        .route("/sitemap.xml", get(sitemap))
        .route("/robots.txt", get(robots))
        .route("/static/*path", get(serve_static))
        .fallback(not_found)
        .with_state(state)
}

fn embedded_html(name: &str) -> Response {
    match StaticAssets::get(name) {
        Some(file) => (
            [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
            file.data,
        )
            .into_response(),
        None => (StatusCode::NOT_FOUND, "not found").into_response(),
    }
}

async fn serve_static(Path(path): Path<String>) -> Response {
    match StaticAssets::get(&path) {
        Some(file) => {
            let mime = mime_guess::from_path(&path)
                .first_or_octet_stream()
                .to_string();
            (
                [
                    (header::CONTENT_TYPE, mime),
                    (header::CACHE_CONTROL, "public, max-age=86400".to_string()),
                ],
                file.data,
            )
                .into_response()
        }
        None => (StatusCode::NOT_FOUND, "not found").into_response(),
    }
}

async fn blog() -> Response {
    embedded_html("blog.html")
}

async fn sitemap() -> impl IntoResponse {
    let base = "https://pii.engineer";

    let mut urls = vec![
        (format!("{base}/"), "1.0", "weekly"),
        (format!("{base}/benchmarks"), "0.9", "monthly"),
        (format!("{base}/docs.html"), "0.8", "monthly"),
        (format!("{base}/blog"), "0.9", "weekly"),
    ];

    #[derive(Deserialize)]
    struct Posts {
        languages: Vec<String>,
        posts: Vec<Post>,
    }
    #[derive(Deserialize)]
    struct Post {
        slug: String,
        langs: Vec<String>,
    }

    if let Some(file) = StaticAssets::get("blog/posts.json") {
        if let Ok(posts) = serde_json::from_slice::<Posts>(&file.data) {
            for l in &posts.languages {
                if l != "en" {
                    urls.push((format!("{base}/blog/{l}"), "0.7", "weekly"));
                }
            }
            for p in &posts.posts {
                for l in &p.langs {
                    urls.push((format!("{base}/blog/post/{l}/{}", p.slug), "0.8", "monthly"));
                }
            }
        }
    }

    let mut xml = String::from(r#"<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">"#);

    for (loc, priority, freq) in &urls {
        xml.push_str(&format!(
            "\n  <url>\n    <loc>{loc}</loc>\n    <changefreq>{freq}</changefreq>\n    <priority>{priority}</priority>\n  </url>"
        ));
    }
    xml.push_str("\n</urlset>\n");

    (
        [(axum::http::header::CONTENT_TYPE, "application/xml")],
        xml,
    )
}

async fn robots() -> impl IntoResponse {
    (
        [(axum::http::header::CONTENT_TYPE, "text/plain")],
        "User-agent: *\nAllow: /\n\nSitemap: https://pii.engineer/sitemap.xml\n",
    )
}

async fn benchmarks() -> Response {
    embedded_html("benchmarks.html")
}

async fn docs() -> Response {
    embedded_html("docs.html")
}

async fn index() -> Response {
    embedded_html("index.html")
}


async fn not_found() -> Response {
    match StaticAssets::get("404.html") {
        Some(file) => (
            StatusCode::NOT_FOUND,
            [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
            file.data,
        )
            .into_response(),
        None => (StatusCode::NOT_FOUND, "404 Not Found").into_response(),
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
