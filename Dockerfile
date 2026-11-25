# ── Builder ──────────────────────────────────────────────────────────────────
# Match the Rust 1.88.0 pin in rust-toolchain.toml and CI.
# Cargo.lock has been tracked since the CI toolchain fix.
FROM rust:1.88.0-slim-bookworm AS builder
WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY migrations ./migrations
COPY src ./src
RUN cargo build --locked --release

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
