---
title: "Benchmarking PII Detection: Precision vs Recall Across 9 Entity Types"
date: "2026-05"
tag: "Benchmarks"
description: "Our evaluation results on real-world multilingual text — what works, where the model struggles, and how the post-processing pipeline improves raw NER output."
---

## Evaluation Setup

We evaluate PII Engineer on a held-out test set of 1,200 annotated samples spanning all 13 supported languages. Each sample contains 1-5 PII entities across the 9 supported types. The test set is balanced across languages and entity types to avoid skew.

Metrics use strict span matching — both the entity boundaries and the label must be correct for a true positive. Partial overlaps count as false positives.

## Overall Results

| Label | Precision | Recall | F1 | Support |
|-------|-----------|--------|----|---------|
| person_name | 0.808 | 0.838 | 0.823 | 412 |
| phone_number | 0.962 | 0.975 | 0.968 | 198 |
| government_id | 0.902 | 0.938 | 0.920 | 187 |
| street_address | 0.903 | 0.891 | 0.897 | 156 |
| date_of_birth | 0.901 | 0.901 | 0.901 | 134 |
| email_address | 0.974 | 0.966 | 0.970 | 89 |
| passport_number | 0.808 | 0.812 | 0.810 | 72 |
| license_plate | 0.837 | 0.847 | 0.842 | 64 |
| bank_account_number | 0.879 | 0.906 | 0.892 | 88 |
| **Macro Average** | **0.886** | **0.897** | **0.902** | **1400** |

## Performance by Language

The model performs consistently across primary languages, with slightly lower scores on secondary languages that had less training data:

| Language | F1 | Notes |
|----------|----|-------|
| English | 0.931 | Highest overall — most training data available |
| Chinese | 0.918 | Dual-model approach (GLiNER2 + Chinese NER) helps significantly |
| Vietnamese | 0.912 | Strong on names and CCCD numbers |
| Malay | 0.904 | Good coverage of MyKad format |
| Indonesian | 0.897 | NIK detection reliable, addresses slightly lower |
| Tamil | 0.871 | Script complexity affects span boundaries |
| Thai | 0.856 | No word boundaries — tokenization challenges |
| Hindi | 0.849 | Devanagari names sometimes split incorrectly |
| Korean | 0.862 | Good on structured data, weaker on names |

## What Works Well

### Structured patterns (F1 > 0.95)

**Phone numbers** and **email addresses** are highly structured. The combination of NER detection + regex validation in the post-processing pipeline achieves near-perfect results. The model learns the context ("call me at", "email:", "电话") and the pipeline validates the format.

### Government IDs (F1 > 0.92)

Country-specific ID formats (NRIC, NIK, CCCD, Aadhaar) have distinctive patterns. The model picks up contextual cues ("IC number", "CCCD số", "NIK") and the validation stage checks format compliance.

## Where the Model Struggles

### Person names (F1 = 0.823)

Names are the hardest entity type. Common failure modes:

- **Boundary errors:** "Dr. Sarah Lim" detected as full span vs "Sarah Lim" only. The normalize stage handles common prefixes, but unusual titles or honorifics may remain.
- **Common words as names:** "Joy" (emotion vs name), "Will" (modal verb vs name), "May" (month vs name). Context usually disambiguates, but short sentences lack signal.
- **Transliterated names:** The same Chinese name can be romanized multiple ways (Xiao Ming / Siau Beng / Tiểu Minh). The model handles this through multilingual training data.

### Passport numbers (F1 = 0.810)

Passport numbers look like random alphanumeric strings — easy to confuse with reference numbers, order IDs, or serial numbers. The model relies heavily on context ("passport", "travel document") and the validation stage rejects digits-only candidates.

### License plates (F1 = 0.842)

Plate formats vary dramatically by country (SG: SBA1234A, MY: WKN5678, VN: 51A-12345, ID: B1234ABC). The model handles most formats but occasionally misclassifies short alphanumeric codes as plates.

## Pipeline Impact

The 8-stage post-processing pipeline significantly improves raw NER output. Here's the effect of each stage measured as delta F1 from the raw model output:

| Stage | ΔF1 | Primary Effect |
|-------|-----|----------------|
| Reclassify | +0.008 | Fixes Chinese phone numbers misclassified as generic entities |
| Validate | +0.031 | Removes invalid formats (biggest precision boost) |
| Filter | +0.024 | Removes pronouns, medical terms falsely flagged as names |
| Normalize | +0.012 | Strips prefixes, improving boundary accuracy |
| Email/IP Regex | +0.018 | Catches emails/IPs the NER model missed |
| Threshold | +0.015 | Per-type confidence filtering reduces noise |
| Dedup | +0.006 | Removes redundant overlapping spans |
| Merge | +0.009 | Joins split names and addresses |
| **Total** | **+0.123** | Raw model F1: 0.779 → Final F1: 0.902 |

The pipeline adds ~12 points of F1. Validation and filtering contribute the most — they remove confident-but-wrong predictions that hurt precision.

## INT8 vs FP32 Accuracy

We ship an INT8 quantized encoder (511MB vs 1.1GB FP32). The accuracy impact is minimal:

| Model | F1 | Latency (M-series) | Latency (Xeon 4-core) |
|-------|----|--------------------|-----------------------|
| FP32 encoder | 0.904 | ~150ms | ~350ms |
| INT8 encoder | 0.902 | ~150ms | ~250ms |

INT8 gives a significant speed boost on x86 CPUs (which have native INT8 VNNI instructions) with only 0.002 F1 loss. On ARM (Apple Silicon), the difference is negligible in both speed and accuracy since ARM lacks dedicated INT8 acceleration.

## Comparison with Alternatives

We compared against common PII detection approaches on our multilingual test set:

| Approach | F1 (EN) | F1 (Multi) | Latency | GPU Required |
|----------|---------|------------|---------|--------------|
| Regex-only | 0.62 | 0.41 | <5ms | No |
| spaCy NER | 0.78 | 0.54 | ~50ms | No |
| Presidio (Microsoft) | 0.82 | 0.61 | ~100ms | No |
| GPT-4 (prompted) | 0.91 | 0.85 | ~2000ms | Cloud API |
| **PII Engineer** | **0.93** | **0.90** | **~150ms** | **No** |

PII Engineer achieves GPT-4-level accuracy at 13x lower latency, runs fully on-premise, and doesn't require sending sensitive data to an external API.

## Failure Analysis

We categorized the remaining errors (F1 gap from 1.0):

- **38% boundary errors** — entity detected but span too long or too short
- **27% false negatives** — entity missed entirely (usually low-confidence names in ambiguous context)
- **21% false positives** — non-PII flagged as PII (product names as person names, order numbers as IDs)
- **14% label confusion** — entity detected but wrong type (passport_number vs government_id)

Boundary errors are the largest category and hardest to fix — they require the model to learn more precise span boundaries, which we're addressing in v2.2 training with improved annotation guidelines.

## Reproducing These Results

The evaluation script and test set annotations are available in the repository. To reproduce:

```
cargo build --release --package pii-engineer-server
cargo run --release --package pii-engineer-server &
# Wait for model download and warmup
python eval/run_benchmark.py --test-set eval/test_multilingual.jsonl
```

Source: [github.com/gantz-ai/pii.engineer](https://github.com/gantz-ai/pii.engineer)
