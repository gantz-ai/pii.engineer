---
title: "Building a Data Loss Prevention Pipeline with PII Engineer"
date: "2026-05"
tag: "Architecture"
description: "How to integrate PII Engineer into a DLP pipeline — pre-commit hooks for code repositories, CI/CD scanning, S3 bucket auditing, and database field classification."
---

## PII Leaks Are a Pipeline Problem

Data loss prevention isn't a single tool — it's a set of checkpoints across your data pipeline. Personal data leaks through code commits, log files, cloud storage, database dumps, and API responses. Each leak point needs automated detection.

PII Engineer fits into this pipeline as the detection engine. It doesn't replace your DLP platform — it provides the NER capability that regex-based DLP tools miss, especially for multilingual text and non-standard PII formats.

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                    DLP Pipeline                             │
│                                                             │
│  ┌──────────┐   ┌──────────┐   ┌──────────┐   ┌──────────┐  │
│  │ Pre-     │   │ CI/CD    │   │ Storage  │   │ Database │  │
│  │ Commit   │   │ Pipeline │   │ Audit    │   │ Scanner  │  │
│  │ Hook     │   │ Stage    │   │ (S3/GCS) │   │          │  │
│  └────┬─────┘   └────┬─────┘   └────┬─────┘   └────┬─────┘  │
│       │              │              │              │        │
│       └──────────────┴──────────────┴──────────────┘        │
│                          │                                  │
│                ┌─────────▼──────────┐                       │
│                │  PII Engineer API  │                       │
│                │  (localhost:8000)  │                       │
│                └────────────────────┘                       │
└─────────────────────────────────────────────────────────────┘
```

## 1. Pre-Commit Hook: Block PII Before It Enters the Repo

The cheapest place to catch PII is before it's committed. A pre-commit hook scans staged files and blocks commits containing personal data.

### Setup

Create `.git/hooks/pre-commit`:

```bash
#!/bin/bash

PII_API="http://localhost:8000/api/detect"
FOUND_PII=0

# Scan staged files (text files only, skip binaries)
for file in $(git diff --cached --name-only --diff-filter=ACM); do
    # Skip binary files and common non-text files
    case "$file" in
        *.png|*.jpg|*.gif|*.ico|*.woff|*.woff2|*.ttf|*.lock) continue ;;
    esac

    content=$(git show ":$file" 2>/dev/null)
    [ -z "$content" ] && continue

    # Skip files larger than 50KB
    size=$(echo -n "$content" | wc -c)
    [ "$size" -gt 50000 ] && continue

    response=$(curl -sf "$PII_API" \
        -H "Content-Type: application/json" \
        -d "$(jq -n --arg text "$content" '{text: $text}')" 2>/dev/null)

    if [ $? -ne 0 ]; then
        echo "WARNING: PII Engineer not running, skipping PII check"
        exit 0
    fi

    entities=$(echo "$response" | jq '.entities | length')
    if [ "$entities" -gt 0 ]; then
        echo "PII DETECTED in $file:"
        echo "$response" | jq -r '.entities[] | "  [\(.type)] \(.value) (confidence: \(.score))"'
        FOUND_PII=1
    fi
done

if [ "$FOUND_PII" -eq 1 ]; then
    echo ""
    echo "COMMIT BLOCKED: PII detected in staged files."
    echo "Remove the personal data or add the file to .piiignore"
    exit 1
fi
```

### Ignore File

Create `.piiignore` for files that legitimately contain PII patterns (test fixtures, documentation):

```
# Test fixtures with fake PII
tests/fixtures/*.json
tests/data/*.txt

# Documentation with example PII
docs/api-examples.md

# Seed data (uses generated fake data)
scripts/seed_data.sql
```

Update the hook to respect `.piiignore`:

```bash
# At the top of the pre-commit hook
IGNORE_FILE=".piiignore"
should_ignore() {
    [ ! -f "$IGNORE_FILE" ] && return 1
    while IFS= read -r pattern; do
        [[ "$pattern" =~ ^#.*$ ]] && continue
        [[ -z "$pattern" ]] && continue
        [[ "$1" == $pattern ]] && return 0
    done < "$IGNORE_FILE"
    return 1
}

# In the file loop
for file in $(git diff --cached --name-only --diff-filter=ACM); do
    should_ignore "$file" && continue
    # ... rest of scanning logic
done
```

## 2. CI/CD Pipeline Stage: Scan on Every Push

Add a PII scan stage to your CI pipeline. This catches PII that bypasses local hooks (force pushes, GUI commits, new team members without hooks installed).

### GitHub Actions

```yaml
name: PII Scan

on: [push, pull_request]

jobs:
  pii-scan:
    runs-on: ubuntu-latest
    services:
      pii-engineer:
        image: ghcr.io/gantz-ai/pii-engineer:latest
        ports:
          - 8000:8000
        options: --health-cmd "curl -f http://localhost:8000/api/health" --health-interval 10s --health-timeout 5s --health-retries 12 --health-start-period 120s

    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 2

      - name: Scan changed files for PII
        run: |
          FOUND=0
          for file in $(git diff --name-only HEAD~1 HEAD -- '*.py' '*.js' '*.ts' '*.json' '*.yaml' '*.yml' '*.txt' '*.md' '*.csv'); do
            [ ! -f "$file" ] && continue

            response=$(curl -sf http://localhost:8000/api/detect \
              -H "Content-Type: application/json" \
              -d "$(jq -n --arg text "$(cat "$file")" '{text: $text}')")

            count=$(echo "$response" | jq '.entities | length')
            if [ "$count" -gt 0 ]; then
              echo "::error file=$file::PII detected: $count entities found"
              echo "$response" | jq '.entities[] | "  [\(.type)] \(.value)"'
              FOUND=1
            fi
          done
          [ "$FOUND" -eq 1 ] && exit 1
          echo "No PII detected in changed files"
```

### GitLab CI

```yaml
pii-scan:
  stage: test
  services:
    - name: ghcr.io/gantz-ai/pii-engineer:latest
      alias: pii-engineer
  script:
    - |
      for file in $(git diff --name-only HEAD~1 HEAD); do
        [ ! -f "$file" ] && continue
        response=$(curl -sf http://pii-engineer:8000/api/detect \
          -H "Content-Type: application/json" \
          -d "{\"text\": $(jq -Rs . < "$file")}")
        count=$(echo "$response" | jq '.entities | length')
        if [ "$count" -gt 0 ]; then
          echo "PII found in $file: $count entities"
          exit 1
        fi
      done
```

## 3. S3 Bucket Auditing: Find PII in Cloud Storage

Cloud storage buckets accumulate files over time — CSV exports, log dumps, database backups, uploaded documents. Scan them systematically:

```python
import boto3
import requests
import json
from datetime import datetime

def audit_s3_bucket(bucket_name: str, prefix: str = "", max_file_size_mb: int = 10) -> list[dict]:
    """Scan S3 objects for PII."""
    s3 = boto3.client("s3")
    findings = []

    paginator = s3.get_paginator("list_objects_v2")
    for page in paginator.paginate(Bucket=bucket_name, Prefix=prefix):
        for obj in page.get("Contents", []):
            key = obj["Key"]

            # Skip non-text files
            if not any(key.endswith(ext) for ext in
                       [".txt", ".csv", ".json", ".log", ".md", ".xml", ".html"]):
                continue

            # Skip large files
            if obj["Size"] > max_file_size_mb * 1024 * 1024:
                continue

            # Download and scan
            response = s3.get_object(Bucket=bucket_name, Key=key)
            text = response["Body"].read().decode("utf-8", errors="ignore")

            # Chunk large text
            chunks = [text[i:i+10000] for i in range(0, len(text), 10000)]
            all_entities = []

            for chunk in chunks:
                resp = requests.post("http://localhost:8000/api/detect", json={
                    "text": chunk
                })
                all_entities.extend(resp.json()["entities"])

            if all_entities:
                findings.append({
                    "bucket": bucket_name,
                    "key": key,
                    "size_bytes": obj["Size"],
                    "last_modified": obj["LastModified"].isoformat(),
                    "entities_found": len(all_entities),
                    "entity_types": list(set(e["type"] for e in all_entities)),
                    "sample_entities": all_entities[:5]
                })

    return findings

# Run audit
findings = audit_s3_bucket("my-data-bucket", prefix="exports/")
print(f"Found PII in {len(findings)} files")
for f in findings:
    print(f"  {f['key']}: {f['entities_found']} entities ({', '.join(f['entity_types'])})")
```

### Scheduled Auditing

Run bucket audits on a schedule with a cron job or Lambda:

```python
def generate_audit_report(buckets: list[str]) -> dict:
    """Generate a PII audit report across multiple buckets."""
    report = {
        "timestamp": datetime.utcnow().isoformat(),
        "buckets_scanned": len(buckets),
        "total_files_with_pii": 0,
        "findings_by_bucket": {}
    }

    for bucket in buckets:
        findings = audit_s3_bucket(bucket)
        report["findings_by_bucket"][bucket] = {
            "files_with_pii": len(findings),
            "total_entities": sum(f["entities_found"] for f in findings),
            "entity_types": list(set(
                t for f in findings for t in f["entity_types"]
            )),
            "files": findings
        }
        report["total_files_with_pii"] += len(findings)

    return report
```

## 4. Database Field Classification

Scan database contents to discover which columns contain PII — useful for data cataloging and access control:

```python
import psycopg2

def classify_database_fields(conn_string: str, sample_size: int = 100) -> list[dict]:
    """Sample database tables and classify fields by PII content."""
    conn = psycopg2.connect(conn_string)
    cur = conn.cursor()

    # Get all tables
    cur.execute("""
        SELECT table_name, column_name, data_type
        FROM information_schema.columns
        WHERE table_schema = 'public'
        AND data_type IN ('text', 'character varying', 'varchar')
        ORDER BY table_name, ordinal_position
    """)
    columns = cur.fetchall()

    classifications = []

    for table, column, dtype in columns:
        # Sample non-null values
        cur.execute(f'SELECT "{column}" FROM "{table}" WHERE "{column}" IS NOT NULL LIMIT %s',
                    (sample_size,))
        values = [row[0] for row in cur.fetchall() if row[0] and len(str(row[0])) > 3]

        if not values:
            continue

        # Combine samples for batch detection
        combined = "\n".join(str(v) for v in values[:50])
        resp = requests.post("http://localhost:8000/api/detect", json={
            "text": combined
        })
        entities = resp.json()["entities"]

        if entities:
            type_counts = {}
            for e in entities:
                type_counts[e["type"]] = type_counts.get(e["type"], 0) + 1

            dominant_type = max(type_counts, key=type_counts.get)
            pii_ratio = len(entities) / len(values)

            classifications.append({
                "table": table,
                "column": column,
                "samples_checked": len(values),
                "entities_found": len(entities),
                "pii_ratio": round(pii_ratio, 2),
                "dominant_type": dominant_type,
                "type_distribution": type_counts,
                "classification": "PII" if pii_ratio > 0.3 else "POSSIBLE_PII" if pii_ratio > 0.1 else "UNLIKELY_PII"
            })

    conn.close()
    return classifications
```

Output:

```
Table: customers, Column: full_name
  Classification: PII (ratio: 0.95, type: person_name)

Table: customers, Column: national_id
  Classification: PII (ratio: 0.88, type: government_id)

Table: orders, Column: shipping_address
  Classification: PII (ratio: 0.72, type: street_address)

Table: orders, Column: notes
  Classification: POSSIBLE_PII (ratio: 0.15, type: person_name)
```

## 5. Log File Scanning

Application logs frequently contain PII that was accidentally logged — request bodies, error messages with user data, debug output:

```python
def scan_log_file(filepath: str, batch_size: int = 50) -> dict:
    """Scan a log file for PII leaks."""
    findings = []
    line_batch = []

    with open(filepath, 'r', encoding='utf-8', errors='ignore') as f:
        for line_num, line in enumerate(f, 1):
            line = line.strip()
            if len(line) < 20:
                continue
            line_batch.append((line_num, line))

            if len(line_batch) >= batch_size:
                combined = "\n".join(l[1] for l in line_batch)
                resp = requests.post("http://localhost:8000/api/detect", json={
                    "text": combined
                })
                if resp.json()["entities"]:
                    findings.append({
                        "line_range": f"{line_batch[0][0]}-{line_batch[-1][0]}",
                        "entities": resp.json()["entities"]
                    })
                line_batch = []

    return {
        "file": filepath,
        "findings": len(findings),
        "total_entities": sum(len(f["entities"]) for f in findings),
        "details": findings
    }
```

## Putting It All Together

A complete DLP pipeline combines all these checkpoints:

| Checkpoint      | When                        | What It Catches                         | Action         |
| --------------- | --------------------------- | --------------------------------------- | -------------- |
| Pre-commit hook | Before code is committed    | PII in source code, configs, test data  | Block commit   |
| CI/CD stage     | On every push/PR            | PII that bypassed local hooks           | Fail the build |
| S3 audit        | Weekly/monthly cron         | PII in exported files, uploads, backups | Alert + report |
| Database scan   | Monthly or on schema change | PII in unexpected columns               | Classify + tag |
| Log scanning    | Daily cron                  | PII leaked in application logs          | Alert + redact |

Each checkpoint calls the same PII Engineer API. The detection logic is centralized — update the model once, all checkpoints benefit.

## Source Code

PII Engineer is open source under AGPL-3.0:

- Repository: [github.com/gantz-ai/pii.engineer](https://github.com/gantz-ai/pii.engineer)
- Models: [huggingface.co/pii-engineer](https://huggingface.co/pii-engineer)
