---
title: "Detecting PII in Chat Logs and Customer Support Data"
date: "2026-05"
tag: "Use Case"
description: "How to scan Slack messages, Zendesk tickets, WhatsApp exports, and customer support transcripts for personal data — handling informal text, multilingual conversations, and high-volume message streams."
---

## The Chat Data Problem

Chat logs are the most dangerous blind spot in PII compliance. Unlike structured databases where personal data lives in labeled columns, chat messages scatter PII across thousands of unstructured conversations.

A single Zendesk ticket might contain a customer's full name, phone number, government ID, and home address — all embedded in casual, misspelled, multilingual text that was never designed to be machine-readable.

Organizations accumulate chat data from:

- **Internal tools** — Slack, Microsoft Teams, Discord
- **Customer support** — Zendesk, Freshdesk, Intercom, LiveChat
- **Messaging apps** — WhatsApp Business, Telegram, LINE, WeChat
- **Social media** — Facebook Messenger, Instagram DMs, Twitter DMs

Each platform has different export formats, but the PII detection challenge is the same: find personal data in informal, noisy text.

## Why Chat Text Is Hard for NER

Standard NER models are trained on news articles, Wikipedia, and formal documents. Chat text breaks their assumptions:

### Informal Language

```
"hey can u check acct for sarah lim nric S9012345A shes complaining abt charges"
```

- Abbreviated words: "u", "acct", "abt"
- Missing capitalization and punctuation
- Name embedded mid-sentence without formal introduction

### Code-Switching

Southeast Asian chat commonly mixes languages within a single message:

```
"Customer Ahmad bin Ismail called, dia kata IC dia 850612-10-5523, nak check balance"
```

Malay and English mixed. The government ID appears in a Malay sentence structure.

### Multi-Turn Context

```
Agent: Can I have your name please?
Customer: It's Nguyen Thi Lan
Agent: And your ID number?
Customer: 024198006789
Agent: Phone?
Customer: 0912-345-678
```

Each line alone lacks context. The ID number `024198006789` only becomes meaningful PII because of the preceding question.

### Noisy Formatting

```
**Name:** John Tan Wei Ming
**NRIC:** S 9012 345A (with spaces)
**Contact:** +65-9123-4567 / 91234567
```

IDs and phone numbers appear with inconsistent spacing, dashes, and formatting.

## PII Engineer on Chat Data

PII Engineer's GLiNER2 model handles these challenges better than regex or dictionary-based approaches because it uses contextual span prediction rather than pattern matching.

### Processing Slack Exports

Slack exports conversations as JSON. Extract and scan:

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

### Processing Zendesk Tickets

Zendesk tickets often contain PII in both the initial request and agent replies:

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

### Processing WhatsApp Exports

WhatsApp text exports follow a predictable line format:

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

## Batch Processing at Scale

Customer support platforms generate thousands of messages daily. Process them efficiently:

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

## Handling Multilingual Chat

Southeast Asian customer support frequently involves multilingual text. PII Engineer's model handles this natively:

| Language Mix | Example | Detected Entities |
|---|---|---|
| English | "Contact John at john@mail.com" | person_name, email_address |
| Malay-English | "Encik Ahmad IC 850612-10-5523" | person_name, government_id |
| Chinese | "联系陈伟明，电话 +65 9123 4567" | person_name, phone_number |
| Vietnamese | "Chị Nguyễn Thị Lan, CCCD 024198006789" | person_name, government_id |
| Mixed | "Customer Siti binti Yusof nak check, phone dia 0123456789" | person_name, phone_number |

No language detection or configuration needed. The model processes all languages in a single pass.

## Compliance Considerations

### Retention Policies

Chat data often has longer retention than necessary. After PII scanning, consider:

- **Redact PII in archived conversations** — replace detected entities with placeholders before long-term storage
- **Flag conversations containing PII** — tag for shorter retention periods
- **Auto-delete** — remove conversations with high PII density after the compliance-required retention period

### Access Controls

Not everyone who accesses chat logs needs to see PII:

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

### Audit Trail

Log what was scanned and what was found, without logging the PII itself:

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

## Accuracy on Chat Data

We tested PII Engineer on synthetic chat datasets simulating customer support in English, Malay, Chinese, and Vietnamese:

| Entity Type | Formal Text F1 | Chat Text F1 | Delta |
|---|---|---|---|
| person_name | 0.90 | 0.83 | -0.07 |
| government_id | 0.95 | 0.93 | -0.02 |
| phone_number | 0.97 | 0.94 | -0.03 |
| email_address | 0.98 | 0.97 | -0.01 |
| street_address | 0.88 | 0.80 | -0.08 |

The biggest accuracy drop is in person names and addresses — both are more ambiguous in informal text. Government IDs and phone numbers remain highly accurate because their format is distinctive regardless of surrounding context.

## Source Code

PII Engineer is open source under Apache-2.0:

- Repository: [github.com/gantz-ai/pii.engineer](https://github.com/gantz-ai/pii.engineer)
- Models: [huggingface.co/pii-engineer](https://huggingface.co/pii-engineer)
