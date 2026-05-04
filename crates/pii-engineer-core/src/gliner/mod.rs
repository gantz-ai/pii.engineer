//! Full GLiNER2 Rust inference: NER, classification, structured extraction, relations.
//!
//! v1 (legacy): single model.onnx with 6 inputs → logits
//! v2 (full):   5 ONNX models — encoder, span_rep, count_pred, count_embed, classifier
//! v2-compat:   2 ONNX models — encoder + scorer (NER only, from old export)

pub(crate) mod shared;
mod v1;
mod v2_compat;
mod v2_full;

use std::collections::HashMap;
use std::path::Path;

use ort::session::{builder::GraphOptimizationLevel, Session};
use regex::Regex;
use serde::Serialize;

use crate::error::{Error, Result};
use crate::labels::PROMPT_LABELS;

// ── Public types ────────────────────────────────────────────────────────���───

#[derive(Debug, Clone, Serialize)]
pub struct Entity {
    pub start: usize,
    pub end: usize,
    pub text: String,
    pub label: String,
    pub score: f32,
}

#[derive(Debug, Clone, Serialize)]
pub struct ClassificationResult {
    pub label: String,
    pub score: f32,
}

#[derive(Debug, Clone, Serialize)]
pub struct StructuredResult {
    pub instances: Vec<HashMap<String, Vec<Entity>>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RelationResult {
    pub instances: Vec<RelationInstance>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RelationInstance {
    pub head: Entity,
    pub tail: Entity,
    pub score: f32,
}

// ── Model backend ───────────────────────────────────────────────────────────

#[allow(clippy::large_enum_variant)]
pub(crate) enum ModelBackend {
    V1 {
        session: Session,
    },
    V2Compat {
        encoder: Session,
        scorer: Session,
        hidden_size: usize,
    },
    V2Full {
        encoder: Session,
        span_rep: Session,
        count_pred: Session,
        count_embed: Session,
        count_embed_ner: Option<Session>,
        classifier: Session,
        hidden_size: usize,
    },
}

// ── Schema token markers ────────────────────────────────────────────────────

pub(crate) struct SchemaMarker;
impl SchemaMarker {
    pub const E: &str = "[E]";
    pub const C: &str = "[C]";
    pub const R: &str = "[R]";
    pub const L: &str = "[L]";
    pub const P: &str = "[P]";
    pub const SEP_TEXT: &str = "[SEP_TEXT]";
}

// ── Main struct ─────────────────────────────────────────────────────────────

pub struct GlinerSpanModel {
    pub(crate) backend: ModelBackend,
    pub(crate) tokenizer: tokenizers::Tokenizer,
    pub(crate) word_re: Regex,
    pub(crate) max_width: usize,
}

impl GlinerSpanModel {
    pub fn load(model_dir: impl AsRef<Path>) -> Result<Self> {
        let model_dir = model_dir.as_ref();
        let tokenizer_path = model_dir.join("tokenizer.json");

        if !tokenizer_path.exists() {
            return Err(Error::Model(format!(
                "tokenizer not found: {}",
                tokenizer_path.display()
            )));
        }

        let mut tokenizer = tokenizers::Tokenizer::from_file(&tokenizer_path)
            .map_err(|e| Error::Tokenizer(format!("{e}")))?;
        tokenizer.with_truncation(Some(tokenizers::TruncationParams {
            max_length: 512,
            ..Default::default()
        })).map_err(|e| Error::Tokenizer(format!("{e}")))?;

        let onnx_dir = model_dir.join("onnx");

        let has_encoder = onnx_dir.join("encoder.onnx").exists()
            || onnx_dir.join("encoder_int8.onnx").exists();
        let has_full = has_encoder
            && onnx_dir.join("span_rep.onnx").exists()
            && onnx_dir.join("count_pred.onnx").exists()
            && onnx_dir.join("count_embed.onnx").exists()
            && onnx_dir.join("classifier.onnx").exists();

        if has_full {
            return Self::load_v2_full(model_dir, tokenizer);
        }

        if onnx_dir.join("encoder.onnx").exists() && onnx_dir.join("scorer.onnx").exists() {
            return Self::load_v2_compat(model_dir, tokenizer);
        }

        let int8_path = onnx_dir.join("model_int8.onnx");
        let fp32_path = onnx_dir.join("model.onnx");
        let onnx_path = if int8_path.exists() { &int8_path } else { &fp32_path };

        if !onnx_path.exists() {
            return Err(Error::Model(format!(
                "no ONNX model found in {}",
                onnx_dir.display()
            )));
        }

        Self::load_v1(tokenizer, onnx_path)
    }

    pub(crate) fn load_session(path: &Path) -> Result<Session> {
        let intra = shared::ort_intra_threads();
        let inter = shared::ort_inter_threads();
        Ok(Session::builder()?
            .with_optimization_level(GraphOptimizationLevel::Level3)?
            .with_intra_threads(intra)?
            .with_inter_threads(inter)?
            .with_intra_op_spinning(true)?
            .with_inter_op_spinning(false)?
            .with_parallel_execution(false)?
            .with_memory_pattern(true)?
            .commit_from_file(path)?)
    }

    fn read_config(model_dir: &Path) -> (usize, usize) {
        let config_path = model_dir.join("gliner_config.json");
        if config_path.exists() {
            if let Ok(text) = std::fs::read_to_string(&config_path) {
                if let Ok(data) = serde_json::from_str::<serde_json::Value>(&text) {
                    return (
                        data["max_width"].as_u64().unwrap_or(8) as usize,
                        data["hidden_size"].as_u64().unwrap_or(768) as usize,
                    );
                }
            }
        }
        (8, 768)
    }

    fn load_v1(tokenizer: tokenizers::Tokenizer, onnx_path: &Path) -> Result<Self> {
        tracing::info!(onnx = %onnx_path.display(), "loading GLiNER v1");
        let session = Self::load_session(onnx_path)?;
        Ok(Self {
            backend: ModelBackend::V1 { session },
            tokenizer,
            word_re: Regex::new(r"\w+(?:[-_]\w+)*|\S").unwrap(),
            max_width: 12,
        })
    }

    fn load_v2_compat(model_dir: &Path, tokenizer: tokenizers::Tokenizer) -> Result<Self> {
        tracing::info!(dir = %model_dir.display(), "loading GLiNER2 compat (encoder + scorer)");
        let onnx_dir = model_dir.join("onnx");
        let (max_width, hidden_size) = Self::read_config(model_dir);
        Ok(Self {
            backend: ModelBackend::V2Compat {
                encoder: Self::load_session(&onnx_dir.join("encoder.onnx"))?,
                scorer: Self::load_session(&onnx_dir.join("scorer.onnx"))?,
                hidden_size,
            },
            tokenizer,
            word_re: Regex::new(r"\w+(?:[-_]\w+)*|\S").unwrap(),
            max_width,
        })
    }

    fn load_v2_full(model_dir: &Path, tokenizer: tokenizers::Tokenizer) -> Result<Self> {
        tracing::info!(dir = %model_dir.display(), threads = shared::ort_intra_threads(), "loading GLiNER2 full (5 models)");
        let onnx_dir = model_dir.join("onnx");
        let (max_width, hidden_size) = Self::read_config(model_dir);
        let encoder_path = {
            let int8 = onnx_dir.join("encoder_int8.onnx");
            if int8.exists() {
                tracing::info!("using INT8 encoder");
                int8
            } else {
                onnx_dir.join("encoder.onnx")
            }
        };
        let ce_ner_path = onnx_dir.join("count_embed_ner.onnx");
        let count_embed_ner = if ce_ner_path.exists() {
            tracing::info!("using optimized 1-step count_embed for NER");
            Some(Self::load_session(&ce_ner_path)?)
        } else {
            None
        };
        Ok(Self {
            backend: ModelBackend::V2Full {
                encoder: Self::load_session(&encoder_path)?,
                span_rep: Self::load_session(&onnx_dir.join("span_rep.onnx"))?,
                count_pred: Self::load_session(&onnx_dir.join("count_pred.onnx"))?,
                count_embed: Self::load_session(&onnx_dir.join("count_embed.onnx"))?,
                count_embed_ner,
                classifier: Self::load_session(&onnx_dir.join("classifier.onnx"))?,
                hidden_size,
            },
            tokenizer,
            word_re: Regex::new(r"\w+(?:[-_]\w+)*|\S").unwrap(),
            max_width,
        })
    }

    // ── Public API ────────────────────────────────────────────────��─────────

    pub fn detect(&self, text: &str, labels: &[String]) -> Result<Vec<Entity>> {
        match &self.backend {
            ModelBackend::V1 { .. } => self.detect_v1(text, labels),
            ModelBackend::V2Compat { .. } => self.detect_v2_compat(text, labels),
            ModelBackend::V2Full { .. } => self.detect_v2_full(text, labels, None),
        }
    }

    pub fn detect_with_descriptions(
        &self,
        text: &str,
        labels: &[String],
        descriptions: &[(&str, &str)],
    ) -> Result<Vec<Entity>> {
        match &self.backend {
            ModelBackend::V1 { .. } => self.detect_v1(text, labels),
            ModelBackend::V2Compat { .. } => self.detect_v2_compat(text, labels),
            ModelBackend::V2Full { .. } => self.detect_v2_full(text, labels, Some(descriptions)),
        }
    }

    pub fn classify(
        &self,
        text: &str,
        labels: &[String],
        multi_label: bool,
    ) -> Result<Vec<ClassificationResult>> {
        self.classify_impl(text, labels, multi_label)
    }

    pub fn extract_structured(
        &self,
        text: &str,
        schema_name: &str,
        fields: &[String],
        threshold: f32,
    ) -> Result<StructuredResult> {
        self.extract_span_task(text, schema_name, fields, threshold, SchemaMarker::C)
    }

    pub fn warm_up(&self) {
        let labels: Vec<String> = PROMPT_LABELS.iter().map(|s| s.to_string()).collect();
        let t0 = std::time::Instant::now();
        let _ = self.detect("John Doe lives at 123 Main St", &labels);
        tracing::info!(ms = t0.elapsed().as_millis(), "gliner NER warm-up complete");
    }

    pub fn extract_relations(
        &self,
        text: &str,
        relation_name: &str,
        head_label: &str,
        tail_label: &str,
        threshold: f32,
    ) -> Result<RelationResult> {
        self.extract_relations_with_desc(text, relation_name, head_label, tail_label, threshold)
    }

    pub fn extract_relations_with_desc(
        &self,
        text: &str,
        relation_name: &str,
        head_label: &str,
        tail_label: &str,
        threshold: f32,
    ) -> Result<RelationResult> {
        let fields = vec!["head".to_string(), "tail".to_string()];
        let result = self.extract_span_task(text, relation_name, &fields, threshold, SchemaMarker::R)?;

        let instances: Vec<RelationInstance> = result.instances.into_iter()
            .filter_map(|mut m| {
                let mut head = m.remove("head")?.into_iter().next()?;
                let mut tail = m.remove("tail")?.into_iter().next()?;
                head.label = head_label.to_string();
                tail.label = tail_label.to_string();
                let score = (head.score + tail.score) / 2.0;
                Some(RelationInstance { head, tail, score })
            })
            .collect();

        Ok(RelationResult { instances })
    }
}
