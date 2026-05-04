//! GLiNER V1: single ONNX model with 6 inputs → logits.

use ndarray::{Array2, Array3};
use ort::value::Value;

use crate::error::{Error, Result};
use super::{Entity, GlinerSpanModel, ModelBackend};

impl GlinerSpanModel {
    pub(super) fn detect_v1(&self, text: &str, labels: &[String]) -> Result<Vec<Entity>> {
        let t0 = std::time::Instant::now();
        let words = self.word_split(text);
        if words.is_empty() { return Ok(vec![]); }

        let num_words = words.len();
        let word_strs: Vec<&str> = words.iter().map(|w| w.text.as_str()).collect();

        let mut prompt_words: Vec<&str> = Vec::new();
        for label in labels {
            prompt_words.push("<<ENT>>");
            prompt_words.push(label.as_str());
        }
        prompt_words.push("<<SEP>>");
        let prompt_len = prompt_words.len();

        let mut full_words: Vec<&str> = prompt_words;
        full_words.extend_from_slice(&word_strs);

        let encoding = self.tokenizer.encode(full_words, true)
            .map_err(|e| Error::Tokenizer(format!("{e}")))?;

        let (input_ids, attention_mask, seq_len) = Self::encoding_to_arrays(&encoding);
        let words_mask = self.build_words_mask(&encoding, prompt_len, seq_len);
        let (span_idx_flat, span_mask_flat) = self.build_spans_v1(num_words);
        let num_spans = num_words * self.max_width;

        let input_ids_arr = Array2::from_shape_vec((1, seq_len), input_ids).map_err(|e| Error::Model(format!("{e}")))?;
        let attention_mask_arr = Array2::from_shape_vec((1, seq_len), attention_mask).map_err(|e| Error::Model(format!("{e}")))?;
        let words_mask_arr = Array2::from_shape_vec((1, seq_len), words_mask).map_err(|e| Error::Model(format!("{e}")))?;
        let text_lengths_arr = Array2::from_shape_vec((1, 1), vec![num_words as i64]).map_err(|e| Error::Model(format!("{e}")))?;
        let span_idx_arr = Array3::from_shape_vec((1, num_spans, 2), span_idx_flat).map_err(|e| Error::Model(format!("{e}")))?;
        let span_mask_arr = Array2::from_shape_vec((1, num_spans), span_mask_flat).map_err(|e| Error::Model(format!("{e}")))?;

        let session = match &self.backend {
            ModelBackend::V1 { session } => session,
            _ => unreachable!(),
        };

        let outputs = session.run(ort::inputs![
            "input_ids" => Value::from_array(input_ids_arr)?,
            "attention_mask" => Value::from_array(attention_mask_arr)?,
            "words_mask" => Value::from_array(words_mask_arr)?,
            "text_lengths" => Value::from_array(text_lengths_arr)?,
            "span_idx" => Value::from_array(span_idx_arr)?,
            "span_mask" => Value::from_array(span_mask_arr)?,
        ]?)?;

        let logits_view = outputs[0].try_extract_tensor::<f32>()?;
        let shape = logits_view.shape();
        if shape.len() != 4 || shape[0] != 1 {
            return Err(Error::Model(format!("unexpected logits shape: {shape:?}")));
        }
        let logits_data = logits_view.as_slice()
            .ok_or_else(|| Error::Model("logits not contiguous".into()))?;

        let entities = self.decode_spans(logits_data, num_words, shape[3], labels, &words, text, true);
        tracing::info!(total_ms = t0.elapsed().as_millis(), "gliner::detect_v1");
        Ok(entities)
    }
}
