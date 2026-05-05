---
title: "在聊天记录和客户支持数据中检测 PII"
date: "2026-05"
tag: "Use Case"
description: "如何扫描 Slack 消息、Zendesk 工单、WhatsApp 导出和客户支持对话记录中的个人数据——处理非正式文本、多语言对话和高流量消息流。"
---

## 聊天数据问题

聊天记录是 PII 合规中最危险的盲区。与个人数据存储在带标签列中的结构化数据库不同，聊天消息将 PII 分散在数千条非结构化对话中。

一个 Zendesk 工单可能包含客户的全名、电话号码、政府证件号和家庭地址——全部嵌入在随意的、拼写错误的、多语言的文本中，这些文本从未被设计为机器可读。

组织从以下来源积累聊天数据：

- **内部工具** —— Slack、Microsoft Teams、Discord
- **客户支持** —— Zendesk、Freshdesk、Intercom、LiveChat
- **即时通讯应用** —— WhatsApp Business、Telegram、LINE、WeChat
- **社交媒体** —— Facebook Messenger、Instagram DM、Twitter DM

每个平台有不同的导出格式，但 PII 检测的挑战是相同的：在非正式、噪声较多的文本中找到个人数据。

## 为什么聊天文本对 NER 来说很难

标准 NER 模型是在新闻文章、维基百科和正式文档上训练的。聊天文本打破了它们的假设：

### 非正式语言

```
"hey can u check acct for sarah lim nric S9012345A shes complaining abt charges"
```

- 缩写词："u"、"acct"、"abt"
- 缺少大写和标点
- 姓名嵌入句中，没有正式介绍

### 语码转换

东南亚聊天通常在一条消息中混合多种语言：

```
"Customer Ahmad bin Ismail called, dia kata IC dia 850612-10-5523, nak check balance"
```

马来语和英语混合。政府证件号出现在马来语句子结构中。

### 多轮上下文

```
Agent: Can I have your name please?
Customer: It's Nguyen Thi Lan
Agent: And your ID number?
Customer: 024198006789
Agent: Phone?
Customer: 0912-345-678
```

单独一行缺少上下文。证件号 `024198006789` 只因前面的问题才成为有意义的 PII。

### 格式噪声

```
**Name:** John Tan Wei Ming
**NRIC:** S 9012 345A (with spaces)
**Contact:** +65-9123-4567 / 91234567
```

证件号和电话号码以不一致的空格、短横线和格式出现。

## PII Engineer 处理聊天数据

PII Engineer 的 GLiNER2 模型比正则表达式或基于字典的方法更好地处理这些挑战，因为它使用上下文跨度预测而非模式匹配。

### 处理 Slack 导出

Slack 以 JSON 格式导出对话。提取并扫描：

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

### 处理 Zendesk 工单

Zendesk 工单通常在初始请求和客服回复中都包含 PII：

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

### 处理 WhatsApp 导出

WhatsApp 文本导出遵循可预测的行格式：

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

## 大规模批量处理

客户支持平台每天产生数千条消息。高效处理它们：

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

## 处理多语言聊天

东南亚客户支持经常涉及多语言文本。PII Engineer 的模型原生处理这一情况：

| 语言组合 | 示例 | 检测到的实体 |
|---|---|---|
| English | "Contact John at john@mail.com" | person_name, email_address |
| Malay-English | "Encik Ahmad IC 850612-10-5523" | person_name, government_id |
| Chinese | "联系陈伟明，电话 +65 9123 4567" | person_name, phone_number |
| Vietnamese | "Chị Nguyễn Thị Lan, CCCD 024198006789" | person_name, government_id |
| Mixed | "Customer Siti binti Yusof nak check, phone dia 0123456789" | person_name, phone_number |

无需语言检测或配置。模型在单次处理中处理所有语言。

## 合规考量

### 保留策略

聊天数据的保留时间通常超过必要时间。在 PII 扫描后，考虑：

- **在归档对话中脱敏 PII** —— 在长期存储前用占位符替换检测到的实体
- **标记包含 PII 的对话** —— 标记为更短的保留期限
- **自动删除** —— 在合规要求的保留期后删除 PII 密度高的对话

### 访问控制

并非所有访问聊天记录的人都需要看到 PII：

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

### 审计跟踪

记录扫描了什么和发现了什么，但不记录 PII 本身：

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

## 聊天数据上的准确率

我们在模拟英语、马来语、中文和越南语客户支持的合成聊天数据集上测试了 PII Engineer：

| 实体类型 | 正式文本 F1 | 聊天文本 F1 | 差值 |
|---|---|---|---|
| person_name | 0.90 | 0.83 | -0.07 |
| government_id | 0.95 | 0.93 | -0.02 |
| phone_number | 0.97 | 0.94 | -0.03 |
| email_address | 0.98 | 0.97 | -0.01 |
| street_address | 0.88 | 0.80 | -0.08 |

最大的准确率下降出现在人名和地址——在非正式文本中两者更加模糊。政府证件号和电话号码保持高准确率，因为它们的格式无论周围上下文如何都是独特的。

## 源代码

PII Engineer 在 AGPL-3.0 下开源：

- 仓库：[github.com/gantz-ai/pii.engineer](https://github.com/gantz-ai/pii.engineer)
- 模型：[huggingface.co/pii-engineer](https://huggingface.co/pii-engineer)
