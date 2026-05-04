#!/bin/bash
set -e

APP_DIR=/opt/pii-engineer
LOG=/var/log/pii-engineer-deploy.log

exec >> "$LOG" 2>&1
echo "=== $(date -u) deploy start ==="

cd "$APP_DIR"
git fetch --all --quiet
git reset --hard origin/main

cargo build --release --package pii-engineer-server --quiet

if [ ! -d models/PII-Engineer-Multi-NER-v2.1/onnx ]; then
    echo "MISSING model files — download from HuggingFace or let the server auto-download on first run."
    exit 1
fi

systemctl restart pii-engineer
echo "=== $(date -u) deploy done ==="
