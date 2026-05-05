---
title: "医疗保健中的 PII 检测：在 HIPAA 和 PDPA 合规下保护患者数据"
date: "2026-05"
tag: "Healthcare"
description: "PII Engineer 如何处理医疗保健特有的挑战——临床笔记、用药记录、保险理赔——同时通过自托管部署满足 HIPAA 和 PDPA 合规要求。"
---

## 医疗数据的特殊性

医疗保健文本包含所有行业中密度最高的个人可识别信息。一份临床笔记可能在几段非结构化文本中包含患者的全名、出生日期、政府 ID、电话号码、地址和紧急联系人详情。

挑战不仅在于检测准确度，更在于在不将数据暴露给第三方服务的前提下完成检测。将患者记录发送到云端 API 进行 PII 检测，本身就违背了保护这些数据的初衷。

## PHI：不仅仅是姓名

HIPAA 隐私规则定义了 18 类受保护健康信息（PHI）。大多数 PII 检测工具只关注明显的标识符，如姓名和社会安全号码。但医疗文本中包含通用 NER 模型难以识别的 PHI：

| PHI 类别 | 临床笔记中的示例 |
|---|---|
| 患者姓名 | "Pt: Sarah Tan Wei Lin" |
| 出生日期 | "DOB: 15/03/1982" |
| 政府 ID | "NRIC: S8203451B"（新加坡）、"IC: 820315-10-5523"（马来西亚） |
| 电话号码 | "NOK contact: +65 9123 4567" |
| 地址 | "Discharge to: Blk 123 Ang Mo Kio Ave 4 #05-678" |
| 电子邮件 | "Follow-up appt confirmation to sarah.tan@gmail.com" |
| 日期（入院、出院） | "Admitted 12-Mar-2024, discharged 15-Mar-2024" |
| 病历号 | "MRN: 2024-SG-089123" |
| 账户号码 | "Insurance claim ref: PRU-2024-445566" |

标准 NER 模型在医疗文本上表现不佳，原因在于：

- **缩写姓名** — 临床笔记使用"Pt:"或"NOK:"前缀，而非"Dear Mr."
- **密集格式** — 多个标识符被填入结构化字段中，缺乏自然语言上下文
- **多语言姓名** — 东南亚医疗服务的患者涵盖中文、马来文、泰米尔文和越南文姓名
- **上下文日期** — 并非所有日期都是出生日期；入院日期和手术日期需要不同处理

## PII Engineer 如何处理医疗文本

PII Engineer 的 8 阶段流水线包含针对医疗模式的专门处理：

### 临床上下文中的姓名检测

GLiNER2 模型在临床笔记模式上经过训练。它能识别以医疗特定标记为前缀的姓名：

```
Pt: Ahmad bin Ismail           → person_name: "Ahmad bin Ismail"
NOK: Wife - Siti binti Yusof   → person_name: "Siti binti Yusof"
Attending Dr. Rajesh Kumar     → person_name: "Rajesh Kumar"
Referred by: Dr Ng Wei Kiat    → person_name: "Ng Wei Kiat"
```

归一化阶段会剥离前缀（"Dr."、"Pt:"、"Patient"），使检测到的实体是干净的姓名。

### 跨司法管辖区的政府 ID

不同国家的医疗系统使用不同的国民身份标识：

| 国家 | ID 类型 | 格式 | 示例 |
|---------|---------|--------|---------|
| Singapore | NRIC/FIN | 1 letter + 7 digits + 1 letter | S9012345A |
| Malaysia | MyKad | 6 digits - 2 digits - 4 digits | 820315-10-5523 |
| Indonesia | NIK | 16 digits | 3201011234560001 |
| Vietnam | CCCD | 12 digits | 024198006789 |
| India | Aadhaar | 4-4-4 digits | 2345 6789 0123 |
| Thailand | National ID | 13 digits | 1-1001-12345-12-1 |

PII Engineer 的验证阶段会将检测到的实体与各国已知格式进行比对，降低医疗记录中随机数字串的误报率。

### 日期分类

医疗记录中并非每个日期都是出生日期。PII Engineer 使用上下文信号进行判断：

- "DOB:"、"D.O.B"、"Born:"、"Birth date" → 分类为 `date_of_birth`
- 出现在入院/出院上下文中的日期会被检测到，但置信度较低
- 明显在过去（30 年以上）且出现在患者人口统计信息附近的日期获得更高置信度

### 药物和诊断文本

医学术语会给 NER 模型带来噪声。药品名称如"Panadol"或"Metformin 500mg"不应被标记为实体。疾病名称如"Wong's Syndrome"不应触发人名检测。

PII Engineer 的过滤阶段维护一个医学词汇表，防止常见药品名称、手术名称和疾病名称被误分类为 PII。

## 自托管：PHI 永不离开您的网络

这是医疗合规的关键架构决策。PII Engineer 完全在您的基础设施上运行：

```
┌─────────────────────────────────────────────┐
│  Your Network / VPC                          │
│                                              │
│  ┌──────────┐     ┌──────────────────────┐  │
│  │ EHR/EMR  │────▶│  PII Engineer Server │  │
│  │ System   │◀────│  (localhost:8000)     │  │
│  └──────────┘     └──────────────────────┘  │
│                                              │
│  No external API calls. No cloud dependency. │
│  Models loaded from local disk.              │
└─────────────────────────────────────────────┘
```

- **无需互联网** — 初始模型下载后即可断网运行
- **数据不离开机器** — 推理在本地 CPU 上运行
- **无需第三方数据处理协议**
- **无需对 PII 检测组件进行云服务商审计**
- **支持物理隔离部署** — 通过安全传输复制二进制文件和模型文件

## HIPAA 合规映射

HIPAA 安全规则要求处理 PHI 的系统具备技术保障措施。以下是自托管 PII Engineer 部署如何对应关键要求：

| HIPAA 要求 | PII Engineer 如何满足 |
|---|---|
| Access Control (§164.312(a)) | 部署在现有身份认证层之后；API 有意不内置认证——使用您的网络控制 |
| Audit Controls (§164.312(b)) | 服务器记录所有带时间戳的请求；集成到您的 SIEM |
| Transmission Security (§164.312(e)) | 在 localhost 运行或部署在 TLS 反向代理之后；无外部传输 |
| Minimum Necessary (§164.502(b)) | 通过 `labels` 参数仅检测您指定的 PHI 类型 |
| Business Associate Agreement | 不需要——没有第三方处理您的数据 |

"Business Associate Agreement"这一点很重要。当您使用云端 PII 检测服务时，该供应商在 HIPAA 下成为业务关联方，必须签署 BAA。使用自托管的 PII Engineer，整个合规负担完全消除。

## PDPA 合规（新加坡和马来西亚）

东南亚的医疗保健越来越多地受到《个人数据保护法》的约束——新加坡的 PDPA（2012）和马来西亚的 PDPA（2010）。关键要求：

### 新加坡 PDPA

| 义务 | 与 PII 检测的相关性 |
|---|---|
| Consent (s13) | 用于二次目的（研究、分析）前需去标识化数据 |
| Purpose Limitation (s18) | 仅为声明目的收集/处理 PII |
| Protection (s24) | 对个人数据采取合理安全措施 |
| Transfer Limitation (s26) | 未经充分保护不得将个人数据转移至新加坡境外 |

**Transfer Limitation** 义务正是云端 PII 检测对新加坡医疗保健造成问题的原因。如果您的 PII 检测供应商在美国/欧盟数据中心处理数据，您需要确保符合 s26 的充分保护。自托管 PII Engineer 将所有数据保留在新加坡境内。

### 马来西亚 PDPA

| 原则 | 相关性 |
|---|---|
| Security Principle (s9) | 采取切实步骤保护个人数据免遭丢失、滥用、未经授权的访问 |
| Retention Principle (s10) | 不得超出必要时间保留个人数据 |
| Data Integrity Principle (s11) | 确保个人数据准确且完整 |

## 医疗保健实际部署

### 医院系统架构

```python
# Integration with EHR export pipeline
import requests

def deidentify_clinical_note(note_text: str) -> dict:
    """De-identify a clinical note for research use."""
    resp = requests.post("http://pii-engine:8000/api/detect", json={
        "text": note_text,
        "labels": [
            "person_name",
            "government_id",
            "phone_number",
            "email_address",
            "street_address",
            "date_of_birth"
        ]
    })
    entities = resp.json()["entities"]

    # Replace with consistent pseudonyms for research utility
    deidentified = note_text
    name_counter = 0
    name_map = {}

    # Sort reverse for safe replacement
    for entity in sorted(entities, key=lambda e: e["start"], reverse=True):
        if entity["label"] == "person_name":
            if entity["text"] not in name_map:
                name_counter += 1
                name_map[entity["text"]] = f"PATIENT_{name_counter:03d}"
            replacement = name_map[entity["text"]]
        elif entity["label"] == "date_of_birth":
            replacement = "[DOB_REDACTED]"
        elif entity["label"] == "government_id":
            replacement = "[ID_REDACTED]"
        else:
            replacement = f"[{entity['label'].upper()}]"

        deidentified = deidentified[:entity["start"]] + replacement + deidentified[entity["end"]:]

    return {
        "deidentified_text": deidentified,
        "entities_removed": len(entities),
        "entity_types": list(set(e["label"] for e in entities))
    }
```

### 研究数据集批量去标识化

```python
from pathlib import Path
from concurrent.futures import ThreadPoolExecutor
import json

def deidentify_dataset(input_dir: str, output_dir: str):
    """Batch de-identify clinical notes for research release."""
    input_path = Path(input_dir)
    output_path = Path(output_dir)
    output_path.mkdir(parents=True, exist_ok=True)

    files = list(input_path.glob("*.txt"))
    print(f"Processing {len(files)} clinical notes...")

    def process_file(filepath):
        text = filepath.read_text(encoding="utf-8")
        result = deidentify_clinical_note(text)
        out_file = output_path / filepath.name
        out_file.write_text(result["deidentified_text"], encoding="utf-8")
        return filepath.name, result["entities_removed"]

    with ThreadPoolExecutor(max_workers=4) as executor:
        results = list(executor.map(process_file, files))

    total_entities = sum(r[1] for r in results)
    print(f"Done. Removed {total_entities} PII entities from {len(files)} notes.")
```

### 审计日志

为合规审计记录检测内容，但不记录实际的 PII：

```python
import logging
from datetime import datetime

audit_logger = logging.getLogger("pii_audit")

def detect_and_audit(text: str, document_id: str, user_id: str) -> list[dict]:
    entities = detect_pii(text)

    # Log detection event without the actual PII values
    audit_logger.info(json.dumps({
        "timestamp": datetime.utcnow().isoformat(),
        "document_id": document_id,
        "user_id": user_id,
        "entities_detected": len(entities),
        "entity_types": [e["label"] for e in entities],
        "text_length": len(text)
    }))

    return entities
```

## 医疗文本的准确度

我们在涵盖新加坡、马来西亚和印度尼西亚医疗模式的合成临床笔记测试集上评估了 PII Engineer：

| 实体类型 | Precision | Recall | F1 |
|---|---|---|---|
| 患者姓名（多语言） | 0.84 | 0.87 | 0.85 |
| 政府 ID（NRIC/MyKad/NIK） | 0.93 | 0.95 | 0.94 |
| 电话号码 | 0.96 | 0.97 | 0.97 |
| 出生日期 | 0.89 | 0.88 | 0.89 |
| 街道地址 | 0.88 | 0.85 | 0.86 |
| 电子邮件地址 | 0.98 | 0.97 | 0.97 |

人名是最困难的类别，因为东南亚命名惯例多种多样——父名制（bin/binti）、多部分中文名、复合越南名。该模型在处理这些方面明显优于正则表达式或字典方法。

## 局限性

对于 PII Engineer 在医疗领域不涵盖的内容，需要保持透明：

- **病历号** — 各医院的自定义编号系统差异太大，通用模型无法覆盖。请使用针对您 MRN 格式的正则规则。
- **生物特征数据** — API 处理文本，不处理图像或指纹。
- **遗传信息** — DNA 序列或遗传标记不会被检测为 PII。
- **隐含标识符** — "周二唯一一个因股骨骨折入院的患者"在技术上是可重新识别的，但 NER 无法检测。

对于这些空白，请在您的流水线中将 PII Engineer 与领域特定规则结合使用。

## 开始使用

PII Engineer 在 Apache-2.0 许可证下开源。对于医疗部署，自托管架构意味着您的合规团队只需审计自己的基础设施。

```bash
cargo build --release --package pii-engineer-server
cargo run --release --package pii-engineer-server
```

- 源代码：[github.com/gantz-ai/pii.engineer](https://github.com/gantz-ai/pii.engineer)
- 模型：[huggingface.co/pii-engineer](https://huggingface.co/pii-engineer)
