//! GLiNER2 full: 5 ONNX models — NER, classification, structured extraction, relations.

use std::collections::HashMap;

use ndarray::Array2;
use ort::value::Value;

use crate::error::{Error, Result};
use super::{
    ClassificationResult, Entity, GlinerSpanModel, ModelBackend, SchemaMarker, StructuredResult,
};
use super::shared::sigmoid;

impl GlinerSpanModel {
    // ── NER ─────────────────────────────────────────────────────────────────

    pub(super) fn detect_v2_full(
        &self,
        text: &str,
        labels: &[String],
        descriptions: Option<&[(&str, &str)]>,
    ) -> Result<Vec<Entity>> {
        let ModelBackend::V2Full {
            encoder, span_rep, count_embed, count_embed_ner, hidden_size, ..
        } = &self.backend else { unreachable!() };
        let ce_model = count_embed_ner.as_ref().unwrap_or(count_embed);
        let hidden_size = *hidden_size;
        let t0 = std::time::Instant::now();

        let (token_embs_data, encoding, prompt_len, _, field_word_indices, words) =
            if let Some(descs) = descriptions {
                self.encode_with_descriptions(text, "entities", labels, SchemaMarker::E, encoder, descs)?
            } else {
                self.encode_with_schema(text, "entities", labels, SchemaMarker::E, encoder)?
            };
        let t_enc = t0.elapsed().as_millis();

        let num_words = words.len();
        let num_labels = labels.len();

        let word_embs = self.pool_word_embs(&encoding, prompt_len, &token_embs_data, hidden_size, num_words);
        let field_embs = self.extract_schema_embs(
            &encoding, &field_word_indices, &token_embs_data, hidden_size, num_labels,
        );

        let t1 = std::time::Instant::now();
        let (span_result, ce_result) = std::thread::scope(|s| {
            let span_handle = s.spawn(|| self.run_span_rep(span_rep, &word_embs, num_words, hidden_size));
            let ce_handle = s.spawn(|| -> Result<Vec<f32>> {
                let field_embs_arr = Array2::from_shape_vec((num_labels, hidden_size), field_embs)
                    .map_err(|e| Error::Model(format!("{e}")))?;
                let ce_out = ce_model.run(ort::inputs![
                    "field_embs" => Value::from_array(field_embs_arr)?,
                ]?)?;
                let struct_proj = ce_out[0].try_extract_tensor::<f32>()?;
                Ok(struct_proj.as_slice()
                    .ok_or_else(|| Error::Model("struct_proj not contiguous".into()))?
                    .to_vec())
            });
            (span_handle.join().unwrap(), ce_handle.join().unwrap())
        });
        let span_reps_data = span_result?;
        let struct_proj_data = ce_result?;
        let t_par = t1.elapsed().as_millis();

        let t3 = std::time::Instant::now();
        let mut scores = vec![0.0f32; num_words * self.max_width * num_labels];
        for s in 0..num_words {
            for w in 0..self.max_width {
                let word_end = s + w;
                if word_end >= num_words { break; }
                for f in 0..num_labels {
                    let span_offset = (s * self.max_width + w) * hidden_size;
                    let proj_offset = f * hidden_size;
                    let dot: f32 = (0..hidden_size)
                        .map(|d| span_reps_data[span_offset + d] * struct_proj_data[proj_offset + d])
                        .sum();
                    scores[(s * self.max_width + w) * num_labels + f] = sigmoid(dot);
                }
            }
        }

        let entities = self.decode_spans(&scores, num_words, num_labels, labels, &words, text, false);
        let t_score = t3.elapsed().as_millis();

        tracing::info!(
            enc_ms = t_enc, span_cembed_ms = t_par,
            score_ms = t_score, total_ms = t0.elapsed().as_millis(),
            words = num_words, "gliner::detect_v2_full"
        );
        Ok(entities)
    }

    // ── Classification ──────────────────────────────────────────────────────

    pub(super) fn classify_impl(
        &self,
        text: &str,
        labels: &[String],
        multi_label: bool,
    ) -> Result<Vec<ClassificationResult>> {
        let ModelBackend::V2Full { encoder, classifier, hidden_size, .. } = &self.backend else {
            return Err(Error::Model("classify requires v2-full backend".into()));
        };
        let hidden_size = *hidden_size;
        let t0 = std::time::Instant::now();

        let words = self.word_split(text);
        let word_strs: Vec<&str> = words.iter().map(|w| w.text.as_str()).collect();

        let mut prompt_tokens: Vec<&str> = vec!["(", SchemaMarker::P, "classification", "("];
        let mut label_word_indices: Vec<usize> = Vec::new();
        for label in labels {
            prompt_tokens.push(SchemaMarker::L);
            label_word_indices.push(prompt_tokens.len() - 1);
            prompt_tokens.push(label.as_str());
        }
        prompt_tokens.extend_from_slice(&[")", ")", SchemaMarker::SEP_TEXT]);

        let mut all_tokens: Vec<&str> = prompt_tokens;
        all_tokens.extend_from_slice(&word_strs);

        let encoding = self.tokenizer.encode(all_tokens, true)
            .map_err(|e| Error::Tokenizer(format!("{e}")))?;
        let (input_ids, attention_mask, seq_len) = Self::encoding_to_arrays(&encoding);

        let input_ids_arr = Array2::from_shape_vec((1, seq_len), input_ids)
            .map_err(|e| Error::Model(format!("{e}")))?;
        let attention_mask_arr = Array2::from_shape_vec((1, seq_len), attention_mask)
            .map_err(|e| Error::Model(format!("{e}")))?;

        let enc_out = encoder.run(ort::inputs![
            "input_ids" => Value::from_array(input_ids_arr)?,
            "attention_mask" => Value::from_array(attention_mask_arr)?,
        ]?)?;

        let token_embs = enc_out[0].try_extract_tensor::<f32>()?;
        let token_embs_slice = token_embs.as_slice()
            .ok_or_else(|| Error::Model("token_embs not contiguous".into()))?;

        let label_embs = self.extract_schema_embs(
            &encoding, &label_word_indices, token_embs_slice, hidden_size, labels.len(),
        );

        let label_embs_arr = Array2::from_shape_vec((labels.len(), hidden_size), label_embs)
            .map_err(|e| Error::Model(format!("{e}")))?;

        let clf_out = classifier.run(ort::inputs![
            "label_embs" => Value::from_array(label_embs_arr)?,
        ]?)?;

        let logits_view = clf_out[0].try_extract_tensor::<f32>()?;
        let logits = logits_view.as_slice()
            .ok_or_else(|| Error::Model("logits not contiguous".into()))?;

        let results = if multi_label {
            logits.iter().enumerate()
                .map(|(i, &v)| ClassificationResult {
                    label: labels[i].clone(),
                    score: sigmoid(v),
                })
                .filter(|r| r.score > 0.5)
                .collect()
        } else {
            let max_val = logits.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
            let exps: Vec<f32> = logits.iter().map(|&v| (v - max_val).exp()).collect();
            let sum: f32 = exps.iter().sum();
            let mut results: Vec<ClassificationResult> = exps.iter().enumerate()
                .map(|(i, &e)| ClassificationResult {
                    label: labels[i].clone(),
                    score: e / sum,
                })
                .collect();
            results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
            results
        };

        tracing::info!(total_ms = t0.elapsed().as_millis(), "gliner::classify");
        Ok(results)
    }

    // ── Structured extraction / Relations ────────────────────────────────────

    pub(super) fn extract_span_task(
        &self,
        text: &str,
        schema_name: &str,
        fields: &[String],
        threshold: f32,
        field_marker: &str,
    ) -> Result<StructuredResult> {
        let ModelBackend::V2Full {
            encoder, span_rep, count_pred, count_embed, hidden_size, ..
        } = &self.backend else {
            return Err(Error::Model("extract_structured requires v2-full backend".into()));
        };
        let hidden_size = *hidden_size;
        let t0 = std::time::Instant::now();

        let (token_embs_data, encoding, prompt_len, p_word_idx, field_word_indices, words) =
            self.encode_with_schema(text, schema_name, fields, field_marker, encoder)?;

        let num_words = words.len();
        let num_fields = fields.len();

        let word_embs = self.pool_word_embs(&encoding, prompt_len, &token_embs_data, hidden_size, num_words);
        let p_emb = self.extract_single_emb(&encoding, p_word_idx, &token_embs_data, hidden_size);
        let field_embs = self.extract_schema_embs(
            &encoding, &field_word_indices, &token_embs_data, hidden_size, num_fields,
        );

        // count_pred: [1, D] → [1, 20]
        let p_emb_arr = Array2::from_shape_vec((1, hidden_size), p_emb)
            .map_err(|e| Error::Model(format!("{e}")))?;
        let cp_out = count_pred.run(ort::inputs![
            "p_embedding" => Value::from_array(p_emb_arr)?,
        ]?)?;
        let count_logits = cp_out[0].try_extract_tensor::<f32>()?;
        let count_data = count_logits.as_slice()
            .ok_or_else(|| Error::Model("count not contiguous".into()))?;
        let pred_count = count_data.iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .map(|(i, _)| i)
            .unwrap_or(1)
            .clamp(1, 20);

        let span_reps_data = self.run_span_rep(span_rep, &word_embs, num_words, hidden_size)?;

        // count_embed: [num_fields, D] → [20, num_fields, D]
        let field_embs_arr = Array2::from_shape_vec((num_fields, hidden_size), field_embs)
            .map_err(|e| Error::Model(format!("{e}")))?;
        let ce_out = count_embed.run(ort::inputs![
            "field_embs" => Value::from_array(field_embs_arr)?,
        ]?)?;
        let struct_proj = ce_out[0].try_extract_tensor::<f32>()?;
        let struct_proj_data = struct_proj.as_slice()
            .ok_or_else(|| Error::Model("struct_proj not contiguous".into()))?;

        // Score: einsum per count instance
        let mut instances: Vec<HashMap<String, Vec<Entity>>> = Vec::new();

        for c in 0..pred_count {
            let mut field_entities: HashMap<String, Vec<Entity>> = HashMap::new();
            for (f, field) in fields.iter().enumerate() {
                let proj_offset = (c * num_fields + f) * hidden_size;
                let proj_vec = &struct_proj_data[proj_offset..proj_offset + hidden_size];

                let mut best: Option<(usize, usize, f32)> = None;
                for s in 0..num_words {
                    for w in 0..self.max_width {
                        let word_end = s + w;
                        if word_end >= num_words { break; }
                        let span_offset = (s * self.max_width + w) * hidden_size;
                        let span_vec = &span_reps_data[span_offset..span_offset + hidden_size];
                        let dot: f32 = span_vec.iter().zip(proj_vec).map(|(a, b)| a * b).sum();
                        let score = sigmoid(dot);
                        if score >= threshold && best.map_or(true, |(_, _, bs)| score > bs) {
                            best = Some((s, word_end, score));
                        }
                    }
                }

                if let Some((ws, we, score)) = best {
                    let char_start = words[ws].start;
                    let char_end = words[we].end;
                    field_entities.entry(field.clone()).or_default().push(Entity {
                        start: char_start,
                        end: char_end,
                        text: text[char_start..char_end].to_string(),
                        label: field.clone(),
                        score,
                    });
                }
            }
            if !field_entities.is_empty() {
                instances.push(field_entities);
            }
        }

        tracing::info!(count = pred_count, total_ms = t0.elapsed().as_millis(), "gliner::extract_structured");
        Ok(StructuredResult { instances })
    }
}
