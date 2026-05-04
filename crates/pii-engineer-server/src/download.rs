use std::path::Path;

use anyhow::{Context, Result};
use hf_hub::api::sync::Api;

const GLINER_REPO: &str = "pii-engineer/PII-Engineer-Multi-NER-v2.1";
const CHINESE_NER_REPO: &str = "pii-engineer/PII-Engineer-Chinese-NER-v1.0";

// (remote_name, local_name) pairs — allows renaming on download.
const GLINER_FILES: &[(&str, &str)] = &[
    ("config.json", "gliner_config.json"),
    ("tokenizer.json", "tokenizer.json"),
    ("tokenizer_config.json", "tokenizer_config.json"),
    ("onnx/encoder_int8.onnx", "onnx/encoder_int8.onnx"),
    ("onnx/span_rep.onnx", "onnx/span_rep.onnx"),
    ("onnx/count_pred.onnx", "onnx/count_pred.onnx"),
    ("onnx/count_embed.onnx", "onnx/count_embed.onnx"),
    ("onnx/classifier.onnx", "onnx/classifier.onnx"),
];

const CHINESE_NER_FILES: &[(&str, &str)] = &[
    ("config.json", "config.json"),
    ("model.onnx", "model.onnx"),
    ("model.onnx.data", "model.onnx.data"),
    ("tokenizer.json", "tokenizer.json"),
    ("tokenizer_config.json", "tokenizer_config.json"),
];

pub fn ensure_gliner_model(model_dir: &str) -> Result<()> {
    let dir = Path::new(model_dir);
    let onnx_dir = dir.join("onnx");
    let ready = onnx_dir.join("encoder_int8.onnx").exists()
        || onnx_dir.join("encoder.onnx").exists();
    if ready {
        return Ok(());
    }
    tracing::info!("GLiNER model not found at {model_dir}, downloading from HuggingFace...");
    download_repo(GLINER_REPO, GLINER_FILES, dir)
        .context("downloading GLiNER model from HuggingFace")?;
    tracing::info!("GLiNER model downloaded to {model_dir}");
    Ok(())
}

pub fn ensure_chinese_ner_model(model_dir: &str) -> Result<()> {
    let dir = Path::new(model_dir);
    if dir.join("model.onnx").exists() {
        return Ok(());
    }
    tracing::info!("Chinese NER model not found at {model_dir}, downloading from HuggingFace...");
    download_repo(CHINESE_NER_REPO, CHINESE_NER_FILES, dir)
        .context("downloading Chinese NER model from HuggingFace")?;
    tracing::info!("Chinese NER model downloaded to {model_dir}");
    Ok(())
}

fn download_repo(repo_id: &str, files: &[(&str, &str)], dest: &Path) -> Result<()> {
    let api = Api::new().context("initializing HuggingFace API")?;
    let repo = api.model(repo_id.to_string());

    for &(remote, local) in files {
        tracing::info!("  downloading {remote}...");
        let cached_path = repo
            .get(remote)
            .with_context(|| format!("downloading {remote} from {repo_id}"))?;
        let target = dest.join(local);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating directory {}", parent.display()))?;
        }
        symlink_or_copy(&cached_path, &target)
            .with_context(|| format!("linking {} → {}", cached_path.display(), target.display()))?;
    }
    Ok(())
}

fn symlink_or_copy(src: &Path, dest: &Path) -> Result<()> {
    if dest.exists() {
        return Ok(());
    }
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(src, dest)?;
        Ok(())
    }
    #[cfg(not(unix))]
    {
        std::fs::copy(src, dest)?;
        Ok(())
    }
}
