# ---------------------------------------------------------------------------
# Builder stage
# ---------------------------------------------------------------------------
FROM rust:1.78-bookworm AS builder

WORKDIR /build

# Copy the full workspace source
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/

# Build the server binary in release mode
RUN cargo build --release --package pii-engineer-server

# ---------------------------------------------------------------------------
# Final stage
# ---------------------------------------------------------------------------
FROM debian:bookworm-slim

RUN apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates libssl3 && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the compiled binary
COPY --from=builder /build/target/release/pii-engineer-server /app/pii-engineer-server

# Copy frontend assets
COPY static/ /app/static/

# Copy ONNX Runtime shared library
COPY lib/ /app/lib/

# Point ort to the bundled shared library
ENV ORT_DYLIB_PATH=/app/lib/libonnxruntime.so

# Models are mounted by the user at runtime (e.g. -v ./models:/app/models)
ENV GLINER_MODELS=models/PII-Engineer-Multi-NER-v2.1
ENV CHINESE_NER_MODEL=models/PII-Engineer-Chinese-NER-v1.0

EXPOSE 8000

CMD ["/app/pii-engineer-server"]
