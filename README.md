# PII Engineer

[![License: AGPL-3.0](https://img.shields.io/badge/License-AGPL%20v3-blue.svg)](LICENSE)
[![CI](https://github.com/gantz-ai/pii.engineer/actions/workflows/ci.yml/badge.svg)](https://github.com/gantz-ai/pii.engineer/actions/workflows/ci.yml)

Fast, multilingual PII detection API for PDPA, PDPD, PDP Law, and PIPL compliance. Detects names, phones, IDs, addresses, and more across 13+ languages — single binary, CPU-only, no GPU required.

Built on a fine-tuned [GLiNER2](https://huggingface.co/fastino/gliner2-multi-v1) model (mDeBERTa-v3-base, ~280M params) with ONNX Runtime inference.

**[Live Demo](https://pii.engineer)** | **[Model on HuggingFace](https://huggingface.co/pii-engineer/PII-Engineer-Multi-NER-v2.1)**

## Quick Start

### 1. Build and run

```bash
cargo build --release --package pii-engineer-server
cargo run --release --package pii-engineer-server
# Models download automatically from HuggingFace on first run
# http://localhost:8000
```

Models are downloaded to `models/PII-Engineer-Multi-NER-v2.1` and `models/PII-Engineer-Chinese-NER-v1.0`. To download manually instead:

```bash
pip install huggingface_hub
huggingface-cli download pii-engineer/PII-Engineer-Multi-NER-v2.1 --local-dir models/PII-Engineer-Multi-NER-v2.1
huggingface-cli download pii-engineer/PII-Engineer-Chinese-NER-v1.0 --local-dir models/PII-Engineer-Chinese-NER-v1.0
```

### 3. Detect PII

```bash
curl -X POST http://localhost:8000/api/detect \
  -H "Content-Type: application/json" \
  -d '{"text": "John Doe, NRIC S9012345B, born 12 March 1985"}'
```

### Docker

```bash
docker build -t pii-engineer .

# Mount your model directory
docker run -p 8000:8000 -v ./models:/app/models pii-engineer
```

## PII Types

| Type | Examples |
|------|---------|
| `person_name` | Sarah Lim, Ahmad bin Abdullah, 陈伟明 |
| `phone_number` | +65 9123 4567, 0812-3456-7890 |
| `government_id` | S9012345B (NRIC), 3201010512890001 (NIK), 123456789012 (Aadhaar) |
| `street_address` | 42 Orchard Road #08-12, Jl. Sudirman No. 1 |
| `date_of_birth` | 12 March 1985, 1990-05-15 |
| `email_address` | john@example.com |
| `passport_number` | E12345678 |
| `license_plate` | SBA1234A, B 1234 CD |
| `bank_account_number` | 1234-5678-9012 |

## Supported Languages

**Primary:** English, Malay, Tamil, Chinese, Indonesian, Vietnamese

**Secondary:** Thai, Hindi, Bengali, Korean, German, French, Russian

## API

### `POST /api/detect`

**Request:**

```json
{
  "text": "Patient Sarah Lim, DOB 12 March 1985, NRIC S9012345B.",
  "labels": null,
  "boost": []
}
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `text` | string | required | Input text (max 50,000 chars) |
| `labels` | string[] | all 9 types | PII types to detect |
| `boost` | string[] | [] | Labels to boost with description matching |

**Response:**

```json
{
  "entities": [
    {
      "type": "person_name",
      "value": "Sarah Lim",
      "start": 8,
      "end": 17,
      "score": 0.96,
      "needs_review": false
    }
  ],
  "redacted": "Patient [PERSON_NAME], DOB [DATE_OF_BIRTH], NRIC [GOVERNMENT_ID].",
  "original": "Patient Sarah Lim, DOB 12 March 1985, NRIC S9012345B."
}
```

| Status | Meaning |
|--------|---------|
| `413` | Text too long |
| `429` | Rate limit exceeded |
| `503` | Model not loaded |

### `GET /api/health`

```json
{
  "status": "ok",
  "version": "1.0.0",
  "gliner_loaded": true,
  "chinese_loaded": true
}
```

## Configuration

| Variable | Default | Description |
|----------|---------|-------------|
| `PORT` | `8000` | Server port |
| `GLINER_MODELS` | `models/PII-Engineer-Multi-NER-v2.1` | Path to GLiNER model directory |
| `CHINESE_NER_MODEL` | `models/PII-Engineer-Chinese-NER-v1.0` | Path to Chinese NER model |
| `ORT_DYLIB_PATH` | auto-detect | Path to `libonnxruntime.so` / `.dylib` |
| `ORT_INTRA_THREADS` | `4` | ONNX Runtime intra-op threads |
| `ORT_INTER_THREADS` | `1` | ONNX Runtime inter-op threads |
| `PII_ENGINEER_RATE_LIMIT_RPM` | `120` | Max requests per minute per IP |
| `PII_ENGINEER_AUTO_REDACT_THRESHOLD` | `0.6` | Score above which entities are auto-redacted |
| `PII_ENGINEER_REVIEW_THRESHOLD` | `0.25` | Minimum score to include in results |

## Architecture

```
Request → Language detection → GLiNER2 NER + (Chinese NER if CJK)
            ↓
  Post-processing pipeline
  (reclassify → validate → filter → normalize → email/IP detect → threshold → dedup → merge)
            ↓
  Response (entities + redacted text)
```

**Model:** 5 ONNX models (encoder, span_rep, count_embed, count_pred, classifier) totaling ~1.2GB. INT8 quantized encoder available (511MB, ~15-20% faster on x86).

**Stack:** Rust + Axum + ONNX Runtime + HuggingFace Tokenizers

## Development

```bash
cargo build --workspace          # debug build
cargo test --workspace           # run all tests
cargo clippy --workspace         # lint
cargo run --release -p pii-engineer-server  # run server
```

## Performance

| Setup | Latency | Throughput |
|-------|---------|------------|
| MacBook M-series (FP32) | ~150ms | ~6 req/s |
| 4-vCPU Xeon (INT8) | ~250ms | ~4 req/s |

## License

[AGPL-3.0](LICENSE) — free for open-source use. Commercial license available at [pii.engineer](https://pii.engineer).

See [NOTICE](NOTICE) for upstream attributions.
