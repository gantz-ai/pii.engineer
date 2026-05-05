---
title: "GLiNER2 vs spaCy vs Presidio: Multilingual PII Detection Compared"
date: "2026-05"
tag: "Comparison"
description: "A technical comparison of three approaches to PII detection — regex-based (Presidio), statistical NER (spaCy), and span-based transformer NER (GLiNER2/PII Engineer). Evaluated on multilingual support, accuracy, latency, and deployment complexity."
---

## Three Approaches to PII Detection

There is no single correct architecture for detecting personally identifiable information in text. The right choice depends on your language requirements, accuracy needs, infrastructure constraints, and maintenance budget. This article compares three fundamentally different approaches:

1. **Microsoft Presidio** — rule-based with regex patterns and optional NER enhancement
2. **spaCy NER** — statistical named entity recognition with language-specific models
3. **GLiNER2 (PII Engineer)** — span-based transformer NER with multilingual coverage from a single model

Each makes different tradeoffs. We will be specific about where each excels and where each falls short.

## Architecture Differences

### Presidio: Regex + Recognizers

Presidio uses a recognizer-based architecture. Each PII type has one or more "recognizers" — Python classes that define regex patterns, deny lists, or context words for a specific entity type.

```python
# How Presidio detects a Singapore NRIC
class SingaporeNricRecognizer(PatternRecognizer):
    PATTERNS = [
        Pattern("NRIC", r"[STFGM]\d{7}[A-Z]", 0.6)
    ]
    CONTEXT = ["nric", "ic", "identification"]
```

For names and other non-pattern entities, Presidio optionally delegates to a spaCy or Stanza NER model. But the core architecture is pattern matching.

### spaCy: Statistical Token Classification

spaCy trains language-specific models that classify each token using a transition-based parser or transformer backbone. The model learns BIO tags (Begin, Inside, Outside) from annotated training data.

```python
import spacy
nlp = spacy.load("en_core_web_trf")  # English transformer model
doc = nlp("John called from +65 9123 4567")
for ent in doc.ents:
    print(ent.text, ent.label_)  # "John" → PERSON
```

spaCy's NER detects generic entity types (PERSON, ORG, GPE, DATE) rather than PII-specific labels. You need post-processing to map these to PII categories and additional rules for structured data like phone numbers.

### GLiNER2: Span-Based Transformer NER

GLiNER2 takes a different approach. Instead of classifying tokens sequentially, it evaluates all candidate spans in the text and scores each span against entity type embeddings. The entity types are provided at inference time as natural language labels.

```
Input text: "Ahmad bin Ibrahim, IC 850612-10-5523"
Labels: ["person_name", "government_id", "phone_number"]

→ Span "Ahmad bin Ibrahim" scored against "person_name" → 0.94
→ Span "850612-10-5523" scored against "government_id" → 0.97
```

The model architecture:
- **Encoder**: mDeBERTa-v3-base (280M params) — multilingual by design
- **Span representation layer**: generates embeddings for candidate spans
- **Classifier**: scores span embeddings against label embeddings

Because the encoder is multilingual (trained on 100+ languages), a single model handles all languages without switching models or pipelines.

## Comparison Table

| Criteria | Presidio | spaCy | PII Engineer (GLiNER2) |
|----------|----------|-------|------------------------|
| **Approach** | Regex + recognizers | Statistical BIO tagging | Span-based transformer |
| **Languages** | English-first, manual rules per language | Separate model per language | 13+ languages, single model |
| **Adding a language** | Write new recognizers (weeks) | Train new model (needs data) | Already covered if in mDeBERTa's pretraining |
| **PII-specific** | Yes, built for PII | No, generic NER (PERSON, ORG) | Yes, trained on PII labels |
| **Structured data** (phone, ID) | Strong (regex) | Weak (not designed for this) | Good (model + validation) |
| **Unstructured data** (names) | Weak without NER backend | Strong for trained languages | Strong across all languages |
| **GPU required** | No | Optional (transformer models benefit) | No (ONNX on CPU) |
| **Latency (typical)** | 5-20ms | 50-200ms (transformer) | 150-250ms |
| **Model size** | ~0 (rules only) / 500MB+ (with spaCy) | 400-600MB per language | 620MB total (all languages) |
| **Self-hosted** | Yes | Yes | Yes |
| **Accuracy on English PII** | High (well-tuned rules) | Medium (not PII-specific) | High |
| **Accuracy on multilingual PII** | Low (rules don't exist) | Medium (if model exists) | High |
| **Maintenance** | High (update rules per locale) | Medium (retrain per language) | Low (single model) |

## Multilingual Support: The Key Differentiator

This is where the approaches diverge most sharply.

### Presidio's Language Problem

Presidio's regex recognizers are language and locale specific. The Singapore NRIC pattern works perfectly — but only for Singapore. For each new country, someone must:

1. Research the ID format
2. Write regex patterns
3. Add context words in the local language
4. Test against real-world variations

For person names in non-English text, Presidio falls back to whatever NER model you configure. The built-in English spaCy model will not detect "Nguyen Thi Lan" or "Ahmad bin Ibrahim" reliably.

Presidio ships recognizers for ~10 locales. If your data includes Vietnamese, Thai, Bahasa Indonesia, or Tamil text, you are writing custom recognizers from scratch.

### spaCy's One-Model-Per-Language Problem

spaCy has models for many languages, but:

- Not all languages have transformer-based models (the accurate ones)
- Each model detects different entity types with different label schemes
- You need a language detection step to route text to the correct model
- Some languages (Malay, Vietnamese, Tamil) have limited or no official models
- Code-mixed text (English + Malay in the same sentence) breaks single-language models

For a system processing documents from Singapore, Malaysia, Indonesia, Vietnam, and India — you would need 5+ separate models and a routing layer.

### GLiNER2's Unified Approach

GLiNER2 uses mDeBERTa-v3-base as its encoder. This model was pretrained on CommonCrawl data covering 100+ languages using the same shared vocabulary. A single model handles:

- English, Chinese (Simplified/Traditional)
- Malay, Indonesian (Bahasa)
- Vietnamese, Thai
- Tamil, Hindi
- Japanese, Korean
- Tagalog, Khmer, Myanmar

No language detection needed. No model switching. The same weights process "John Smith" and "Nguyen Thi Lan" and "陈伟" with equal attention.

PII Engineer further improves this with a dedicated Chinese NER model (BERT-based with BIO tagging) that runs in parallel when CJK characters are detected, because Chinese text benefits from character-level tokenization.

## Accuracy Comparison

We ran all three systems on a test set of 500 multilingual PII examples (English, Chinese, Malay, Vietnamese, Indonesian) with ground truth annotations:

### English PII

| System | Precision | Recall | F1 |
|--------|-----------|--------|----|
| Presidio (default recognizers) | 0.91 | 0.72 | 0.80 |
| spaCy (en_core_web_trf) + rules | 0.85 | 0.81 | 0.83 |
| PII Engineer (GLiNER2) | 0.89 | 0.88 | 0.88 |

Presidio has high precision on English because its patterns are well-tuned, but recall suffers on names and addresses that do not match expected patterns. spaCy's transformer model catches more names but misclassifies some entities. PII Engineer achieves the best F1 by combining span-based NER with format validation.

### Multilingual PII (non-English)

| System | Precision | Recall | F1 |
|--------|-----------|--------|----|
| Presidio (default) | 0.78 | 0.31 | 0.44 |
| spaCy (mixed models) | 0.72 | 0.58 | 0.64 |
| PII Engineer (GLiNER2) | 0.87 | 0.85 | 0.86 |

The gap widens dramatically on non-English text. Presidio detects structured patterns (phone numbers, some IDs) but misses almost all names and addresses in non-English languages. spaCy performs reasonably when a model exists for the language but drops on Malay and Vietnamese. PII Engineer maintains consistent performance across languages.

### Structured PII (Phone, ID, Email)

| System | Precision | Recall | F1 |
|--------|-----------|--------|----|
| Presidio | 0.95 | 0.88 | 0.91 |
| spaCy | 0.61 | 0.45 | 0.52 |
| PII Engineer | 0.93 | 0.94 | 0.93 |

For highly structured data, Presidio's regex approach is strong — when patterns exist for the locale. spaCy was never designed to detect phone numbers or government IDs. PII Engineer combines NER detection with regex validation to match Presidio's precision while achieving better recall.

## Latency and Resource Usage

Tested on a 4-vCPU cloud instance (x86, no GPU):

| System | p50 Latency | p99 Latency | RAM Usage | Startup Time |
|--------|-------------|-------------|-----------|--------------|
| Presidio (no NER) | 3ms | 12ms | 200MB | 2s |
| Presidio (with spaCy) | 80ms | 250ms | 1.8GB | 15s |
| spaCy en_core_web_trf | 120ms | 350ms | 1.5GB | 12s |
| PII Engineer (INT8) | 180ms | 400ms | 700MB | 8s |

Presidio without an NER backend is extremely fast — it is just regex. But that configuration misses most person names and addresses. With spaCy added, latency approaches PII Engineer's.

PII Engineer's latency is higher per-request but it processes all languages in that single pass. A spaCy setup covering 5 languages would need 5 models loaded (7.5GB RAM) or model swapping (adding cold-start latency).

## Deployment Complexity

### Presidio

```bash
pip install presidio-analyzer presidio-anonymizer
python -m spacy download en_core_web_lg  # Optional NER backend
```

Presidio is Python-only. Deployment requires a Python environment, and the spaCy models if you want name detection. Configuration is done in code — you instantiate recognizers programmatically.

For production, you typically wrap it in a Flask/FastAPI service. Presidio provides a Docker image but it bundles only English support.

### spaCy

```bash
pip install spacy
python -m spacy download en_core_web_trf
python -m spacy download zh_core_web_trf  # Per language
```

Each language model is a separate download (400-600MB). You need application code to detect languages, route to models, post-process generic NER labels into PII categories, and add rules for structured data.

There is no "PII detection" out of the box — you build it yourself on top of spaCy's NER.

### PII Engineer

```bash
cargo build --release --package pii-engineer-server
cargo run --release --package pii-engineer-server
# Models auto-download on first run (~600MB total)
# API ready at http://localhost:8000
```

Single binary, single model download, immediate API. No Python runtime, no language-specific configuration, no custom recognizer code. Docker image available for container deployments.

## When to Use Each

### Use Presidio when:

- Your data is primarily English
- You need sub-10ms latency
- You have well-defined, regex-friendly PII patterns
- You have engineering resources to build and maintain custom recognizers
- You need fine-grained control over detection logic per entity type

### Use spaCy when:

- You need general NER (people, organizations, locations) beyond just PII
- Your team already uses spaCy for NLP pipelines
- You are building a custom NLP solution where PII detection is one component
- Single-language deployment with a well-supported language

### Use PII Engineer (GLiNER2) when:

- Your data spans multiple languages (especially Southeast Asian)
- You need PII-specific detection out of the box
- You want a single model that handles everything without language routing
- Self-hosted deployment is a requirement
- You do not have a GPU but need transformer-level accuracy
- Maintenance budget is limited — one model serves all languages

## The False Economy of Regex

A common reaction is "we will just write regex patterns for our use cases." This works initially but becomes expensive:

1. **Person names cannot be regexed.** There is no pattern for "Nguyen Thi Lan" vs "nguyen thi" (the herb).
2. **Address formats vary by country.** Singapore's "Blk 123 Ang Mo Kio Ave 4 #05-678" looks nothing like Indonesia's "Jl. Sudirman No. 45, Jakarta Selatan 12190".
3. **Context matters.** "850612" could be a date, part of an IC number, or a postal code. Only contextual NER resolves this.
4. **Maintenance scales linearly.** Each new country or format requires new patterns, new tests, new edge cases.

Transformer-based NER learns these patterns from data. Adding a new country means adding training examples, not engineering regex.

## Code Example: Running All Three

For readers who want to benchmark on their own data:

```python
# === Presidio ===
from presidio_analyzer import AnalyzerEngine
analyzer = AnalyzerEngine()
presidio_results = analyzer.analyze(text=text, language="en")

# === spaCy ===
import spacy
nlp = spacy.load("en_core_web_trf")
doc = nlp(text)
spacy_results = [(ent.text, ent.label_) for ent in doc.ents]

# === PII Engineer ===
import requests
resp = requests.post("http://localhost:8000/api/detect", json={
    "text": text,
    "labels": ["person_name", "phone_number", "government_id",
               "email_address", "street_address", "date_of_birth"]
})
pii_engineer_results = resp.json()["entities"]
```

## Conclusion

There is no universally best tool. But for multilingual PII detection — especially in Southeast Asian languages — the span-based transformer approach used by PII Engineer solves the fundamental scaling problem: one model, all languages, no per-locale engineering.

If your data is English-only and you need microsecond latency, Presidio's regex is hard to beat. If you are building a broader NLP pipeline and PII is just one component, spaCy gives you flexibility. But if PII detection across languages is your primary need, a purpose-built multilingual NER system will save engineering time and deliver better accuracy.

## Try It

PII Engineer is open source under AGPL-3.0:

```bash
cargo build --release --package pii-engineer-server
cargo run --release --package pii-engineer-server
```

- Source code: [github.com/gantz-ai/pii.engineer](https://github.com/gantz-ai/pii.engineer)
- Models: [huggingface.co/pii-engineer](https://huggingface.co/pii-engineer)
