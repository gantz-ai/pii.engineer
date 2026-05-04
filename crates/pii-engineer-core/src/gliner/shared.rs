//! Shared building blocks: tokenization, embedding extraction, span decoding.

use ndarray::{Array2, Array3};
use ort::session::Session;
use ort::value::Value;

use crate::error::{Error, Result};
use super::{Entity, GlinerSpanModel, SchemaMarker};

pub(super) type EncodeResult = (Vec<f32>, tokenizers::Encoding, usize, usize, Vec<usize>, Vec<WordSpan>);

// ── Word span ───────────────────────────────────────────────────────────────

pub(super) struct WordSpan {
    pub text: String,
    pub start: usize,
    pub end: usize,
}

pub(super) struct RawSpan {
    pub word_start: usize,
    pub word_end: usize,
    pub class_idx: usize,
    pub score: f32,
}

// ── Helpers ─────────────────────────────────────────────────────────────────

#[inline]
pub(super) fn sigmoid(x: f32) -> f32 {
    1.0 / (1.0 + (-x).exp())
}

pub(crate) fn ort_intra_threads() -> usize {
    std::env::var("ORT_INTRA_THREADS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(4)
}

pub(crate) fn ort_inter_threads() -> usize {
    std::env::var("ORT_INTER_THREADS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1)
}

// ── Impl on GlinerSpanModel ─────────────────────────────────────────────────

impl GlinerSpanModel {
    pub(super) fn encoding_to_arrays(encoding: &tokenizers::Encoding) -> (Vec<i64>, Vec<i64>, usize) {
        let input_ids: Vec<i64> = encoding.get_ids().iter().map(|&x| x as i64).collect();
        let attention_mask: Vec<i64> = encoding.get_attention_mask().iter().map(|&x| x as i64).collect();
        let seq_len = input_ids.len();
        (input_ids, attention_mask, seq_len)
    }

    pub(super) fn word_split(&self, text: &str) -> Vec<WordSpan> {
        self.word_re
            .find_iter(text)
            .map(|m| WordSpan { text: m.as_str().to_string(), start: m.start(), end: m.end() })
            .collect()
    }

    /// Build schema prompt, encode, run encoder, return raw embeddings + metadata.
    pub(super) fn encode_with_schema(
        &self,
        text: &str,
        schema_name: &str,
        fields: &[String],
        field_marker: &str,
        encoder: &Session,
    ) -> Result<EncodeResult> {
        self.encode_with_schema_inner(text, schema_name, fields, field_marker, encoder, None)
    }

    /// Build schema prompt with optional label descriptions.
    pub(super) fn encode_with_descriptions(
        &self,
        text: &str,
        schema_name: &str,
        fields: &[String],
        field_marker: &str,
        encoder: &Session,
        descriptions: &[(&str, &str)],
    ) -> Result<EncodeResult> {
        self.encode_with_schema_inner(text, schema_name, fields, field_marker, encoder, Some(descriptions))
    }

    fn encode_with_schema_inner(
        &self,
        text: &str,
        schema_name: &str,
        fields: &[String],
        field_marker: &str,
        encoder: &Session,
        descriptions: Option<&[(&str, &str)]>,
    ) -> Result<EncodeResult> {
        let words = self.word_split(text);
        if words.is_empty() {
            return Err(Error::Model("empty text".into()));
        }
        let word_strs_owned: Vec<String> = words.iter().map(|w| w.text.to_lowercase()).collect();
        let word_strs: Vec<&str> = word_strs_owned.iter().map(|s| s.as_str()).collect();

        // Build [P] prompt: "schema_name [DESCRIPTION] label: desc [DESCRIPTION] label: desc ..."
        let prompt_str_owned: String;
        let prompt_ref: &str = if let Some(descs) = descriptions {
            let mut s = schema_name.to_string();
            for (label, desc) in descs {
                if fields.iter().any(|f| f == label) {
                    s.push_str(" [DESCRIPTION] ");
                    s.push_str(label);
                    s.push_str(": ");
                    s.push_str(desc);
                }
            }
            prompt_str_owned = s;
            &prompt_str_owned
        } else {
            schema_name
        };

        // ( [P] prompt ( [E/C/R] field1 [E/C/R] field2 ) ) [SEP_TEXT] word1 word2 ...
        let mut prompt_tokens: Vec<&str> = vec!["(", SchemaMarker::P, prompt_ref, "("];
        let p_word_idx = 1;
        let mut field_word_indices: Vec<usize> = Vec::new();
        for field in fields {
            prompt_tokens.push(field_marker);
            field_word_indices.push(prompt_tokens.len() - 1);
            prompt_tokens.push(field.as_str());
        }
        prompt_tokens.extend_from_slice(&[")", ")", SchemaMarker::SEP_TEXT]);
        let prompt_len = prompt_tokens.len();

        let mut all_tokens: Vec<&str> = prompt_tokens;
        all_tokens.extend_from_slice(&word_strs);

        let encoding = self.tokenizer.encode(all_tokens, false)
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
        let token_embs_data = token_embs.as_slice()
            .ok_or_else(|| Error::Model("token_embs not contiguous".into()))?
            .to_vec();

        Ok((token_embs_data, encoding, prompt_len, p_word_idx, field_word_indices, words))
    }

    /// First-subword pooling for text words (after prompt).
    pub(super) fn pool_word_embs(
        &self,
        encoding: &tokenizers::Encoding,
        prompt_len: usize,
        token_embs: &[f32],
        hidden_size: usize,
        num_words: usize,
    ) -> Vec<f32> {
        let word_ids = encoding.get_word_ids();
        let mut embs = vec![0.0f32; num_words * hidden_size];
        let mut filled = vec![false; num_words];
        let mut prev_wid: Option<u32> = None;
        let mut seen: usize = 0;

        for (tok_idx, &wid_opt) in word_ids.iter().enumerate() {
            if let Some(wid) = wid_opt {
                if Some(wid) != prev_wid {
                    seen += 1;
                    if seen > prompt_len {
                        let wi = seen - prompt_len - 1;
                        if wi < num_words && !filled[wi] {
                            let src = tok_idx * hidden_size;
                            let dst = wi * hidden_size;
                            embs[dst..dst + hidden_size].copy_from_slice(&token_embs[src..src + hidden_size]);
                            filled[wi] = true;
                        }
                    }
                }
                prev_wid = Some(wid);
            }
        }
        embs
    }

    /// Extract embeddings for schema marker tokens (first subword).
    pub(super) fn extract_schema_embs(
        &self,
        encoding: &tokenizers::Encoding,
        schema_word_indices: &[usize],
        token_embs: &[f32],
        hidden_size: usize,
        num_fields: usize,
    ) -> Vec<f32> {
        let word_ids = encoding.get_word_ids();
        let mut embs = vec![0.0f32; num_fields * hidden_size];
        let mut filled = vec![false; num_fields];
        let mut prev_wid: Option<u32> = None;
        let mut seen: usize = 0;

        for (tok_idx, &wid_opt) in word_ids.iter().enumerate() {
            if let Some(wid) = wid_opt {
                if Some(wid) != prev_wid {
                    seen += 1;
                    for (fi, &swi) in schema_word_indices.iter().enumerate() {
                        if seen == swi + 1 && !filled[fi] {
                            let src = tok_idx * hidden_size;
                            let dst = fi * hidden_size;
                            embs[dst..dst + hidden_size].copy_from_slice(&token_embs[src..src + hidden_size]);
                            filled[fi] = true;
                        }
                    }
                }
                prev_wid = Some(wid);
            }
        }
        embs
    }

    /// Extract a single embedding at a specific prompt word index.
    pub(super) fn extract_single_emb(
        &self,
        encoding: &tokenizers::Encoding,
        word_idx: usize,
        token_embs: &[f32],
        hidden_size: usize,
    ) -> Vec<f32> {
        let word_ids = encoding.get_word_ids();
        let mut prev_wid: Option<u32> = None;
        let mut seen: usize = 0;

        for (tok_idx, &wid_opt) in word_ids.iter().enumerate() {
            if let Some(wid) = wid_opt {
                if Some(wid) != prev_wid {
                    seen += 1;
                    if seen == word_idx + 1 {
                        let src = tok_idx * hidden_size;
                        return token_embs[src..src + hidden_size].to_vec();
                    }
                }
                prev_wid = Some(wid);
            }
        }
        vec![0.0f32; hidden_size]
    }

    /// Run span_rep ONNX: word_embs → span_reps.
    pub(super) fn run_span_rep(
        &self,
        span_rep: &Session,
        word_embs: &[f32],
        num_words: usize,
        hidden_size: usize,
    ) -> Result<Vec<f32>> {
        let num_spans = num_words * self.max_width;
        let mut span_idx_flat: Vec<i64> = Vec::with_capacity(num_spans * 2);
        for start in 0..num_words {
            for w in 0..self.max_width {
                let end = start + w;
                span_idx_flat.push(start as i64);
                span_idx_flat.push(end.min(num_words - 1) as i64);
            }
        }

        let word_embs_arr = Array3::from_shape_vec((1, num_words, hidden_size), word_embs.to_vec())
            .map_err(|e| Error::Model(format!("{e}")))?;
        let span_idx_arr = Array3::from_shape_vec((1, num_spans, 2), span_idx_flat)
            .map_err(|e| Error::Model(format!("{e}")))?;

        let sr_out = span_rep.run(ort::inputs![
            "word_embs" => Value::from_array(word_embs_arr)?,
            "span_idx" => Value::from_array(span_idx_arr)?,
        ]?)?;
        let span_reps = sr_out[0].try_extract_tensor::<f32>()?;
        let data = span_reps.as_slice()
            .ok_or_else(|| Error::Model("span_reps not contiguous".into()))?
            .to_vec();
        Ok(data)
    }

    /// Decode span scores → entities with greedy non-overlapping selection.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn decode_spans(
        &self,
        scores_data: &[f32],
        num_words: usize,
        num_classes: usize,
        labels: &[String],
        words: &[WordSpan],
        text: &str,
        apply_sigmoid: bool,
    ) -> Vec<Entity> {
        let threshold = 0.01_f32;
        let mut candidates: Vec<RawSpan> = Vec::new();

        for s in 0..num_words {
            for k in 0..self.max_width {
                let word_end = s + k;
                if word_end >= num_words { break; }
                for c in 0..num_classes {
                    let idx = (s * self.max_width + k) * num_classes + c;
                    let score = if apply_sigmoid { sigmoid(scores_data[idx]) } else { scores_data[idx] };
                    if score > threshold {
                        candidates.push(RawSpan { word_start: s, word_end, class_idx: c, score });
                    }
                }
            }
        }

        self.greedy_select(candidates, labels, words, text)
    }

    fn greedy_select(&self, mut candidates: Vec<RawSpan>, labels: &[String], words: &[WordSpan], text: &str) -> Vec<Entity> {
        candidates.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        let mut selected: Vec<RawSpan> = Vec::new();
        for cand in candidates {
            let overlaps = selected.iter().any(|sel| cand.word_start <= sel.word_end && cand.word_end >= sel.word_start);
            if !overlaps {
                selected.push(cand);
            }
        }
        selected.sort_by_key(|s| s.word_start);
        selected.into_iter()
            .filter_map(|span| {
                if span.class_idx >= labels.len() { return None; }
                let char_start = words[span.word_start].start;
                let char_end = words[span.word_end].end;
                Some(Entity {
                    start: char_start,
                    end: char_end,
                    text: text[char_start..char_end].to_string(),
                    label: labels[span.class_idx].to_string(),
                    score: span.score,
                })
            })
            .collect()
    }

    /// Build words_mask for V1 inference.
    pub(super) fn build_words_mask(&self, encoding: &tokenizers::Encoding, prompt_len: usize, seq_len: usize) -> Vec<i64> {
        let word_ids = encoding.get_word_ids();
        let mut mask: Vec<i64> = Vec::with_capacity(seq_len);
        let mut prev_wid: Option<u32> = None;
        let mut seen: usize = 0;

        for &wid_opt in word_ids {
            match wid_opt {
                None => mask.push(0),
                Some(wid) => {
                    if Some(wid) != prev_wid {
                        seen += 1;
                        if seen <= prompt_len { mask.push(0); } else { mask.push((seen - prompt_len) as i64); }
                    } else {
                        mask.push(0);
                    }
                    prev_wid = Some(wid);
                }
            }
        }
        mask
    }

    /// Build span indices and mask for V1 inference.
    pub(super) fn build_spans_v1(&self, num_words: usize) -> (Vec<i64>, Vec<bool>) {
        let num_spans = num_words * self.max_width;
        let mut idx: Vec<i64> = Vec::with_capacity(num_spans * 2);
        let mut mask: Vec<bool> = Vec::with_capacity(num_spans);
        for start in 0..num_words {
            for w in 0..self.max_width {
                let end = start + w;
                idx.push(start as i64);
                idx.push(end as i64);
                mask.push(end < num_words);
            }
        }
        (idx, mask)
    }
}
