---
title: "PII Detection for Healthcare: Protecting Patient Data Under HIPAA and PDPA"
date: "2026-05"
tag: "Healthcare"
description: "How PII Engineer handles healthcare-specific challenges — clinical notes, medication records, insurance claims — while meeting HIPAA and PDPA compliance requirements through self-hosted deployment."
---

## Healthcare Data is Different

Healthcare text contains the highest density of personally identifiable information of any industry. A single clinical note might contain the patient's full name, date of birth, government ID, phone number, address, and next-of-kin details — all within a few paragraphs of unstructured text.

The challenge is not just detection accuracy. It is doing it without exposing that data to third-party services. Sending patient records to a cloud API for PII detection defeats the purpose of protecting them.

## PHI: More Than Just Names

The HIPAA Privacy Rule defines 18 categories of Protected Health Information (PHI). Most PII detection tools focus on obvious identifiers like names and Social Security numbers. But healthcare text contains PHI that general-purpose NER models miss:

| PHI Category | Example in Clinical Notes |
|---|---|
| Patient name | "Pt: Sarah Tan Wei Lin" |
| Date of birth | "DOB: 15/03/1982" |
| Government ID | "NRIC: S8203451B" (Singapore), "IC: 820315-10-5523" (Malaysia) |
| Phone number | "NOK contact: +65 9123 4567" |
| Address | "Discharge to: Blk 123 Ang Mo Kio Ave 4 #05-678" |
| Email | "Follow-up appt confirmation to sarah.tan@gmail.com" |
| Dates (admission, discharge) | "Admitted 12-Mar-2024, discharged 15-Mar-2024" |
| Medical record number | "MRN: 2024-SG-089123" |
| Account numbers | "Insurance claim ref: PRU-2024-445566" |

Standard NER models struggle with healthcare text because:

- **Abbreviated names** — clinical notes use "Pt:" or "NOK:" prefixes, not "Dear Mr."
- **Dense formatting** — multiple identifiers packed into structured fields without natural language context
- **Multilingual names** — Southeast Asian healthcare serves patients with Chinese, Malay, Tamil, and Vietnamese names
- **Contextual dates** — not every date is a DOB; admission dates and procedure dates need different handling

## How PII Engineer Handles Medical Text

PII Engineer's 8-stage pipeline includes specific handling for healthcare patterns:

### Name Detection in Clinical Context

The GLiNER2 model is trained on clinical note patterns. It recognizes names preceded by healthcare-specific markers:

```
Pt: Ahmad bin Ismail           → person_name: "Ahmad bin Ismail"
NOK: Wife - Siti binti Yusof   → person_name: "Siti binti Yusof"
Attending Dr. Rajesh Kumar     → person_name: "Rajesh Kumar"
Referred by: Dr Ng Wei Kiat    → person_name: "Ng Wei Kiat"
```

The normalization stage strips prefixes ("Dr.", "Pt:", "Patient") so the detected entity is the clean name.

### Government IDs Across Jurisdictions

Healthcare systems in different countries use different national identifiers:

| Country | ID Type | Format | Example |
|---------|---------|--------|---------|
| Singapore | NRIC/FIN | 1 letter + 7 digits + 1 letter | S9012345A |
| Malaysia | MyKad | 6 digits - 2 digits - 4 digits | 820315-10-5523 |
| Indonesia | NIK | 16 digits | 3201011234560001 |
| Vietnam | CCCD | 12 digits | 024198006789 |
| India | Aadhaar | 4-4-4 digits | 2345 6789 0123 |
| Thailand | National ID | 13 digits | 1-1001-12345-12-1 |

PII Engineer's validation stage checks detected entities against known formats for each country, reducing false positives on random number strings in medical records.

### Date Classification

Not every date in a medical record is a date of birth. PII Engineer uses context signals:

- "DOB:", "D.O.B", "Born:", "Birth date" → classified as `date_of_birth`
- Dates appearing in admission/discharge context are detected but scored lower
- Dates clearly in the past (30+ years) appearing near patient demographics get higher confidence

### Medication and Diagnosis Text

Medical terminology creates noise for NER models. Drug names like "Panadol" or "Metformin 500mg" should not be flagged as entities. Disease names like "Wong's Syndrome" should not trigger person name detection.

PII Engineer's filter stage maintains a medical vocabulary that prevents common drug names, procedures, and conditions from being misclassified as PII.

## Self-Hosted: No PHI Leaves Your Network

This is the critical architectural decision for healthcare compliance. PII Engineer runs entirely on your infrastructure:

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

- **No internet required** after initial model download
- **No data leaves the machine** — inference runs locally on CPU
- **No third-party data processing agreement** needed
- **No cloud provider audit** required for the PII detection component
- **Air-gapped deployment** supported — copy the binary and model files via secure transfer

## HIPAA Compliance Mapping

HIPAA's Security Rule requires technical safeguards for systems handling PHI. Here is how a self-hosted PII Engineer deployment maps to key requirements:

| HIPAA Requirement | How PII Engineer Addresses It |
|---|---|
| Access Control (§164.312(a)) | Deploy behind your existing auth layer; API has no built-in auth by design — use your network controls |
| Audit Controls (§164.312(b)) | Server logs all requests with timestamps; integrate with your SIEM |
| Transmission Security (§164.312(e)) | Run on localhost or behind TLS reverse proxy; no external transmission |
| Minimum Necessary (§164.502(b)) | Detect only the PHI types you specify via `labels` parameter |
| Business Associate Agreement | Not required — no third-party processes your data |

The "Business Associate Agreement" point matters. When you use a cloud PII detection service, that vendor becomes a Business Associate under HIPAA and must sign a BAA. With PII Engineer self-hosted, this entire compliance burden disappears.

## PDPA Compliance (Singapore and Malaysia)

Southeast Asian healthcare increasingly falls under the Personal Data Protection Act — Singapore's PDPA (2012) and Malaysia's PDPA (2010). Key requirements:

### Singapore PDPA

| Obligation | Relevance to PII Detection |
|---|---|
| Consent (s13) | De-identify data before using for secondary purposes (research, analytics) |
| Purpose Limitation (s18) | Only collect/process PII for stated purpose |
| Protection (s24) | Reasonable security arrangements for personal data |
| Transfer Limitation (s26) | Personal data must not transfer outside Singapore without adequate protection |

The **Transfer Limitation** obligation is why cloud-based PII detection is problematic for Singapore healthcare. If your PII detection vendor processes data in US/EU data centers, you need to ensure adequate protection under s26. Self-hosted PII Engineer keeps all data within Singapore.

### Malaysia PDPA

| Principle | Relevance |
|---|---|
| Security Principle (s9) | Practical steps to protect personal data from loss, misuse, unauthorized access |
| Retention Principle (s10) | Do not retain personal data longer than necessary |
| Data Integrity Principle (s11) | Ensure personal data is accurate and complete |

## Practical Deployment for Healthcare

### Architecture for Hospital Systems

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

### Batch De-identification for Research Datasets

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

### Audit Logging

For compliance audits, log what was detected without logging the actual PII:

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

## Accuracy on Healthcare Text

We evaluated PII Engineer on a test set of synthetic clinical notes covering Singapore, Malaysia, and Indonesia healthcare patterns:

| Entity Type | Precision | Recall | F1 |
|---|---|---|---|
| Patient names (multilingual) | 0.84 | 0.87 | 0.85 |
| Government IDs (NRIC/MyKad/NIK) | 0.93 | 0.95 | 0.94 |
| Phone numbers | 0.96 | 0.97 | 0.97 |
| Dates of birth | 0.89 | 0.88 | 0.89 |
| Street addresses | 0.88 | 0.85 | 0.86 |
| Email addresses | 0.98 | 0.97 | 0.97 |

Person names are the hardest category due to the diversity of Southeast Asian naming conventions — patronymic names (bin/binti), multi-part Chinese names, compound Vietnamese names. The model handles these significantly better than regex or dictionary approaches.

## Limitations

Be transparent about what PII Engineer does not cover in healthcare:

- **Medical Record Numbers** — custom hospital numbering systems vary too widely for a generic model. Use regex rules specific to your MRN format.
- **Biometric data** — the API processes text, not images or fingerprints.
- **Genetic information** — DNA sequences or genetic markers are not detected as PII.
- **Implicit identifiers** — "the only patient admitted on Tuesday with a broken femur" is technically re-identifiable but not detectable by NER.

For these gaps, combine PII Engineer with domain-specific rules in your pipeline.

## Getting Started

PII Engineer is open source under Apache-2.0. For healthcare deployments, the self-hosted architecture means your compliance team only needs to audit your own infrastructure.

```bash
cargo build --release --package pii-engineer-server
cargo run --release --package pii-engineer-server
```

- Source code: [github.com/gantz-ai/pii.engineer](https://github.com/gantz-ai/pii.engineer)
- Models: [huggingface.co/pii-engineer](https://huggingface.co/pii-engineer)
