---
title: "How PII Engineer Detects Personal Data Across 13+ Languages"
date: "2026-05"
tag: "Architecture"
description: "A deep dive into the GLiNER2 NER engine, the 8-stage post-processing pipeline, and how we achieve 90%+ F1 across 9 PII types without a GPU."
---

## The Challenge

Detecting personally identifiable information (PII) in multilingual text is harder than it looks. A person's name in English follows different patterns than one in Chinese, Malay, or Vietnamese. Government IDs vary by country — Singapore's NRIC, Indonesia's NIK, India's Aadhaar, and Vietnam's CCCD all have different formats. Phone numbers, addresses, and dates are written differently across cultures.

Most PII detection tools are English-first, with other languages bolted on. We needed something that treats all 13+ languages as first-class citizens.

## Architecture Overview

PII Engineer uses a two-model approach:

1. **GLiNER2 Multi-NER** — a span-based NER model built on mDeBERTa-v3-base (~280M parameters), fine-tuned with LoRA for PII detection across all supported languages
2. **Chinese NER** — a BERT-based token classifier with BIO tagging, specifically trained for Chinese text where CJK characters require different tokenization

The system detects CJK characters in the input and automatically routes to both models when Chinese text is present. For non-CJK text, only the GLiNER2 model runs.

## GLiNER2: Span-Based NER

Unlike traditional sequence labeling (BIO tagging), GLiNER2 uses a span-based approach. Instead of classifying each token, it evaluates candidate spans of text and scores them against entity type embeddings.

The model consists of 5 ONNX components:

| Component | Size | Role |
|-----------|------|------|
| encoder | 511MB (INT8) | mDeBERTa-v3-base token encoder |
| span_rep | 63MB | Span representation layer |
| count_embed | 41MB | Count embedding for span scoring |
| count_pred | 4.6MB | Count prediction head |
| classifier | 4.5MB | Final classification head |

We ship an INT8 quantized encoder (511MB vs 1.1GB FP32), which gives ~15-20% faster inference on x86 CPUs with negligible accuracy loss.

## The 8-Stage Pipeline

Raw NER output is noisy. The model might flag "Dr." as a person name, detect an invalid phone number, or return overlapping entities. Our post-processing pipeline cleans this up:

### 1. Reclassify

Chinese phone numbers often appear near markers like 电话 (phone) or 手机 (mobile). If a generic entity appears near these markers, we reclassify it as `phone_number`.

### 2. Validate

Each entity type has format validation. Phone numbers must contain enough digits and no letters. Government IDs need a minimum length with alphanumeric characters. Passport numbers can't be digits-only. Invalid formats are rejected.

### 3. Filter

We maintain a vocabulary of meta-words — pronouns ("I", "you", "she"), family terms ("mom", "husband"), medical terms ("doctor", "patient"), and label words themselves ("name", "phone"). Entities matching these words are filtered out.

### 4. Normalize

Context prefixes like "Patient", "Dr.", "Mr." are stripped from person names. "Patient Sarah Lim" becomes "Sarah Lim".

### 5. Email/IP Detection

Regex-based detection for email addresses and IPv4 addresses. These patterns are highly structured and regex catches them more reliably than NER.

### 6. Threshold

Per-label confidence thresholds. Person names use a lower threshold (0.25) because they're diverse, while phone numbers use a higher one (0.30) because the model is more confident on structured patterns.

### 7. Dedup

Overlapping entities with the same label are deduplicated — the longer span wins. Overlapping entities with different labels are both kept.

### 8. Merge

Adjacent entities with the same label separated by 3 or fewer characters are merged. "John" + " " + "Doe" becomes "John Doe".

## Performance

| Label | Precision | Recall | F1 |
|-------|-----------|--------|----|
| person_name | 0.808 | 0.838 | 0.823 |
| phone_number | 0.962 | 0.975 | 0.968 |
| government_id | 0.902 | 0.938 | 0.920 |
| street_address | 0.903 | 0.891 | 0.897 |
| date_of_birth | 0.901 | 0.901 | 0.901 |
| email_address | 0.974 | 0.966 | 0.970 |
| passport_number | 0.808 | 0.812 | 0.810 |
| license_plate | 0.837 | 0.847 | 0.842 |
| bank_account_number | 0.879 | 0.906 | 0.892 |
| **Mean** | | | **0.902** |

## Latency

On a MacBook M-series with FP32 encoder, typical inference is ~150ms per request. On a 4-vCPU Xeon with INT8 encoder, ~250ms. The entire pipeline runs on CPU — no GPU required.

The server auto-downloads models from HuggingFace on first run, warms up the ONNX sessions, and locks model weights in RAM to prevent swap.

## Try It

PII Engineer is open source under Apache-2.0. Get started in one command:

```
cargo build --release --package pii-engineer-server
cargo run --release --package pii-engineer-server
# Models download automatically — http://localhost:8000
```

Source code: [github.com/gantz-ai/pii.engineer](https://github.com/gantz-ai/pii.engineer)

Models: [huggingface.co/pii-engineer](https://huggingface.co/pii-engineer)
