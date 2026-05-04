//! Chinese BERT token-classification NER (BIO scheme).
//! Tokenize with offsets, run logits, softmax, aggregate BIO spans.

use std::path::Path;
use std::sync::Mutex;

use ndarray::Array2;
use ort::session::{builder::GraphOptimizationLevel, Session};
use ort::value::Value;
use tokenizers::Tokenizer;

use crate::error::{Error, Result};
use crate::gliner::Entity;

pub struct ChineseNer {
    session: Mutex<Session>,
    tokenizer: Tokenizer,
    id2label: Vec<String>,
    has_token_type_ids: bool,
}

impl ChineseNer {
    /// Load from a directory containing `model.onnx`, `tokenizer.json`, and
    /// `id2label.json` (optional — falls back to default BIO labels).
    pub fn load(model_dir: impl AsRef<Path>) -> Result<Self> {
        let dir = model_dir.as_ref();
        let onnx = dir.join("model.onnx");
        let tok = dir.join("tokenizer.json");
        if !onnx.exists() {
            return Err(Error::Model(format!("missing {}", onnx.display())));
        }
        if !tok.exists() {
            return Err(Error::Model(format!("missing {}", tok.display())));
        }
        let session = Session::builder()?
            .with_optimization_level(GraphOptimizationLevel::Level3)?
            .with_intra_threads(num_cpus_or(8))?
            .commit_from_file(&onnx)?;
        let tokenizer = Tokenizer::from_file(&tok)
            .map_err(|e| Error::Tokenizer(format!("{e}")))?;

        let id2label = read_id2label(dir).unwrap_or_else(default_bio_labels);

        // Probe whether the graph wants token_type_ids by checking input names.
        let has_token_type_ids = session
            .inputs
            .iter()
            .any(|i| i.name == "token_type_ids");

        Ok(Self {
            session: Mutex::new(session),
            tokenizer,
            id2label,
            has_token_type_ids,
        })
    }

    pub fn warm_up(&self) {
        let t0 = std::time::Instant::now();
        let _ = self.predict("张三住在北京市朝阳区", 0.5);
        tracing::info!(ms = t0.elapsed().as_millis(), "chinese-ner warm-up complete");
    }

    pub fn predict(&self, text: &str, threshold: f32) -> Result<Vec<Entity>> {
        let enc = self
            .tokenizer
            .encode(text, true)
            .map_err(|e| Error::Tokenizer(format!("{e}")))?;

        let ids: Vec<i64> = enc.get_ids().iter().map(|&x| x as i64).collect();
        let mask: Vec<i64> = enc.get_attention_mask().iter().map(|&x| x as i64).collect();
        let type_ids: Vec<i64> = enc.get_type_ids().iter().map(|&x| x as i64).collect();
        let offsets = enc.get_offsets().to_vec();
        let special = enc.get_special_tokens_mask().to_vec();
        let seq = ids.len();

        let input_ids = Array2::from_shape_vec((1, seq), ids)
            .map_err(|e| Error::Model(format!("ids shape: {e}")))?;
        let attention_mask = Array2::from_shape_vec((1, seq), mask)
            .map_err(|e| Error::Model(format!("mask shape: {e}")))?;
        let token_type_ids = Array2::from_shape_vec((1, seq), type_ids)
            .map_err(|e| Error::Model(format!("tt shape: {e}")))?;

        let session = self.session.lock().map_err(|_| Error::Model("poisoned".into()))?;
        let outputs = if self.has_token_type_ids {
            session.run(ort::inputs![
                "input_ids" => Value::from_array(input_ids)?,
                "attention_mask" => Value::from_array(attention_mask)?,
                "token_type_ids" => Value::from_array(token_type_ids)?,
            ]?)?
        } else {
            session.run(ort::inputs![
                "input_ids" => Value::from_array(input_ids)?,
                "attention_mask" => Value::from_array(attention_mask)?,
            ]?)?
        };

        let view = outputs[0].try_extract_tensor::<f32>()?;
        let dims = view.shape();
        if dims.len() != 3 || dims[0] != 1 || dims[1] != seq {
            return Err(Error::Model(format!(
                "unexpected NER logits shape {:?}",
                dims
            )));
        }
        let n_labels = dims[2];

        // Argmax + softmax(max) per token, then aggregate BIO spans.
        let mut entities: Vec<Entity> = Vec::new();
        let mut current: Option<Entity> = None;
        let mut current_label: Option<&str> = None;

        for t in 0..seq {
            if special[t] != 0 {
                continue;
            }
            // Skip tokens with attention_mask=0 (padding handled by special, but be safe).
            let mut max_idx = 0usize;
            let mut max_logit = f32::NEG_INFINITY;
            let mut sum_exp = 0.0f32;
            // First pass: find max for stable softmax.
            for c in 0..n_labels {
                let v = view[[0, t, c]];
                if v > max_logit {
                    max_logit = v;
                    max_idx = c;
                }
            }
            for c in 0..n_labels {
                sum_exp += (view[[0, t, c]] - max_logit).exp();
            }
            let prob = 1.0 / sum_exp; // exp(0)/sum

            let raw = self
                .id2label
                .get(max_idx)
                .map(|s| s.as_str())
                .unwrap_or("O");
            if raw == "O" || prob < threshold {
                if let Some(e) = current.take() {
                    entities.push(e);
                }
                current_label = None;
                continue;
            }

            let (prefix, bare) = split_bio(raw);
            let (start, end) = offsets[t];
            let token_text = &text[start..end];

            let extend = matches!((prefix, current_label), ("I", Some(lbl)) if lbl == bare);

            if extend {
                if let Some(e) = current.as_mut() {
                    let between = &text[e.end..start];
                    e.text.push_str(between);
                    e.text.push_str(token_text);
                    e.end = end;
                    e.score = e.score.max(prob);
                }
            } else {
                if let Some(e) = current.take() {
                    entities.push(e);
                }
                current = Some(Entity {
                    start,
                    end,
                    text: token_text.to_string(),
                    label: bare.to_string(),
                    score: prob,
                });
                current_label = Some(bare_static(bare));
            }
        }
        if let Some(e) = current {
            entities.push(e);
        }
        Ok(entities)
    }
}

fn split_bio(tag: &str) -> (&str, &str) {
    if let Some((p, rest)) = tag.split_once('-') {
        (p, rest)
    } else {
        ("O", tag)
    }
}

// Workaround for the borrow lifetime: we re-derive the bare label from id2label
// each iteration so this just needs a 'static-ish handle. We pass through string
// equality only, so leak via a small interner.
fn bare_static(s: &str) -> &'static str {
    use once_cell::sync::Lazy;
    use std::collections::HashSet;
    use std::sync::Mutex;
    static POOL: Lazy<Mutex<HashSet<&'static str>>> = Lazy::new(Default::default);
    let mut pool = POOL.lock().unwrap();
    if let Some(existing) = pool.iter().find(|x| **x == s) {
        return existing;
    }
    let leaked: &'static str = Box::leak(s.to_string().into_boxed_str());
    pool.insert(leaked);
    leaked
}

fn default_bio_labels() -> Vec<String> {
    vec![
        "O".into(),
        "B-person".into(), "I-person".into(),
        "B-phone_number".into(), "I-phone_number".into(),
        "B-government_id".into(), "I-government_id".into(),
        "B-street_address".into(), "I-street_address".into(),
        "B-date_of_birth".into(), "I-date_of_birth".into(),
    ]
}

fn read_id2label(dir: &Path) -> Option<Vec<String>> {
    let cfg = std::fs::read_to_string(dir.join("config.json")).ok()?;
    let v: serde_json::Value = serde_json::from_str(&cfg).ok()?;
    let map = v.get("id2label")?.as_object()?;
    let mut pairs: Vec<(usize, String)> = map
        .iter()
        .filter_map(|(k, val)| Some((k.parse::<usize>().ok()?, val.as_str()?.to_string())))
        .collect();
    pairs.sort_by_key(|(i, _)| *i);
    Some(pairs.into_iter().map(|(_, s)| s).collect())
}

fn num_cpus_or(default: usize) -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(default)
}
