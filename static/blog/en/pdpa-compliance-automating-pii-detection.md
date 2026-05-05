---
title: "PDPA Compliance: Automating PII Detection in Southeast Asia"
date: "2026-05"
tag: "Compliance"
description: "How to use PII Engineer to scan documents, chat logs, and databases for personal data under PDPA, PDPD, PDP Law, and PIPL regulations."
---

## The Regulatory Landscape

Southeast and East Asia have rapidly adopted data protection legislation. Organizations operating across these jurisdictions face a patchwork of requirements — each law defines "personal data" slightly differently and imposes different obligations around collection, storage, and processing.

| Law | Country | Enacted | Key Scope |
|-----|---------|---------|-----------|
| PDPA | Singapore | 2012 | Any data that identifies an individual, including NRIC, phone, address |
| PDPA | Malaysia | 2010 | Commercial transactions involving personal data processing |
| PDPD | Vietnam | 2023 | Basic and sensitive personal data, including biometrics, health, finance |
| PDP Law | Indonesia | 2022 | General and specific personal data (NIK, health, biometrics) |
| PIPL | China | 2021 | Broad personal information scope, including name, ID, phone, location |

## Common Compliance Tasks

Regardless of which regulation applies, the practical requirements are similar:

1. **Data inventory** — identify where PII exists across your systems
2. **Access control audit** — verify that only authorized personnel can access PII
3. **Data minimization** — remove or redact PII that's no longer needed
4. **Breach detection** — monitor for unauthorized PII exposure in logs, exports, or messages
5. **Consent management** — ensure PII was collected with proper consent

PII Engineer addresses tasks 1, 3, and 4 — detecting PII wherever it appears, redacting it on demand, and flagging exposures in real-time.

## PII Types by Jurisdiction

Different laws emphasize different data types. PII Engineer's 9 entity types cover the most commonly regulated categories:

| PII Type | PDPA (SG) | PDPD (VN) | PDP Law (ID) | PIPL (CN) |
|----------|-----------|-----------|--------------|-----------|
| person_name | ✓ | ✓ | ✓ | ✓ |
| phone_number | ✓ | ✓ | ✓ | ✓ |
| government_id | NRIC/FIN | CCCD | NIK/KTP | 身份证 |
| street_address | ✓ | ✓ | ✓ | ✓ |
| date_of_birth | ✓ | ✓ | ✓ | ✓ |
| email_address | ✓ | ✓ | ✓ | ✓ |
| passport_number | ✓ | ✓ | ✓ | ✓ |
| license_plate | — | ✓ | ✓ | ✓ |
| bank_account_number | ✓ | ✓ | ✓ | ✓ |

## Scanning Documents

PII Engineer accepts any text input via its REST API. For document scanning, extract text first (using PDF parsers, OCR, or plain text readers) and send it to the detection endpoint:

```
curl -X POST http://localhost:8000/api/detect \
  -H "Content-Type: application/json" \
  -d '{"text": "Patient Nguyen Van An, CCCD 012345678901, phone 0901234567"}'
```

The response includes detected entities with types, positions, confidence scores, and a redacted version of the text:

```json
{
  "entities": [
    {"label": "person_name", "text": "Nguyen Van An", "score": 0.87},
    {"label": "government_id", "text": "012345678901", "score": 0.92},
    {"label": "phone_number", "text": "0901234567", "score": 0.95}
  ],
  "redacted": "Patient [PERSON_NAME], CCCD [GOVERNMENT_ID], phone [PHONE_NUMBER]"
}
```

## Language Coverage

Unlike English-only tools that fail on Southeast Asian text, PII Engineer supports 13+ languages natively. The GLiNER2 model was trained on multilingual data spanning:

- **Primary:** English, Malay, Tamil, Chinese, Indonesian, Vietnamese
- **Secondary:** Thai, Hindi, Bengali, Korean, German, French, Russian

This means a single deployment handles customer support tickets in Bahasa Indonesia, medical records in Vietnamese, legal contracts in Chinese, and HR documents in Malay — without switching models or configurations.

## Government ID Formats

Each country uses different ID formats, and the model recognizes all of them:

- **Singapore NRIC:** S1234567A (letter + 7 digits + letter)
- **Vietnam CCCD:** 012345678901 (12 digits)
- **Indonesia NIK:** 3201234567890001 (16 digits)
- **China 身份证:** 110101199001011234 (18 digits)
- **India Aadhaar:** 1234 5678 9012 (12 digits, often spaced)
- **Malaysia MyKad:** 901231-14-1234 (12 digits with dashes)

The post-processing validation stage ensures detected IDs match expected formats, reducing false positives.

## Real-Time Monitoring

For compliance teams that need continuous monitoring, PII Engineer can be deployed as a service that scans:

- Customer support chat messages before they're stored
- Database exports and backups before transfer
- Log files for accidentally logged PII
- Email content for data leakage detection

At ~150ms per request on Apple Silicon (or ~250ms on server CPUs), it handles real-time scanning without noticeable latency.

## Self-Hosted = Data Stays On-Premise

A critical advantage for compliance: PII Engineer runs entirely on your infrastructure. No data leaves your network. This eliminates the paradox of sending PII to a cloud service in order to detect PII — a common concern with SaaS-based scanning tools.

The server runs on CPU, requires no GPU, and auto-downloads models from HuggingFace on first run. A single `cargo run` gets you a production-ready endpoint.

## Getting Started

```
cargo build --release --package pii-engineer-server
cargo run --release --package pii-engineer-server
# Models auto-download from HuggingFace
# Server starts at http://localhost:8000
```

For compliance scanning at scale, deploy behind your existing API gateway and integrate with your data pipeline. The AGPL-3.0 license allows free use in open-source systems; commercial licenses are available for proprietary deployments.

Source: [github.com/gantz-ai/pii.engineer](https://github.com/gantz-ai/pii.engineer)
