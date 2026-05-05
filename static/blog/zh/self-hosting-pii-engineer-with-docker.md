---
title: "使用 Docker 自托管 PII Engineer"
date: "2026-05"
tag: "Tutorial"
description: "部署 PII Engineer 到您自己的基础设施的分步指南——从单容器部署到带有健康检查和持久模型缓存的生产就绪 docker-compose 配置。"
---

## 为什么要自托管？

PII Engineer 在设计上就是用来处理敏感数据的。您通过 API 发送的每段文本都可能包含姓名、政府证件号、电话号码和地址。自托管意味着这些数据永远不会离开您的网络——没有第三方处理者，没有云依赖，无需协商 BAA 协议。

服务器是一个内嵌 ONNX 运行时的 Rust 二进制文件。无需 Python、pip 或虚拟环境。Docker 将部署简化为一条命令。

## 快速开始：单容器

拉取并运行：

```bash
docker run -d \
  --name pii-engineer \
  -p 8000:8000 \
  -v pii-models:/root/.cache/huggingface \
  ghcr.io/gantz-ai/pii-engineer:latest
```

首次启动需要 1-2 分钟下载模型（约 600MB），来源为 HuggingFace。卷挂载会缓存模型，后续重启将即时完成。

测试 API：

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

## 生产环境的 Docker Compose

生产部署需要健康检查、资源限制、重启策略和适当的日志记录：

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

关键决策：

- **端口绑定到 127.0.0.1** —— API 在设计上没有身份验证。绑定到 localhost，并在前面放置带有身份验证的反向代理。
- **2GB 内存限制** —— 模型使用约 700MB 内存。2GB 为并发请求提供了充足的余量。
- **start_period: 120s** —— 首次运行时模型下载和预热需要时间。不要让健康检查在启动期间杀死容器。

## 环境变量

| 变量 | 默认值 | 说明 |
|---|---|---|
| `RUST_LOG` | `info` | 日志级别：`error`、`warn`、`info`、`debug`、`trace` |
| `PII_MAX_TEXT_LENGTH` | `50000` | 输入文本最大长度（字节） |
| `PII_RAW_THRESHOLD` | `0.3` | 考虑实体的最低原始模型置信度 |
| `PII_REVIEW_THRESHOLD` | `0.5` | 低于此分数的实体标记为 `needs_review: true` |
| `PII_AUTO_REDACT_THRESHOLD` | `0.65` | 高于此分数的实体自动脱敏 |
| `PII_ENGINEER_STATIC_DIR` | `static` | 静态文件路径（Web UI、博客） |

## 使用 Nginx 反向代理

在 API 前面添加身份验证和 TLS：

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

## 多实例扩展

服务器是无状态的。通过在负载均衡器后面运行多个容器进行水平扩展：

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

每个实例独立加载模型（每个约 700MB）。在 16GB 的机器上，您可以轻松运行 4 个实例，以约 250ms 延迟达到约 16 请求/秒的吞吐量。

## 从源码构建

如果您更喜欢自己构建 Docker 镜像：

```bash
git clone https://github.com/gantz-ai/pii.engineer.git
cd pii.engineer
docker build -t pii-engineer:local .
```

Dockerfile 使用多阶段构建——在构建阶段进行 Rust 编译，最终镜像是仅包含二进制文件和 ONNX 库的最小运行时。

## 离线环境部署

对于没有互联网访问的环境（政府、医疗、国防），预先下载模型：

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

`--network none` 标志确保容器完全没有网络访问——模型完全从本地磁盘运行。

## 监控

`/api/health` 端点返回模型加载状态。与您的监控系统集成：

```bash
# Prometheus-style check
curl -sf http://localhost:8000/api/health | jq -e '.gliner_loaded == true' > /dev/null
echo $?  # 0 = healthy, 1 = unhealthy
```

容器日志以 `info` 级别输出结构化的请求信息：

```
INFO pii_engineer_server: listening on 0.0.0.0:8000
INFO pii_engineer_server: detect request text_length=1234 entities_found=5 duration_ms=187
```

设置 `RUST_LOG=debug` 可在排错时查看每个实体的评分详情。

## 资源需求

| 部署方案 | CPU | 内存 | 吞吐量 |
|---|---|---|---|
| 最低配置 | 2 vCPU | 1.5 GB | ~2 req/s |
| 推荐配置 | 4 vCPU | 2 GB | ~4 req/s |
| 高吞吐量 | 4 实例，16 vCPU | 8 GB | ~16 req/s |

无需 GPU。ONNX 运行时使用优化线程的 CPU 推理。更多核心 = 每个请求更低的延迟和更高的并发吞吐量。

## 源代码

PII Engineer 在 AGPL-3.0 下开源：

- 仓库：[github.com/gantz-ai/pii.engineer](https://github.com/gantz-ai/pii.engineer)
- 模型：[huggingface.co/pii-engineer](https://huggingface.co/pii-engineer)
