---
title: "GDPR vs PDPA: PII Detection Requirements Compared"
date: "2026-05"
tag: "Compliance"
description: "A side-by-side comparison of EU GDPR and Southeast Asian PDPA regulations — what counts as personal data, how detection requirements differ, and how PII Engineer maps to both frameworks."
---

## Two Frameworks, One Problem

Organizations operating across Europe and Southeast Asia face overlapping but distinct privacy regulations. The EU's General Data Protection Regulation (GDPR) and Singapore's Personal Data Protection Act (PDPA) both require organizations to identify, protect, and manage personal data — but they define scope, obligations, and penalties differently.

Automated PII detection needs to account for both frameworks. A system tuned only for GDPR misses Southeast Asian identifiers. A system built for PDPA may not cover the broader "personal data" definition under GDPR.

## What Counts as Personal Data?

### GDPR Definition (Article 4)

> "Any information relating to an identified or identifiable natural person."

This is deliberately broad. It includes:

- Direct identifiers: name, ID number, email
- Indirect identifiers: location data, IP addresses, cookie IDs
- Special categories: racial/ethnic origin, political opinions, health data, biometric data, sexual orientation

GDPR's scope extends to any data that *could* identify someone, even in combination with other data.

### PDPA Definition (Section 2)

> "Data, whether true or not, about an individual who can be identified from that data; or from that data and other information to which the organisation has or is likely to have access."

PDPA is narrower in practice:

- Focuses on data that identifies a *specific* individual
- No explicit "special categories" concept (though the Do Not Call registry adds phone-specific rules)
- Business contact information is generally excluded when used for business purposes

### Practical Differences

| Aspect | GDPR | PDPA (Singapore) |
|---|---|---|
| Scope | Any data that could identify a person | Data that identifies a specific person |
| IP addresses | Personal data | Not explicitly covered |
| Cookie IDs | Personal data (with ePrivacy) | Generally not personal data |
| Business cards | Personal data | Excluded for business purposes |
| Deceased persons | Not covered | Not covered |
| Anonymized data | Not personal data if truly anonymous | Not personal data if truly anonymous |
| Pseudonymized data | Still personal data | Depends on re-identification risk |

## Key Obligations Compared

### Consent

| | GDPR | PDPA |
|---|---|---|
| Default | Opt-in required | Consent required (deemed or express) |
| Legitimate interest | Yes — can process without consent | No equivalent — consent or exception needed |
| Withdrawal | Must be as easy as giving consent | Must be able to withdraw |
| Children | Under 16 requires parental consent | No specific age threshold |

### Data Breach Notification

| | GDPR | PDPA |
|---|---|---|
| Authority notification | 72 hours | "As soon as practicable" (updated 2021: 3 calendar days) |
| Individual notification | When "high risk" to rights | When "significant harm" likely |
| Threshold | Risk-based assessment | 500+ individuals or significant harm |

### Cross-Border Transfer

| | GDPR | PDPA |
|---|---|---|
| Default rule | Prohibited unless adequate protection | Prohibited unless comparable protection |
| Mechanisms | Adequacy decisions, SCCs, BCRs | Contractual arrangements, binding corporate rules |
| Key concern | Data flowing to non-EU countries | Data flowing outside Singapore |

## PII Entity Types: Coverage Mapping

PII Engineer detects 9 entity types. Here's how each maps to GDPR and PDPA requirements:

| Entity Type | GDPR Relevance | PDPA Relevance | Detection Priority |
|---|---|---|---|
| `person_name` | Direct identifier (Art. 4) | Direct identifier (s2) | Critical for both |
| `government_id` | National ID number (Art. 87) | NRIC (advisory guidelines) | Critical for both |
| `phone_number` | Contact data | DNC Registry (Part IX) | High for both |
| `email_address` | Direct identifier | Direct identifier | High for both |
| `street_address` | Location data (Recital 30) | Direct identifier | High for both |
| `date_of_birth` | Indirect identifier | Indirect identifier | Medium — higher risk in combination |
| `passport_number` | Travel document (Art. 87) | Government-issued ID | Critical for both |
| `bank_account_number` | Financial data | Financial data | High for both |
| `license_plate` | Indirect identifier (can trace owner) | Indirect identifier | Medium for both |

### GDPR-Specific Gaps

GDPR's broader definition means some personal data types fall outside standard NER detection:

- **IP addresses** — not entity-like, but personal data under GDPR. Use regex patterns alongside NER.
- **Cookie identifiers** — technical strings, not natural language entities
- **Genetic/biometric data** — special category, requires domain-specific detection
- **Location data** — GPS coordinates, cell tower IDs. PII Engineer detects street addresses but not raw coordinates.

### PDPA-Specific Focus

Singapore's PDPA has particular emphasis on:

- **NRIC numbers** — Singapore issued advisory guidelines specifically restricting NRIC collection. PII Engineer validates NRIC format (letter + 7 digits + checksum letter).
- **Phone numbers** — the Do Not Call (DNC) Registry makes phone number detection especially important for marketing compliance.

## Detection Strategy by Regulation

### For GDPR Compliance

```python
# GDPR: detect all standard PII types + augment with regex for technical identifiers
entities = detect_pii(text, labels=[
    "person_name",
    "government_id",
    "phone_number",
    "email_address",
    "street_address",
    "date_of_birth",
    "passport_number",
    "bank_account_number",
    "license_plate"
])

# Supplement with regex for GDPR-specific technical identifiers
import re
ip_pattern = re.compile(r'\b(?:\d{1,3}\.){3}\d{1,3}\b')
ips = [{"label": "ip_address", "text": m.group(), "start": m.start(), "end": m.end()}
       for m in ip_pattern.finditer(text)]
entities.extend(ips)
```

### For PDPA Compliance

```python
# PDPA: focus on direct identifiers, especially NRIC and phone
entities = detect_pii(text, labels=[
    "person_name",
    "government_id",    # catches NRIC, FIN
    "phone_number",     # DNC registry compliance
    "email_address",
    "street_address",
    "date_of_birth"
])

# Filter for Singapore-specific ID formats if needed
sg_entities = [e for e in entities
    if e["label"] != "government_id"
    or re.match(r'^[STFGM]\d{7}[A-Z]$', e["text"])]
```

### For Dual Compliance (GDPR + PDPA)

```python
# Use the full label set — covers both frameworks
entities = detect_pii(text)

# Classify each entity by regulatory relevance
for entity in entities:
    entity["gdpr_relevant"] = True  # All PII types are GDPR-relevant
    entity["pdpa_relevant"] = entity["label"] in [
        "person_name", "government_id", "phone_number",
        "email_address", "street_address", "date_of_birth"
    ]
```

## Self-Hosting: The Compliance Shortcut

Both GDPR and PDPA impose restrictions on cross-border data transfers. Using a cloud-based PII detection service creates a data transfer event — you're sending personal data to a third party for processing.

Self-hosting PII Engineer eliminates this entirely:

| Compliance Concern | Cloud PII Service | Self-Hosted PII Engineer |
|---|---|---|
| GDPR Data Processor Agreement | Required (Art. 28) | Not needed |
| PDPA Data Intermediary | Must contractually bind | Not applicable |
| Cross-border transfer assessment | Required if service is outside jurisdiction | No transfer occurs |
| Data Processing Impact Assessment | May be required (Art. 35) | Simplified — no external processing |
| Vendor audit rights | Must negotiate | Not applicable |

## Regional PII Patterns

PII Engineer's multilingual model handles the distinct personal data patterns across EU and ASEAN:

| Region | Name Pattern | ID Format | Address Format |
|---|---|---|---|
| EU (Germany) | "Hans Müller" | Personalausweisnr | Straße + PLZ + Stadt |
| EU (France) | "Jean-Pierre Dubois" | Numéro de sécurité sociale | Rue + Code postal + Ville |
| Singapore | "Tan Ah Kow" / "S. Ramasamy" | NRIC: S9012345A | Blk + Street + #unit |
| Malaysia | "Ahmad bin Ismail" | MyKad: 820315-10-5523 | Jalan + Taman + Poskod |
| Vietnam | "Nguyen Thi Lan" | CCCD: 024198006789 | Số + Đường + Quận |
| Indonesia | "Budi Santoso" | NIK: 3201011234560001 | Jalan + RT/RW + Kecamatan |

The GLiNER2 model handles all these patterns without language-specific configuration. A single API call detects PII regardless of the language or regional format in the input text.

## Recommendations

1. **Start with full detection** — use all 9 entity types. It's easier to filter results downstream than to miss entities.
2. **Map to your specific obligations** — not every detected entity requires the same handling. GDPR special categories need stronger protections than standard personal data.
3. **Document your detection coverage** — both GDPR (Art. 30) and PDPA (s24) require demonstrating your data protection measures. A documented PII detection pipeline strengthens your compliance posture.
4. **Self-host for simplicity** — eliminating the data processor relationship removes an entire category of compliance obligations.

## Source Code

PII Engineer is open source under Apache-2.0:

- Repository: [github.com/gantz-ai/pii.engineer](https://github.com/gantz-ai/pii.engineer)
- Models: [huggingface.co/pii-engineer](https://huggingface.co/pii-engineer)
