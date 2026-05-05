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

## Features

- **Multilingual** — single model handles 50+ languages including CJK, SEA, South Asian, and European languages
- **High accuracy** — 0.90 F1 overall, outperforms regex-based tools on non-English text
- **Fast** — ~180ms p50 on CPU (INT8 quantized ONNX inference)
- **Zero-shot labels** — detect custom entity types without retraining
- **Self-hosted** — runs on a $42/mo VPS, no external API calls, your data never leaves your server
- **Single binary** — Rust binary with embedded static assets, no Python runtime or dependency hell
- **Auto-redaction** — returns both detected entities and redacted text in one call
- **9 PII types** — person names, phone numbers, government IDs, addresses, DOB, emails, passports, license plates, bank accounts

## Quick Start

### From Source

```bash
cargo build --release --package pii-engineer-server
cargo run --release --package pii-engineer-server
# Models auto-download from HuggingFace on first run
# API ready at http://localhost:8000
```

### Docker

```bash
docker build -t pii-engineer .
docker run -p 8000:8000 -v ./models:/app/models pii-engineer
```

### Test It

```bash
curl -X POST http://localhost:8000/api/detect \
  -H "Content-Type: application/json" \
  -d '{"text": "John Doe, NRIC S9012345B, born 12 March 1985"}'
```

Response:

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

## Integration Examples

### Python

```python
import requests

response = requests.post("http://localhost:8000/api/detect", json={
    "text": "Ahmad bin Abdullah, +60 12-345 6789, IC 901201-14-5678"
})
data = response.json()
print(data["redacted"])
# [PERSON_NAME], [PHONE_NUMBER], IC [GOVERNMENT_ID]
```

### JavaScript / Node.js

```javascript
const res = await fetch("http://localhost:8000/api/detect", {
  method: "POST",
  headers: { "Content-Type": "application/json" },
  body: JSON.stringify({
    text: "Nguyễn Văn A, CCCD 079201012345, sinh ngày 15/03/1990"
  }),
});
const { entities, redacted } = await res.json();
console.log(redacted);
// [PERSON_NAME], CCCD [GOVERNMENT_ID], sinh ngày [DATE_OF_BIRTH]
```

### cURL (batch labels)

```bash
curl -X POST http://localhost:8000/api/detect \
  -H "Content-Type: application/json" \
  -d '{
    "text": "Call me at 9123 4567 or email john@acme.com",
    "labels": ["phone_number", "email_address"]
  }'
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

**Primary (highest accuracy):** English, Malay, Tamil, Chinese, Indonesian, Vietnamese

**Secondary:** Thai, Hindi, Bengali, Korean, Japanese, German, French, Spanish, Portuguese, Russian, Arabic, Turkish, Polish, Dutch, Italian, Swedish, and [35+ more](https://pii.engineer/benchmarks)

The model handles multilingual text natively — mixed-language documents (e.g., English + Chinese + Malay in one paragraph) work without language selection.

## Use Cases

- **PDPA / GDPR compliance** — scan documents, databases, and logs for personal data before audits
- **LLM guardrails** — redact PII before sending user input to GPT/Claude/Gemini
- **Data pipelines** — clean PII from ETL outputs, data warehouse columns, Kafka streams
- **Chat moderation** — detect PII in real-time in Slack, support tickets, or chat apps
- **Code review** — catch hardcoded PII in test fixtures, config files, and documentation
- **Document redaction** — auto-redact contracts, resumes, medical records before sharing

## API Reference

### `POST /api/detect`

| Field    | Type     | Default     | Description                               |
| -------- | -------- | ----------- | ----------------------------------------- |
| `text`   | string   | required    | Input text (max 50,000 chars)             |
| `labels` | string[] | all 9 types | PII types to detect                       |
| `boost`  | string[] | []          | Labels to boost with description matching |

**Response:**

```json
{
  "entities": [
    { "type": "person_name", "value": "John Doe", "start": 0, "end": 8, "score": 0.99, "needs_review": false }
  ],
  "redacted": "[PERSON_NAME] lives at [STREET_ADDRESS]",
  "original": "John Doe lives at 123 Main St"
}
```

### `GET /api/health`

```json
{ "status": "ok", "version": "1.0.0", "gliner_loaded": true, "chinese_loaded": true }
```

## Architecture

```
Request → Language detection → GLiNER2 NER + (Chinese NER if CJK)
            ↓
  Post-processing pipeline (8 stages)
  reclassify → validate → filter → normalize → email/IP detect → threshold → dedup → merge
            ↓
  Response (entities + redacted text)
```

**Model:** Fine-tuned [GLiNER2](https://huggingface.co/fastino/gliner2-multi-v1) (mDeBERTa-v3-base, 280M params) split into 5 ONNX models. INT8 quantized encoder for CPU inference.

**Stack:** Rust + Axum + ONNX Runtime + HuggingFace Tokenizers + mimalloc

**How it works:**
1. Text and entity labels are encoded together by the transformer encoder
2. Span representation layer scores all possible token spans (up to 8 tokens wide)
3. Classifier determines which spans match which PII labels
4. 8-stage post-processing pipeline validates, deduplicates, and merges results
5. Regex-based detection supplements NER for emails and IP addresses

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

| Setup                   | Latency (p50) | Throughput |
| ----------------------- | ------------- | ---------- |
| MacBook M-series (FP32) | ~150ms        | ~6 req/s   |
| 4-vCPU AMD (INT8)       | ~250ms        | ~4 req/s   |
| 8-vCPU AMD (INT8)       | ~180ms        | ~5 req/s   |

Memory usage: ~800MB (model weights loaded in RAM).

Tips:
- Set `ORT_INTRA_THREADS` equal to your vCPU count
- INT8 encoder gives ~40% speedup with <0.5% accuracy loss
- First request after idle is slower — the server runs periodic warmup to mitigate this

## Development

```bash
cargo build --workspace
cargo test --workspace
cargo clippy --workspace
cargo run --release -p pii-engineer-server
```

### Project Structure

```
crates/
├── pii-engineer-core/     # NER engine, pipeline, model loading
│   └── src/
│       ├── gliner/        # GLiNER2 ONNX inference (v1, v2-compat, v2-full)
│       ├── pipeline.rs    # 8-stage post-processing
│       ├── labels.rs      # PII label definitions and canonicalization
│       └── lang.rs        # Language detection (CJK)
├── pii-engineer-server/   # HTTP server (Axum)
│   └── src/
│       ├── routes.rs      # API endpoints
│       ├── state.rs       # App state, model loading
│       └── middleware.rs  # Rate limiting, error handling
static/                    # Embedded frontend (rust-embed)
models/                    # ONNX models (auto-downloaded)
```

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines. We especially welcome:
- Validation rules for country-specific ID formats
- Test cases for underrepresented languages
- Performance optimizations

## License

[Apache-2.0](LICENSE)

See [NOTICE](NOTICE) for upstream attributions.
