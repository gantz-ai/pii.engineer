---
title: "Tự Triển Khai PII Engineer với Docker"
date: "2026-05"
tag: "Tutorial"
description: "Hướng dẫn từng bước triển khai PII Engineer trên hạ tầng riêng bằng Docker — từ cài đặt container đơn lẻ đến cấu hình docker-compose sẵn sàng cho production với health check và cache model lâu dài."
---

## Tại Sao Nên Tự Triển Khai?

PII Engineer xử lý dữ liệu nhạy cảm theo thiết kế. Mỗi đoạn văn bản bạn gửi qua API đều có thể chứa tên, số chứng minh nhân dân, số điện thoại và địa chỉ. Tự triển khai đồng nghĩa với việc dữ liệu không bao giờ rời khỏi mạng nội bộ của bạn — không có bên xử lý thứ ba, không phụ thuộc cloud, không cần đàm phán BAA.

Server là một file binary Rust duy nhất với ONNX runtime tích hợp. Không có Python, không pip, không virtual environment. Docker đơn giản hóa việc triển khai chỉ với một lệnh duy nhất.

## Khởi Động Nhanh: Container Đơn Lẻ

Pull và chạy:

```bash
docker run -d \
  --name pii-engineer \
  -p 8000:8000 \
  -v pii-models:/root/.cache/huggingface \
  ghcr.io/gantz-ai/pii-engineer:latest
```

Lần khởi động đầu tiên mất 1-2 phút khi model được tải xuống (~600MB) từ HuggingFace. Volume mount sẽ cache model để các lần khởi động lại sau đó diễn ra ngay lập tức.

Kiểm tra API:

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

## Docker Compose cho Production

Triển khai production cần health check, giới hạn tài nguyên, chính sách restart và logging phù hợp:

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

Các quyết định quan trọng:

- **Bind port vào 127.0.0.1** — API không có authentication theo thiết kế. Bind vào localhost và đặt reverse proxy có auth phía trước.
- **Giới hạn bộ nhớ 2GB** — model sử dụng ~700MB trong bộ nhớ. 2GB cho đủ dư địa cho các request đồng thời.
- **start_period: 120s** — việc tải model và khởi động cần thời gian ở lần chạy đầu. Đừng để health check kill container trong quá trình khởi động.

## Biến Môi Trường

| Biến | Mặc định | Mô tả |
|---|---|---|
| `RUST_LOG` | `info` | Mức log: `error`, `warn`, `info`, `debug`, `trace` |
| `PII_MAX_TEXT_LENGTH` | `50000` | Độ dài tối đa của văn bản đầu vào tính bằng byte |
| `PII_RAW_THRESHOLD` | `0.3` | Độ tin cậy tối thiểu từ model để xem xét một entity |
| `PII_REVIEW_THRESHOLD` | `0.5` | Các entity dưới ngưỡng này được đánh dấu `needs_review: true` |
| `PII_AUTO_REDACT_THRESHOLD` | `0.65` | Các entity trên ngưỡng này được tự động che giấu |
| `PII_ENGINEER_STATIC_DIR` | `static` | Đường dẫn đến file tĩnh (web UI, blog) |

## Reverse Proxy với Nginx

Thêm authentication và TLS phía trước API:

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

## Mở Rộng Quy Mô với Nhiều Instance

Server là stateless. Mở rộng theo chiều ngang bằng cách chạy nhiều container phía sau load balancer:

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

Mỗi instance tải model độc lập (~700MB mỗi cái). Trên máy 16GB, bạn có thể chạy thoải mái 4 instance với throughput ~16 request/giây ở độ trễ ~250ms.

## Build từ Source Code

Nếu bạn muốn tự build Docker image:

```bash
git clone https://github.com/gantz-ai/pii.engineer.git
cd pii.engineer
docker build -t pii-engineer:local .
```

Dockerfile sử dụng multi-stage build — biên dịch Rust trong builder stage, image cuối cùng là runtime tối giản chỉ chứa binary và thư viện ONNX.

## Triển Khai Không Có Internet (Air-Gapped)

Cho các môi trường không có truy cập internet (chính phủ, y tế, quốc phòng), tải trước model:

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

Cờ `--network none` đảm bảo container không có bất kỳ truy cập mạng nào — model chạy hoàn toàn từ ổ đĩa cục bộ.

## Giám Sát

Endpoint `/api/health` trả về trạng thái tải model. Tích hợp với hệ thống giám sát của bạn:

```bash
# Prometheus-style check
curl -sf http://localhost:8000/api/health | jq -e '.gliner_loaded == true' > /dev/null
echo $?  # 0 = healthy, 1 = unhealthy
```

Log container xuất thông tin request có cấu trúc ở mức `info`:

```
INFO pii_engineer_server: listening on 0.0.0.0:8000
INFO pii_engineer_server: detect request text_length=1234 entities_found=5 duration_ms=187
```

Đặt `RUST_LOG=debug` để xem chi tiết điểm số từng entity khi troubleshooting.

## Yêu Cầu Tài Nguyên

| Cấu hình triển khai | CPU | Bộ nhớ | Throughput |
|---|---|---|---|
| Tối thiểu | 2 vCPU | 1.5 GB | ~2 req/s |
| Khuyến nghị | 4 vCPU | 2 GB | ~4 req/s |
| Throughput cao | 4x instance, 16 vCPU | 8 GB | ~16 req/s |

Không cần GPU. ONNX runtime sử dụng CPU inference với threading tối ưu. Nhiều core hơn = độ trễ thấp hơn cho mỗi request và throughput đồng thời cao hơn.

## Mã Nguồn

PII Engineer là mã nguồn mở theo giấy phép Apache-2.0:

- Repository: [github.com/gantz-ai/pii.engineer](https://github.com/gantz-ai/pii.engineer)
- Models: [huggingface.co/pii-engineer](https://huggingface.co/pii-engineer)
