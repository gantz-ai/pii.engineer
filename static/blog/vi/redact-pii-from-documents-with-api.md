---
title: "Cách Che Giấu PII Trong Tài Liệu Bằng API PII Engineer"
date: "2026-05"
tag: "Tutorial"
description: "Hướng dẫn thực hành tích hợp REST API của PII Engineer vào quy trình xử lý tài liệu. Bao gồm ví dụ curl, tích hợp Python, xử lý hàng loạt và quy trình chuyển đổi PDF thành văn bản đã được che giấu."
---

## Tại Sao Nên Che Giấu Qua API

Che giấu PII thủ công không thể mở rộng quy mô. Đội ngũ pháp lý rà soát hợp đồng, nhân viên tuân thủ xử lý tài liệu KYC, và kỹ sư dữ liệu chuẩn bị dataset huấn luyện đều cần phát hiện PII tự động tích hợp vào quy trình hiện có.

PII Engineer cung cấp một REST API đơn giản nhận văn bản và trả về các thực thể PII được phát hiện kèm vị trí ký tự. Bạn xử lý văn bản tài liệu, gửi tới API, và sử dụng phản hồi để che, thay thế hoặc xóa dữ liệu nhạy cảm. Không phụ thuộc cloud — server chạy trên hạ tầng của bạn.

## Khởi Động Server

PII Engineer là một binary Rust. Build và chạy:

```bash
cargo build --release --package pii-engineer-server
cargo run --release --package pii-engineer-server
```

Lần chạy đầu tiên, model sẽ được tải từ HuggingFace (~600MB). Sau khi khởi động xong, API lắng nghe tại `http://localhost:8000`.

## Định Dạng Request API

Endpoint phát hiện nhận JSON với trường `text` và tùy chọn `labels` để giới hạn loại thực thể cần phát hiện:

```bash
curl -X POST http://localhost:8000/api/detect \
  -H "Content-Type: application/json" \
  -d '{
    "text": "Patient John Tan (NRIC: S9012345A) visited on 15/03/2024. Contact: john.tan@email.com, +65 9123 4567.",
    "labels": ["person_name", "government_id", "email_address", "phone_number", "date_of_birth"]
  }'
```

## Định Dạng Response API

Phản hồi chứa một mảng các thực thể được phát hiện, mỗi thực thể gồm nhãn, văn bản khớp và vị trí ký tự:

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

Vị trí ký tự được đánh số từ 0, tính theo byte UTF-8. Trường `score` là độ tin cậy của model (0.0-1.0).

## Che Giấu Văn Bản Bằng Python

Sử dụng vị trí offset từ phản hồi để thay thế PII bằng placeholder. Xử lý các thực thể theo thứ tự ngược để giữ nguyên vị trí:

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

## Xử Lý Hàng Loạt Nhiều Tài Liệu

Để xử lý nhiều tài liệu, gửi request đồng thời. Server xử lý đồng thời bằng async Rust — thông lượng tăng theo số lõi CPU.

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

## Quy Trình Xử Lý Tài Liệu PDF

PDF cần trích xuất văn bản trước khi phát hiện PII. Đây là quy trình hoàn chỉnh sử dụng `pdfplumber` để trích xuất và PII Engineer để che giấu:

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

## Tùy Chỉnh Hành Vi Che Giấu

### Phát Hiện Chọn Lọc Theo Nhãn

Chỉ phát hiện các loại thực thể cụ thể bằng cách truyền `labels`:

```python
# Only detect names and government IDs — ignore phone/email
entities = detect_pii(text, labels=["person_name", "government_id"])
```

### Chiến Lược Thay Thế Tùy Chỉnh

Thay vì placeholder chung chung, sử dụng che giấu theo từng loại thực thể:

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

Chiến lược `hash` và `consistent` hữu ích khi bạn cần giữ tính toàn vẹn tham chiếu — cùng một người được nhắc đến nhiều lần sẽ nhận cùng một bí danh.

## Các Loại Thực Thể Được Hỗ Trợ

| Nhãn | Ví dụ |
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

## Xử Lý Lỗi

API trả về mã trạng thái HTTP tiêu chuẩn:

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

## Lưu Ý Về Độ Dài Văn Bản

Model GLiNER2 có giới hạn token cho mỗi request (thường là 512 token sau khi tokenize). Với tài liệu dài, chia văn bản thành đoạn hoặc chunk có phần chồng lặp:

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

## Triển Khai

Trong môi trường production, chạy PII Engineer phía sau reverse proxy. Server không lưu trạng thái — mở rộng theo chiều ngang bằng cách chạy nhiều instance.

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

Mỗi instance tải model vào RAM (~700MB). Trên máy 4-vCPU, một instance xử lý khoảng 4 request/giây với độ trễ ~250ms.

## Mã Nguồn

PII Engineer là mã nguồn mở theo giấy phép Apache-2.0:

- Repository: [github.com/gantz-ai/pii.engineer](https://github.com/gantz-ai/pii.engineer)
- Models: [huggingface.co/pii-engineer](https://huggingface.co/pii-engineer)
