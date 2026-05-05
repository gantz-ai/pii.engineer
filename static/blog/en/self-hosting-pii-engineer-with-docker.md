---
title: "Self-Hosting PII Engineer with Docker"
date: "2026-05"
tag: "Tutorial"
description: "A step-by-step guide to deploying PII Engineer on your own infrastructure using Docker — from single-container setup to production-ready docker-compose with health checks and persistent model caching."
---

## Why Self-Host?

PII Engineer processes sensitive data by design. Every text you send through the API potentially contains names, government IDs, phone numbers, and addresses. Self-hosting means that data never leaves your network — no third-party processors, no cloud dependencies, no BAAs to negotiate.

The server is a single Rust binary with an embedded ONNX runtime. No Python, no pip, no virtual environments. Docker simplifies deployment to a single command.

## Quick Start: Single Container

Pull and run:

```bash
docker run -d \
  --name pii-engineer \
  -p 8000:8000 \
  -v pii-models:/root/.cache/huggingface \
  ghcr.io/gantz-ai/pii-engineer:latest
```

First startup takes 1-2 minutes as the model downloads (~600MB) from HuggingFace. The volume mount caches the model so subsequent restarts are instant.

Test the API:

```bash
curl -s http://localhost:8000/api/health | jq
```

```json
{
  "status": "ok",
  "gliner_loaded": true,
  "chinese_loaded": true
}
```

## Docker Compose for Production

A production deployment needs health checks, resource limits, restart policies, and proper logging:

```yaml
version: "3.8"

services:
  pii-engineer:
    image: ghcr.io/gantz-ai/pii-engineer:latest
    container_name: pii-engineer
    restart: unless-stopped
    ports:
      - "127.0.0.1:8000:8000"
    volumes:
      - model-cache:/root/.cache/huggingface
    environment:
      - RUST_LOG=info
      - PII_MAX_TEXT_LENGTH=50000
      - PII_RAW_THRESHOLD=0.3
      - PII_REVIEW_THRESHOLD=0.5
    deploy:
      resources:
        limits:
          memory: 2G
          cpus: "4"
        reservations:
          memory: 1G
          cpus: "2"
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8000/api/health"]
      interval: 30s
      timeout: 5s
      retries: 3
      start_period: 120s

volumes:
  model-cache:
```

Key decisions:

- **Port binding to 127.0.0.1** — the API has no authentication by design. Bind to localhost and put a reverse proxy with auth in front.
- **2GB memory limit** — the model uses ~700MB in memory. 2GB gives comfortable headroom for concurrent requests.
- **start_period: 120s** — the model download and warmup takes time on first run. Don't let the health check kill the container during startup.

## Environment Variables

| Variable | Default | Description |
|---|---|---|
| `RUST_LOG` | `info` | Log level: `error`, `warn`, `info`, `debug`, `trace` |
| `PII_MAX_TEXT_LENGTH` | `50000` | Maximum input text length in bytes |
| `PII_RAW_THRESHOLD` | `0.3` | Minimum raw model confidence to consider an entity |
| `PII_REVIEW_THRESHOLD` | `0.5` | Entities below this score are flagged `needs_review: true` |
| `PII_AUTO_REDACT_THRESHOLD` | `0.65` | Entities above this score are auto-redacted |
| `PII_ENGINEER_STATIC_DIR` | `static` | Path to static files (web UI, blog) |

## Reverse Proxy with Nginx

Add authentication and TLS in front of the API:

```nginx
server {
    listen 443 ssl http2;
    server_name pii.internal.company.com;

    ssl_certificate     /etc/ssl/certs/pii.crt;
    ssl_certificate_key /etc/ssl/private/pii.key;

    location /api/ {
        auth_basic "PII Engineer";
        auth_basic_user_file /etc/nginx/.htpasswd;

        proxy_pass http://127.0.0.1:8000;
        proxy_read_timeout 30s;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
    }

    location / {
        proxy_pass http://127.0.0.1:8000;
    }
}
```

## Scaling with Multiple Instances

The server is stateless. Scale horizontally by running multiple containers behind a load balancer:

```yaml
services:
  pii-engineer-1:
    image: ghcr.io/gantz-ai/pii-engineer:latest
    ports: ["127.0.0.1:8001:8000"]
    volumes: [model-cache:/root/.cache/huggingface]

  pii-engineer-2:
    image: ghcr.io/gantz-ai/pii-engineer:latest
    ports: ["127.0.0.1:8002:8000"]
    volumes: [model-cache:/root/.cache/huggingface]

  pii-engineer-3:
    image: ghcr.io/gantz-ai/pii-engineer:latest
    ports: ["127.0.0.1:8003:8000"]
    volumes: [model-cache:/root/.cache/huggingface]
```

Each instance loads the model independently (~700MB each). On a 16GB machine, you can comfortably run 4 instances for ~16 requests/second at ~250ms latency.

## Building from Source

If you prefer to build the Docker image yourself:

```bash
git clone https://github.com/gantz-ai/pii.engineer.git
cd pii.engineer
docker build -t pii-engineer:local .
```

The Dockerfile uses a multi-stage build — Rust compilation in a builder stage, final image is a minimal runtime with just the binary and ONNX libraries.

## Air-Gapped Deployment

For environments with no internet access (government, healthcare, defense), pre-download the model:

```bash
# On a machine with internet
docker run --rm -v pii-models:/root/.cache/huggingface \
  ghcr.io/gantz-ai/pii-engineer:latest \
  echo "Model downloaded"

# Export the volume
docker run --rm -v pii-models:/data -v $(pwd):/backup \
  alpine tar czf /backup/pii-models.tar.gz -C /data .

# Transfer pii-models.tar.gz to air-gapped machine
# Import the volume
docker volume create pii-models
docker run --rm -v pii-models:/data -v $(pwd):/backup \
  alpine tar xzf /backup/pii-models.tar.gz -C /data

# Run without internet
docker run -d --network none \
  -v pii-models:/root/.cache/huggingface \
  -p 8000:8000 \
  pii-engineer:local
```

The `--network none` flag ensures the container has zero network access — the model runs entirely from local disk.

## Monitoring

The `/api/health` endpoint returns model load status. Integrate with your monitoring stack:

```bash
# Prometheus-style check
curl -sf http://localhost:8000/api/health | jq -e '.gliner_loaded == true' > /dev/null
echo $?  # 0 = healthy, 1 = unhealthy
```

Container logs output structured request information at `info` level:

```
INFO pii_engineer_server: listening on 0.0.0.0:8000
INFO pii_engineer_server: detect request text_length=1234 entities_found=5 duration_ms=187
```

Set `RUST_LOG=debug` for per-entity scoring details during troubleshooting.

## Resource Requirements

| Deployment | CPU | Memory | Throughput |
|---|---|---|---|
| Minimum | 2 vCPU | 1.5 GB | ~2 req/s |
| Recommended | 4 vCPU | 2 GB | ~4 req/s |
| High throughput | 4× instances, 16 vCPU | 8 GB | ~16 req/s |

No GPU required. The ONNX runtime uses CPU inference with optimized threading. More cores = lower latency per request and higher concurrent throughput.

## Source Code

PII Engineer is open source under Apache-2.0:

- Repository: [github.com/gantz-ai/pii.engineer](https://github.com/gantz-ai/pii.engineer)
- Models: [huggingface.co/pii-engineer](https://huggingface.co/pii-engineer)
