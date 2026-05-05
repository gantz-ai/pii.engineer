---
title: "如何使用 PII Engineer API 从文档中脱敏个人信息"
date: "2026-05"
tag: "Tutorial"
description: "将 PII Engineer 的 REST API 集成到文档处理流水线中的实用指南。涵盖 curl 示例、Python 集成、批量处理和 PDF 转脱敏文本的工作流程。"
---

## 为什么选择基于 API 的脱敏方案

手动 PII 脱敏无法扩展。审查合同的法律团队、处理 KYC 文档的合规人员、以及准备训练数据集的数据工程师，都需要能够接入现有工作流的自动化 PII 检测。

PII Engineer 提供了一个简单的 REST API，接受文本输入并返回检测到的 PII 实体及其字符偏移量。您处理文档文本，发送到 API，然后使用响应结果来遮蔽、替换或删除敏感数据。无需云依赖——服务器在您自己的基础设施上运行。

## 启动服务器

PII Engineer 是一个 Rust 二进制程序。构建并运行：

```bash
cargo build --release --package pii-engineer-server
cargo run --release --package pii-engineer-server
```

首次启动时，模型会从 HuggingFace 下载（约 600MB）。预热完成后，API 将在 `http://localhost:8000` 上监听。

## API 请求格式

检测端点接受包含 `text` 字段和可选 `labels` 参数的 JSON，用于限制检测的实体类型：

```bash
curl -X POST http://localhost:8000/api/detect \
  -H "Content-Type: application/json" \
  -d '{
    "text": "Patient John Tan (NRIC: S9012345A) visited on 15/03/2024. Contact: john.tan@email.com, +65 9123 4567.",
    "labels": ["person_name", "government_id", "email_address", "phone_number", "date_of_birth"]
  }'
```

## API 响应格式

响应包含一个检测到的实体数组，每个实体带有标签、匹配文本和字符位置：

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

字符偏移量为从零开始的 UTF-8 字节位置。`score` 字段表示模型的置信度（0.0-1.0）。

## 使用 Python 进行文本脱敏

利用响应中的偏移量将 PII 替换为占位符。按逆序处理实体以保持位置不变：

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

## 批量处理多个文档

处理大量文档时，可以并发发送请求。服务器使用异步 Rust 处理并发请求——吞吐量随 CPU 核心数线性扩展。

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

## PDF 文档处理流水线

PDF 文件需要先进行文本提取，然后才能进行 PII 检测。以下是使用 `pdfplumber` 提取文本和 PII Engineer 进行脱敏的完整流水线：

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

## 自定义脱敏行为

### 选择性标签检测

通过传入 `labels` 参数仅检测特定实体类型：

```python
# Only detect names and government IDs — ignore phone/email
entities = detect_pii(text, labels=["person_name", "government_id"])
```

### 自定义替换策略

除了通用占位符外，还可以使用针对实体类型的遮蔽方式：

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

`hash` 和 `consistent` 策略在需要保持引用一致性时非常有用——同一个人在多次提及时会获得相同的化名。

## 支持的实体类型

| 标签 | 示例 |
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

## 错误处理

API 返回标准 HTTP 状态码：

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

## 文本长度注意事项

GLiNER2 模型每次请求有 token 限制（通常在分词后为 512 个 token）。对于长文档，请将文本按段落或重叠块进行分割：

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

## 部署

在生产环境中，请将 PII Engineer 部署在反向代理后面。服务器是无状态的——可以通过运行多个实例进行水平扩展。

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

每个实例将模型加载到内存中（约 700MB）。在 4 核 vCPU 机器上，单个实例可以在约 250ms 延迟下处理约 4 个请求/秒。

## 源代码

PII Engineer 在 AGPL-3.0 许可证下开源：

- 代码仓库：[github.com/gantz-ai/pii.engineer](https://github.com/gantz-ai/pii.engineer)
- 模型：[huggingface.co/pii-engineer](https://huggingface.co/pii-engineer)
