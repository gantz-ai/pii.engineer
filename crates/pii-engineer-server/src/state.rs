use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result};
use pii_engineer_core::{ChineseNer, GlinerSpanModel};

use crate::config::Settings;
use crate::download;
use crate::middleware::RateLimiter;

#[derive(Clone)]
pub struct AppState {
    pub settings: Arc<Settings>,
    pub gliner: Option<Arc<GlinerSpanModel>>,
    pub chinese: Option<Arc<ChineseNer>>,
    pub limiter: Arc<RateLimiter>,
}

impl AppState {
    pub fn new(settings: Settings) -> Result<Self> {
        if let Some(p) = settings.gliner_models.first() {
            if let Err(e) = download::ensure_gliner_model(p) {
                tracing::warn!("auto-download GLiNER failed: {e:#}");
            }
        }
        if let Err(e) = download::ensure_chinese_ner_model(&settings.chinese_ner_model) {
            tracing::warn!("auto-download Chinese NER failed: {e:#}");
        }

        let gliner = settings
            .gliner_models
            .first()
            .and_then(|p| Self::try_load_gliner(p).transpose())
            .transpose()?
            .map(Arc::new);

        if let Some(ref g) = gliner {
            g.warm_up();
        }

        let chinese = Self::try_load_chinese(&settings)?.map(Arc::new);

        if let Some(ref c) = chinese {
            c.warm_up();
        }

        Ok(Self {
            settings: Arc::new(settings),
            gliner,
            chinese,
            limiter: Arc::new(RateLimiter::new()),
        })
    }

    fn try_load_gliner(path: &str) -> Result<Option<GlinerSpanModel>> {
        let p = Path::new(path);
        let onnx = p.join("onnx");
        let has_v1 = onnx.join("model.onnx").exists();
        let has_v2_compat = onnx.join("encoder.onnx").exists() && onnx.join("scorer.onnx").exists();
        let has_encoder = onnx.join("encoder.onnx").exists() || onnx.join("encoder_int8.onnx").exists();
        let has_v2_full = has_encoder && onnx.join("span_rep.onnx").exists();
        if !has_v1 && !has_v2_compat && !has_v2_full {
            tracing::warn!(model = %path, "skipping GLiNER load: no ONNX model found in {}", p.join("onnx").display());
            return Ok(None);
        }
        let model = GlinerSpanModel::load(p)
            .with_context(|| format!("loading GLiNER model from {path}"))?;
        tracing::info!(model = %path, "GLiNER loaded");
        Ok(Some(model))
    }

    fn try_load_chinese(settings: &Settings) -> Result<Option<ChineseNer>> {
        let dir = PathBuf::from(&settings.chinese_ner_model);
        if !dir.join("model.onnx").exists() {
            tracing::warn!(
                "skipping Chinese NER: {} not found",
                dir.join("model.onnx").display()
            );
            return Ok(None);
        }
        let model = ChineseNer::load(&dir)
            .with_context(|| format!("loading Chinese NER from {}", dir.display()))?;
        tracing::info!(dir = %dir.display(), "Chinese NER loaded");
        Ok(Some(model))
    }
}
