---
title: "Phát Hiện PII trong Chat Log và Dữ Liệu Hỗ Trợ Khách Hàng"
date: "2026-05"
tag: "Use Case"
description: "Cách quét tin nhắn Slack, ticket Zendesk, bản xuất WhatsApp và bản ghi hỗ trợ khách hàng để tìm dữ liệu cá nhân — xử lý văn bản phi trang trọng, hội thoại đa ngôn ngữ và luồng tin nhắn khối lượng lớn."
---

## Vấn Đề Dữ Liệu Chat

Chat log là điểm mù nguy hiểm nhất trong tuân thủ PII. Khác với cơ sở dữ liệu có cấu trúc nơi dữ liệu cá nhân nằm trong các cột được gắn nhãn, tin nhắn chat phân tán PII khắp hàng nghìn cuộc hội thoại phi cấu trúc.

Một ticket Zendesk đơn lẻ có thể chứa tên đầy đủ, số điện thoại, số chứng minh nhân dân và địa chỉ nhà của khách hàng — tất cả được nhúng trong văn bản phi trang trọng, có lỗi chính tả, đa ngôn ngữ và chưa bao giờ được thiết kế để máy đọc.

Các tổ chức tích lũy dữ liệu chat từ:

- **Công cụ nội bộ** — Slack, Microsoft Teams, Discord
- **Hỗ trợ khách hàng** — Zendesk, Freshdesk, Intercom, LiveChat
- **Ứng dụng nhắn tin** — WhatsApp Business, Telegram, LINE, WeChat
- **Mạng xã hội** — Facebook Messenger, Instagram DM, Twitter DM

Mỗi nền tảng có định dạng xuất khác nhau, nhưng thách thức phát hiện PII là như nhau: tìm dữ liệu cá nhân trong văn bản phi trang trọng, nhiều nhiễu.

## Tại Sao Văn Bản Chat Khó cho NER

Các model NER tiêu chuẩn được huấn luyện trên bài báo, Wikipedia và tài liệu trang trọng. Văn bản chat phá vỡ các giả định của chúng:

### Ngôn Ngữ Phi Trang Trọng

```
"hey can u check acct for sarah lim nric S9012345A shes complaining abt charges"
```

- Từ viết tắt: "u", "acct", "abt"
- Thiếu viết hoa và dấu câu
- Tên được nhúng giữa câu mà không có giới thiệu trang trọng

### Chuyển Mã Ngôn Ngữ

Hội thoại chat Đông Nam Á thường trộn ngôn ngữ trong cùng một tin nhắn:

```
"Customer Ahmad bin Ismail called, dia kata IC dia 850612-10-5523, nak check balance"
```

Tiếng Malay và tiếng Anh trộn lẫn. Số chứng minh xuất hiện trong cấu trúc câu tiếng Malay.

### Ngữ Cảnh Nhiều Lượt

```
Agent: Can I have your name please?
Customer: It's Nguyen Thi Lan
Agent: And your ID number?
Customer: 024198006789
Agent: Phone?
Customer: 0912-345-678
```

Mỗi dòng đơn lẻ thiếu ngữ cảnh. Số ID `024198006789` chỉ trở thành PII có ý nghĩa nhờ câu hỏi trước đó.

### Định Dạng Nhiễu

```
**Name:** John Tan Wei Ming
**NRIC:** S 9012 345A (with spaces)
**Contact:** +65-9123-4567 / 91234567
```

Số chứng minh và số điện thoại xuất hiện với khoảng trắng, dấu gạch ngang và định dạng không nhất quán.

## PII Engineer Trên Dữ Liệu Chat

Model GLiNER2 của PII Engineer xử lý các thách thức này tốt hơn regex hoặc phương pháp dựa trên từ điển vì nó sử dụng dự đoán span theo ngữ cảnh thay vì khớp mẫu.

### Xử Lý Bản Xuất Slack

Slack xuất hội thoại dưới dạng JSON. Trích xuất và quét:

```python
import json
import requests
from pathlib import Path

def scan_slack_export(export_dir: str) -> list[dict]:
    """Scan a Slack export directory for PII."""
    results = []
    export_path = Path(export_dir)

    for channel_dir in export_path.iterdir():
        if not channel_dir.is_dir():
            continue
        for json_file in sorted(channel_dir.glob("*.json")):
            messages = json.loads(json_file.read_text())
            for msg in messages:
                text = msg.get("text", "")
                if len(text) < 10:
                    continue

                resp = requests.post("http://localhost:8000/api/detect", json={
                    "text": text,
                    "labels": ["person_name", "government_id", "phone_number",
                               "email_address", "street_address", "date_of_birth"]
                })
                entities = resp.json()["entities"]
                if entities:
                    results.append({
                        "channel": channel_dir.name,
                        "timestamp": msg.get("ts"),
                        "user": msg.get("user"),
                        "entities": entities,
                        "entity_count": len(entities)
                    })

    return results
```

### Xử Lý Ticket Zendesk

Ticket Zendesk thường chứa PII trong cả yêu cầu ban đầu và phản hồi của agent:

```python
import requests

def scan_zendesk_ticket(ticket: dict) -> dict:
    """Scan a Zendesk ticket and its comments for PII."""
    all_entities = []

    # Scan the ticket description
    if ticket.get("description"):
        resp = requests.post("http://localhost:8000/api/detect", json={
            "text": ticket["description"]
        })
        for entity in resp.json()["entities"]:
            entity["source"] = "description"
            all_entities.append(entity)

    # Scan each comment
    for i, comment in enumerate(ticket.get("comments", [])):
        body = comment.get("body", "")
        if len(body) < 10:
            continue
        resp = requests.post("http://localhost:8000/api/detect", json={
            "text": body
        })
        for entity in resp.json()["entities"]:
            entity["source"] = f"comment_{i}"
            entity["author"] = comment.get("author_id")
            all_entities.append(entity)

    return {
        "ticket_id": ticket["id"],
        "total_entities": len(all_entities),
        "entity_types": list(set(e["type"] for e in all_entities)),
        "entities": all_entities
    }
```

### Xử Lý Bản Xuất WhatsApp

Bản xuất văn bản WhatsApp tuân theo định dạng dòng có thể dự đoán:

```python
import re

def parse_whatsapp_export(filepath: str) -> list[dict]:
    """Parse WhatsApp text export into messages."""
    pattern = re.compile(
        r'\[(\d{1,2}/\d{1,2}/\d{2,4}),\s*(\d{1,2}:\d{2}:\d{2}\s*[AP]M)\]\s*([^:]+):\s*(.*)'
    )
    messages = []
    with open(filepath, 'r', encoding='utf-8') as f:
        for line in f:
            m = pattern.match(line.strip())
            if m:
                messages.append({
                    "date": m.group(1),
                    "time": m.group(2),
                    "sender": m.group(3),
                    "text": m.group(4)
                })
    return messages

def scan_whatsapp_export(filepath: str) -> list[dict]:
    messages = parse_whatsapp_export(filepath)
    results = []

    # Batch messages into chunks for efficiency
    batch = []
    for msg in messages:
        batch.append(msg)
        if len(batch) >= 20:
            combined = "\n".join(m["text"] for m in batch)
            resp = requests.post("http://localhost:8000/api/detect", json={
                "text": combined
            })
            if resp.json()["entities"]:
                results.append({
                    "message_range": f"{batch[0]['date']} - {batch[-1]['date']}",
                    "entities": resp.json()["entities"]
                })
            batch = []

    return results
```

## Xử Lý Hàng Loạt Ở Quy Mô Lớn

Các nền tảng hỗ trợ khách hàng tạo ra hàng nghìn tin nhắn mỗi ngày. Xử lý chúng một cách hiệu quả:

```python
from concurrent.futures import ThreadPoolExecutor, as_completed
import time

def scan_messages_batch(messages: list[dict], workers: int = 8) -> dict:
    """Scan a batch of messages for PII with concurrent requests."""
    flagged = []
    total_entities = 0
    start = time.time()

    def scan_one(msg):
        resp = requests.post("http://localhost:8000/api/detect", json={
            "text": msg["text"]
        }, timeout=10)
        entities = resp.json()["entities"]
        return msg, entities

    with ThreadPoolExecutor(max_workers=workers) as executor:
        futures = {executor.submit(scan_one, m): m for m in messages}
        for future in as_completed(futures):
            msg, entities = future.result()
            if entities:
                total_entities += len(entities)
                flagged.append({
                    "message_id": msg.get("id"),
                    "entity_count": len(entities),
                    "types": [e["type"] for e in entities]
                })

    elapsed = time.time() - start
    return {
        "total_scanned": len(messages),
        "messages_with_pii": len(flagged),
        "total_entities": total_entities,
        "duration_seconds": round(elapsed, 2),
        "messages_per_second": round(len(messages) / elapsed, 1),
        "flagged": flagged
    }
```

## Xử Lý Chat Đa Ngôn Ngữ

Hỗ trợ khách hàng Đông Nam Á thường xuyên liên quan đến văn bản đa ngôn ngữ. Model của PII Engineer xử lý điều này một cách tự nhiên:

| Ngôn ngữ kết hợp | Ví dụ | Entity được phát hiện |
|---|---|---|
| English | "Contact John at john@mail.com" | person_name, email_address |
| Malay-English | "Encik Ahmad IC 850612-10-5523" | person_name, government_id |
| Chinese | "联系陈伟明，电话 +65 9123 4567" | person_name, phone_number |
| Vietnamese | "Chị Nguyễn Thị Lan, CCCD 024198006789" | person_name, government_id |
| Hỗn hợp | "Customer Siti binti Yusof nak check, phone dia 0123456789" | person_name, phone_number |

Không cần phát hiện ngôn ngữ hay cấu hình. Model xử lý tất cả ngôn ngữ trong một lần chạy duy nhất.

## Cân Nhắc Tuân Thủ

### Chính Sách Lưu Giữ

Dữ liệu chat thường được lưu giữ lâu hơn mức cần thiết. Sau khi quét PII, hãy xem xét:

- **Che giấu PII trong hội thoại lưu trữ** — thay thế các entity được phát hiện bằng placeholder trước khi lưu trữ dài hạn
- **Đánh dấu hội thoại chứa PII** — gắn tag cho thời gian lưu giữ ngắn hơn
- **Tự động xóa** — loại bỏ các hội thoại có mật độ PII cao sau thời gian lưu giữ bắt buộc theo quy định

### Kiểm Soát Truy Cập

Không phải ai truy cập chat log đều cần nhìn thấy PII:

```python
def redact_for_role(text: str, entities: list[dict], role: str) -> str:
    """Redact PII based on viewer's role."""
    if role == "admin":
        return text  # Full access

    # Determine which entity types to redact for this role
    redact_types = {
        "analyst": ["government_id", "bank_account_number", "date_of_birth"],
        "support_lead": ["government_id", "bank_account_number"],
        "auditor": []  # Auditors see everything
    }.get(role, ["person_name", "government_id", "phone_number",
                  "email_address", "bank_account_number", "date_of_birth"])

    sorted_entities = sorted(
        [e for e in entities if e["type"] in redact_types],
        key=lambda e: e["start"], reverse=True
    )
    redacted = text
    for entity in sorted_entities:
        redacted = redacted[:entity["start"]] + f"[{entity['type'].upper()}]" + redacted[entity["end"]:]
    return redacted
```

### Nhật Ký Kiểm Toán

Ghi nhận những gì đã được quét và phát hiện, mà không ghi lại chính PII:

```python
def audit_log(source: str, message_id: str, entities: list[dict]):
    """Log PII detection event for compliance audit."""
    print(json.dumps({
        "event": "pii_scan",
        "source": source,
        "message_id": message_id,
        "timestamp": datetime.utcnow().isoformat(),
        "entities_found": len(entities),
        "entity_types": list(set(e["type"] for e in entities)),
        "highest_confidence": max((e["score"] for e in entities), default=0)
    }))
```

## Độ Chính Xác Trên Dữ Liệu Chat

Chúng tôi đã kiểm thử PII Engineer trên bộ dữ liệu chat tổng hợp mô phỏng hỗ trợ khách hàng bằng tiếng Anh, tiếng Malay, tiếng Trung và tiếng Việt:

| Loại Entity | F1 văn bản trang trọng | F1 văn bản chat | Chênh lệch |
|---|---|---|---|
| person_name | 0.90 | 0.83 | -0.07 |
| government_id | 0.95 | 0.93 | -0.02 |
| phone_number | 0.97 | 0.94 | -0.03 |
| email_address | 0.98 | 0.97 | -0.01 |
| street_address | 0.88 | 0.80 | -0.08 |

Mức giảm độ chính xác lớn nhất là ở tên người và địa chỉ — cả hai đều mơ hồ hơn trong văn bản phi trang trọng. Số chứng minh nhân dân và số điện thoại vẫn có độ chính xác cao vì định dạng của chúng rõ ràng bất kể ngữ cảnh xung quanh.

## Mã Nguồn

PII Engineer là mã nguồn mở theo giấy phép Apache-2.0:

- Repository: [github.com/gantz-ai/pii.engineer](https://github.com/gantz-ai/pii.engineer)
- Models: [huggingface.co/pii-engineer](https://huggingface.co/pii-engineer)
