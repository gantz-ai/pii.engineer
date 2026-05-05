<p align="center">
  <a href="https://pii.engineer">
    <img src="https://pii.engineer/static/banner.webp" alt="PII Engineer" width="300" />
  </a>
</p>

<h1 align="center">PII Engineer</h1>

<p align="center">
  Fast, multilingual PII detection. 50+ languages, single model, no GPU required.
</p>

<p align="center">
  <a href="https://github.com/gantz-ai/pii.engineer/actions/workflows/ci.yml"><img src="https://github.com/gantz-ai/pii.engineer/actions/workflows/ci.yml/badge.svg" alt="CI" /></a>
  <img src="https://img.shields.io/badge/Made%20with-Rust-dea584?logo=rust&logoColor=white" alt="Made with Rust" />
  <a href="LICENSE"><img src="https://img.shields.io/badge/License-Apache%202.0-blue.svg" alt="License: Apache-2.0" /></a>
  <a href="https://huggingface.co/pii-engineer"><img src="https://img.shields.io/badge/HuggingFace-Models-yellow" alt="HuggingFace" /></a>
  <a href="https://pii.engineer/benchmarks"><img src="https://img.shields.io/badge/F1-0.902-brightgreen" alt="F1 Score" /></a>
</p>

<p align="center">
  <a href="https://pii.engineer">Live Demo</a> · <a href="https://pii.engineer/benchmarks">Benchmarks</a> · <a href="https://pii.engineer/docs.html">API Docs</a> · <a href="https://huggingface.co/pii-engineer/PII-Engineer-Multi-NER-v2.1">Models</a> · <a href="https://pii.engineer/blog">Blog</a>
</p>

---

## Why PII Engineer?

|                       | PII Engineer | Presidio      | spaCy       | AWS Comprehend |
| --------------------- | ------------ | ------------- | ----------- | -------------- |
| **F1 (multilingual)** | **0.86**     | 0.44          | 0.64        | 0.52           |
| **F1 (English)**      | **0.88**     | 0.80          | 0.83        | 0.82           |
| **Languages**         | **50+**      | ~10 locales   | 1 per model | 12             |
| **Latency (p50)**     | 180ms        | 80ms (w/ NER) | 120ms       | 200ms          |
| **GPU required**      | No           | No            | Optional    | N/A            |
| **Self-hosted**       | Yes          | Yes           | Yes         | No             |
| **Cost (1M req/mo)**  | **$42**      | $42           | $42         | ~$1,000        |

[Full benchmarks →](https://pii.engineer/benchmarks)

## Quick Start

```bash
cargo build --release --package pii-engineer-server
cargo run --release --package pii-engineer-server
# Models auto-download from HuggingFace on first run
# API ready at http://localhost:8000
```

```bash
curl -X POST http://localhost:8000/api/detect \
  -H "Content-Type: application/json" \
  -d '{"text": "John Doe, NRIC S9012345B, born 12 March 1985"}'
```

```json
{
  "entities": [
    { "type": "person_name", "value": "John Doe", "score": 0.99 },
    { "type": "government_id", "value": "S9012345B", "score": 0.99 },
    { "type": "date_of_birth", "value": "12 March 1985", "score": 0.97 }
  ],
  "redacted": "[PERSON_NAME], NRIC [GOVERNMENT_ID], born [DATE_OF_BIRTH]"
}
```

### Docker

```bash
docker build -t pii-engineer .
docker run -p 8000:8000 -v ./models:/app/models pii-engineer
```

## PII Types

| Type                  | Examples                                          |
| --------------------- | ------------------------------------------------- |
| `person_name`         | Sarah Lim, Ahmad bin Abdullah, 陈伟明             |
| `phone_number`        | +65 9123 4567, 0812-3456-7890                     |
| `government_id`       | S9012345B (NRIC), 3201010512890001 (NIK), Aadhaar |
| `street_address`      | 42 Orchard Road #08-12, Jl. Sudirman No. 1        |
| `date_of_birth`       | 12 March 1985, 1990-05-15                         |
| `email_address`       | john@example.com                                  |
| `passport_number`     | E12345678                                         |
| `license_plate`       | SBA1234A, B 1234 CD                               |
| `bank_account_number` | 1234-5678-9012                                    |

## Supported Languages

**Primary:** English, Malay, Tamil, Chinese, Indonesian, Vietnamese

**Secondary:** Thai, Hindi, Bengali, Korean, Japanese, German, French, Russian, and [40+ more](https://pii.engineer/benchmarks)

## API

### `POST /api/detect`

| Field    | Type     | Default     | Description                               |
| -------- | -------- | ----------- | ----------------------------------------- |
| `text`   | string   | required    | Input text (max 50,000 chars)             |
| `labels` | string[] | all 9 types | PII types to detect                       |
| `boost`  | string[] | []          | Labels to boost with description matching |

### `GET /api/health`

```json
{
  "status": "ok",
  "version": "1.0.0",
  "gliner_loaded": true,
  "chinese_loaded": true
}
```

## Architecture

```
Request → Language detection → GLiNER2 NER + (Chinese NER if CJK)
            ↓
  Post-processing pipeline
  (reclassify → validate → filter → normalize → email/IP detect → threshold → dedup → merge)
            ↓
  Response (entities + redacted text)
```

**Model:** Fine-tuned [GLiNER2](https://huggingface.co/fastino/gliner2-multi-v1) (mDeBERTa-v3-base, 280M params) with 5 ONNX models. INT8 quantized encoder for CPU inference.

**Stack:** Rust + Axum + ONNX Runtime + HuggingFace Tokenizers

## Configuration

| Variable                      | Default                                | Description                            |
| ----------------------------- | -------------------------------------- | -------------------------------------- |
| `PORT`                        | `8000`                                 | Server port                            |
| `GLINER_MODELS`               | `models/PII-Engineer-Multi-NER-v2.1`   | GLiNER model path                      |
| `CHINESE_NER_MODEL`           | `models/PII-Engineer-Chinese-NER-v1.0` | Chinese NER model path                 |
| `ORT_DYLIB_PATH`              | auto-detect                            | Path to `libonnxruntime.so` / `.dylib` |
| `ORT_INTRA_THREADS`           | `4`                                    | ONNX Runtime intra-op threads          |
| `ORT_INTER_THREADS`           | `1`                                    | ONNX Runtime inter-op threads          |
| `PII_ENGINEER_RATE_LIMIT_RPM` | `120`                                  | Max requests per minute per IP         |

## Performance

| Setup                   | Latency | Throughput |
| ----------------------- | ------- | ---------- |
| MacBook M-series (FP32) | ~150ms  | ~6 req/s   |
| 4-vCPU AMD (INT8)       | ~250ms  | ~4 req/s   |

## Development

```bash
cargo build --workspace
cargo test --workspace
cargo clippy --workspace
cargo run --release -p pii-engineer-server
```

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

[Apache-2.0](LICENSE)

See [NOTICE](NOTICE) for upstream attributions.
