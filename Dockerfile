# ── Builder ──────────────────────────────────────────────────────────────────
FROM rust:1.82-slim-bookworm AS builder
WORKDIR /app

# Pre-cache dependencies (only re-runs when Cargo manifests change).
COPY Cargo.toml ./
COPY Cargo.lock* ./
RUN mkdir src && echo 'fn main() {}' > src/main.rs \
    && cargo build --release \
    && rm -rf src

# Build the real binary.
COPY migrations ./migrations
COPY src ./src
RUN cargo build --release

# ── Runtime ──────────────────────────────────────────────────────────────────
FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app

COPY --from=builder /app/target/release/asistente-comercial /usr/local/bin/asistente-comercial
COPY config ./config

ENV BIND_ADDR=0.0.0.0:8080
EXPOSE 8080
CMD ["asistente-comercial"]
