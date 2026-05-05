---
title: "How to Redact PII from Documents Using the PII Engineer API"
date: "2026-05"
tag: "Tutorial"
description: "A practical guide to integrating PII Engineer's REST API into your document processing pipeline. Covers curl examples, Python integration, batch processing, and PDF-to-redacted-text workflows."
---

## Why API-Based Redaction

Manual PII redaction does not scale. Legal teams reviewing contracts, compliance officers processing KYC documents, and data engineers preparing training datasets all need automated PII detection that plugs into existing workflows.

PII Engineer exposes a simple REST API that accepts text and returns detected PII entities with character offsets. You process the document text, send it to the API, and use the response to mask, replace, or remove sensitive data. No cloud dependency — the server runs on your infrastructure.

## Starting the Server

PII Engineer is a Rust binary. Build and run:

```bash
cargo build --release --package pii-engineer-server
cargo run --release --package pii-engineer-server
```

On first launch, models download from HuggingFace (~600MB). After warmup, the API listens on `http://localhost:8000`.

## API Request Format

The detection endpoint accepts JSON with a `text` field and optional `labels` to restrict which entity types to detect:

```bash
curl -X POST http://localhost:8000/api/detect \
  -H "Content-Type: application/json" \
  -d '{
    "text": "Patient John Tan (NRIC: S9012345A) visited on 15/03/2024. Contact: john.tan@email.com, +65 9123 4567.",
    "labels": ["person_name", "government_id", "email_address", "phone_number", "date_of_birth"]
  }'
```

## API Response Format

The response contains an array of detected entities, each with the label, matched text, and character positions:

```json
{
  "entities": [
    {
      "label": "person_name",
      "text": "John Tan",
      "start": 8,
      "end": 16,
      "score": 0.92
    },
    {
      "label": "government_id",
      "text": "S9012345A",
      "start": 24,
      "end": 33,
      "score": 0.97
    },
    {
      "label": "date_of_birth",
      "text": "15/03/2024",
      "start": 46,
      "end": 56,
      "score": 0.88
    },
    {
      "label": "email_address",
      "text": "john.tan@email.com",
      "start": 67,
      "end": 85,
      "score": 0.99
    },
    {
      "label": "phone_number",
      "text": "+65 9123 4567",
      "start": 87,
      "end": 100,
      "score": 0.95
    }
  ]
}
```

Character offsets are zero-indexed, UTF-8 byte positions. The `score` field is the model's confidence (0.0-1.0).

## Redacting Text in Python

Use the response offsets to replace PII with placeholders. Process entities in reverse order to preserve positions:

```python
import requests

def detect_pii(text: str, labels: list[str] | None = None) -> list[dict]:
    payload = {"text": text}
    if labels:
        payload["labels"] = labels
    resp = requests.post("http://localhost:8000/api/detect", json=payload)
    resp.raise_for_status()
    return resp.json()["entities"]

def redact(text: str, entities: list[dict]) -> str:
    # Sort by start position descending to preserve offsets
    sorted_entities = sorted(entities, key=lambda e: e["start"], reverse=True)
    redacted = text
    for entity in sorted_entities:
        placeholder = f"[{entity['label'].upper()}]"
        redacted = redacted[:entity["start"]] + placeholder + redacted[entity["end"]:]
    return redacted

# Usage
text = "Contact Sarah Lim at sarah.lim@company.sg or +65 8234 5678."
entities = detect_pii(text)
print(redact(text, entities))
# Output: "Contact [PERSON_NAME] at [EMAIL_ADDRESS] or [PHONE_NUMBER]."
```

## Batch Processing Multiple Documents

For processing many documents, send requests concurrently. The server handles concurrent requests using async Rust — throughput scales with CPU cores.

```python
import requests
from concurrent.futures import ThreadPoolExecutor, as_completed

def process_document(doc_id: str, text: str) -> dict:
    entities = detect_pii(text)
    redacted = redact(text, entities)
    return {
        "doc_id": doc_id,
        "original_length": len(text),
        "entities_found": len(entities),
        "redacted_text": redacted
    }

documents = [
    ("doc_001", "Ahmad bin Ibrahim, IC 850612-10-5523, lives at 45 Jalan Ampang..."),
    ("doc_002", "Ms. Nguyen Thi Lan, CCCD 024198006789, phone 0912-345-678..."),
    ("doc_003", "Rajesh Kumar, Aadhaar 2345 6789 0123, DOB 12-Jan-1985..."),
]

results = []
with ThreadPoolExecutor(max_workers=8) as executor:
    futures = {
        executor.submit(process_document, doc_id, text): doc_id
        for doc_id, text in documents
    }
    for future in as_completed(futures):
        results.append(future.result())

print(f"Processed {len(results)} documents")
for r in results:
    print(f"  {r['doc_id']}: {r['entities_found']} entities redacted")
```

## PDF Document Pipeline

PDFs require text extraction before PII detection. Here is a complete pipeline using `pdfplumber` for extraction and PII Engineer for redaction:

```python
import pdfplumber
import requests
import json

def extract_pdf_text(pdf_path: str) -> list[dict]:
    """Extract text from each page of a PDF."""
    pages = []
    with pdfplumber.open(pdf_path) as pdf:
        for i, page in enumerate(pdf.pages):
            text = page.extract_text()
            if text and text.strip():
                pages.append({"page": i + 1, "text": text})
    return pages

def redact_pdf_pipeline(pdf_path: str, output_path: str):
    """Full pipeline: PDF -> text extraction -> PII detection -> redacted output."""
    pages = extract_pdf_text(pdf_path)
    print(f"Extracted {len(pages)} pages from {pdf_path}")

    results = []
    total_entities = 0

    for page_data in pages:
        entities = detect_pii(page_data["text"])
        redacted_text = redact(page_data["text"], entities)
        total_entities += len(entities)
        results.append({
            "page": page_data["page"],
            "entities": entities,
            "redacted_text": redacted_text
        })

    # Write redacted output
    with open(output_path, "w") as f:
        for result in results:
            f.write(f"--- Page {result['page']} ---\n")
            f.write(result["redacted_text"])
            f.write("\n\n")

    print(f"Redacted {total_entities} entities across {len(pages)} pages")
    print(f"Output written to {output_path}")
    return results

# Usage
redact_pdf_pipeline("patient_records.pdf", "patient_records_redacted.txt")
```

## Customizing Redaction Behavior

### Selective Label Detection

Only detect specific entity types by passing `labels`:

```python
# Only detect names and government IDs — ignore phone/email
entities = detect_pii(text, labels=["person_name", "government_id"])
```

### Custom Replacement Strategies

Instead of generic placeholders, use entity-specific masking:

```python
def redact_with_strategy(text: str, entities: list[dict], strategy: str = "label") -> str:
    sorted_entities = sorted(entities, key=lambda e: e["start"], reverse=True)
    redacted = text

    for entity in sorted_entities:
        if strategy == "label":
            replacement = f"[{entity['label'].upper()}]"
        elif strategy == "hash":
            import hashlib
            h = hashlib.sha256(entity["text"].encode()).hexdigest()[:8]
            replacement = f"[{entity['label'].upper()}_{h}]"
        elif strategy == "consistent":
            # Same PII text always gets the same placeholder
            import hashlib
            h = hashlib.sha256(entity["text"].encode()).hexdigest()[:6]
            replacement = f"ENTITY_{h}"
        elif strategy == "asterisk":
            replacement = "*" * len(entity["text"])
        else:
            replacement = "███"

        redacted = redacted[:entity["start"]] + replacement + redacted[entity["end"]:]
    return redacted
```

The `hash` and `consistent` strategies are useful when you need to preserve referential integrity — the same person mentioned multiple times gets the same pseudonym.

## Supported Entity Types

| Label | Examples |
|-------|----------|
| `person_name` | John Tan, Ahmad bin Ibrahim, Nguyen Thi Lan |
| `phone_number` | +65 9123 4567, 0912-345-678, 081234567890 |
| `government_id` | S9012345A (NRIC), 850612-10-5523 (MyKad), 024198006789 (CCCD) |
| `email_address` | user@example.com |
| `street_address` | 45 Jalan Ampang, Kuala Lumpur 50450 |
| `date_of_birth` | 15/03/1990, 12-Jan-1985 |
| `passport_number` | E12345678, A00123456 |
| `bank_account_number` | 1234-5678-9012 |
| `license_plate` | SGX1234A, B 1234 CD |

## Error Handling

The API returns standard HTTP status codes:

```python
def detect_pii_safe(text: str) -> list[dict]:
    try:
        resp = requests.post(
            "http://localhost:8000/api/detect",
            json={"text": text},
            timeout=30
        )
        if resp.status_code == 200:
            return resp.json()["entities"]
        elif resp.status_code == 422:
            print(f"Validation error: {resp.json()}")
            return []
        else:
            print(f"Server error: {resp.status_code}")
            return []
    except requests.exceptions.ConnectionError:
        print("Cannot connect to PII Engineer server")
        return []
    except requests.exceptions.Timeout:
        print("Request timed out — text may be too long, split into chunks")
        return []
```

## Text Length Considerations

The GLiNER2 model has a token limit per request (typically 512 tokens after tokenization). For long documents, split text into paragraphs or chunks with overlap:

```python
def chunk_text(text: str, max_chars: int = 1500, overlap: int = 200) -> list[str]:
    """Split text into overlapping chunks at sentence boundaries."""
    sentences = text.replace("\n", " ").split(". ")
    chunks = []
    current_chunk = ""

    for sentence in sentences:
        if len(current_chunk) + len(sentence) > max_chars and current_chunk:
            chunks.append(current_chunk.strip())
            # Keep overlap from end of previous chunk
            words = current_chunk.split()
            overlap_text = " ".join(words[-overlap // 5:]) if len(words) > overlap // 5 else ""
            current_chunk = overlap_text + " " + sentence + ". "
        else:
            current_chunk += sentence + ". "

    if current_chunk.strip():
        chunks.append(current_chunk.strip())

    return chunks
```

## Deployment

For production, run PII Engineer behind a reverse proxy. The server is stateless — scale horizontally by running multiple instances.

```nginx
upstream pii_engineer {
    server 127.0.0.1:8000;
    server 127.0.0.1:8001;
    server 127.0.0.1:8002;
}

server {
    listen 443 ssl;
    location /api/ {
        proxy_pass http://pii_engineer;
        proxy_read_timeout 30s;
    }
}
```

Each instance loads models into RAM (~700MB). On a 4-vCPU machine, a single instance handles ~4 requests/second at ~250ms latency.

## Source Code

PII Engineer is open source under Apache-2.0:

- Repository: [github.com/gantz-ai/pii.engineer](https://github.com/gantz-ai/pii.engineer)
- Models: [huggingface.co/pii-engineer](https://huggingface.co/pii-engineer)
