//! GLiNER2 compat: encoder + scorer (NER only, from old 2-model export).

use ndarray::{Array2, Array3};
use ort::value::Value;

use crate::error::{Error, Result};
use super::{Entity, GlinerSpanModel, ModelBackend, SchemaMarker};

impl GlinerSpanModel {
    pub(super) fn detect_v2_compat(&self, text: &str, labels: &[String]) -> Result<Vec<Entity>> {
        let t0 = std::time::Instant::now();
        let words = self.word_split(text);
        if words.is_empty() { return Ok(vec![]); }

        let num_words = words.len();
        let num_labels = labels.len();
        let word_strs_owned: Vec<String> = words.iter().map(|w| w.text.to_lowercase()).collect();
        let word_strs: Vec<&str> = word_strs_owned.iter().map(|s| s.as_str()).collect();

        let (encoder, scorer, hidden_size) = match &self.backend {
            ModelBackend::V2Compat { encoder, scorer, hidden_size } => (encoder, scorer, *hidden_size),
            _ => unreachable!(),
        };

        let mut prompt_words: Vec<&str> = Vec::new();
        let mut schema_word_indices: Vec<usize> = Vec::new();
        for label in labels {
            prompt_words.push(SchemaMarker::E);
            prompt_words.push(label.as_str());
            schema_word_indices.push(prompt_words.len() - 1);
        }
        prompt_words.push("[SEP]");
        let prompt_len = prompt_words.len();

        let mut full_words: Vec<&str> = prompt_words;
        full_words.extend_from_slice(&word_strs);

        let encoding = self.tokenizer.encode(full_words, false)
            .map_err(|e| Error::Tokenizer(format!("{e}")))?;
        let (input_ids, attention_mask, seq_len) = Self::encoding_to_arrays(&encoding);

        let input_ids_arr = Array2::from_shape_vec((1, seq_len), input_ids).map_err(|e| Error::Model(format!("{e}")))?;
        let attention_mask_arr = Array2::from_shape_vec((1, seq_len), attention_mask).map_err(|e| Error::Model(format!("{e}")))?;

        let enc_out = encoder.run(ort::inputs![
            "input_ids" => Value::from_array(input_ids_arr)?,
            "attention_mask" => Value::from_array(attention_mask_arr)?,
        ]?)?;
        let token_embs = enc_out[0].try_extract_tensor::<f32>()?;
        let token_embs_slice = token_embs.as_slice()
            .ok_or_else(|| Error::Model("token_embs not contiguous".into()))?;

        let word_embs = self.pool_word_embs(&encoding, prompt_len, token_embs_slice, hidden_size, num_words);
        let schema_embs = self.extract_schema_embs(
            &encoding, &schema_word_indices, token_embs_slice, hidden_size, num_labels,
        );

        let num_spans = num_words * self.max_width;
        let mut span_idx_flat: Vec<i64> = Vec::with_capacity(num_spans * 2);
        let mut span_mask_flat: Vec<f32> = Vec::with_capacity(num_words * self.max_width);
        for start in 0..num_words {
            for w in 0..self.max_width {
                let end = start + w;
                if end < num_words {
                    span_idx_flat.push(start as i64);
                    span_idx_flat.push(end as i64);
                    span_mask_flat.push(1.0);
                } else {
                    span_idx_flat.push(0);
                    span_idx_flat.push(0);
                    span_mask_flat.push(0.0);
                }
            }
        }

        let word_embs_arr = Array3::from_shape_vec((1, num_words, hidden_size), word_embs).map_err(|e| Error::Model(format!("{e}")))?;
        let schema_embs_arr = Array3::from_shape_vec((1, num_labels, hidden_size), schema_embs).map_err(|e| Error::Model(format!("{e}")))?;
        let span_idx_arr = Array3::from_shape_vec((1, num_spans, 2), span_idx_flat).map_err(|e| Error::Model(format!("{e}")))?;
        let span_mask_arr = Array3::from_shape_vec((1, num_words, self.max_width), span_mask_flat).map_err(|e| Error::Model(format!("{e}")))?;

        let sc_out = scorer.run(ort::inputs![
            "word_embs" => Value::from_array(word_embs_arr)?,
            "schema_embs" => Value::from_array(schema_embs_arr)?,
            "span_idx" => Value::from_array(span_idx_arr)?,
            "span_mask" => Value::from_array(span_mask_arr)?,
        ]?)?;
        let scores_view = sc_out[0].try_extract_tensor::<f32>()?;
        let shape = scores_view.shape();
        if shape.len() != 4 || shape[0] != 1 {
            return Err(Error::Model(format!("unexpected scores shape: {shape:?}")));
        }
        let scores_data = scores_view.as_slice()
            .ok_or_else(|| Error::Model("scores not contiguous".into()))?;

        let entities = self.decode_spans(scores_data, num_words, num_labels, labels, &words, text, false);
        tracing::info!(total_ms = t0.elapsed().as_millis(), "gliner::detect_v2_compat");
        Ok(entities)
    }
}
