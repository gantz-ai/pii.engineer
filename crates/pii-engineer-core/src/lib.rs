//! Core inference + pipeline crate for PII Engineer.

pub mod chinese_ner;
pub mod error;
pub mod gliner;
pub mod labels;
pub mod lang;
pub mod pipeline;

pub use chinese_ner::ChineseNer;
pub use error::{Error, Result};
pub use gliner::{
    ClassificationResult, Entity, GlinerSpanModel, RelationInstance, RelationResult,
    StructuredResult,
};
pub use pipeline::{run as run_pipeline, PipelineConfig};
