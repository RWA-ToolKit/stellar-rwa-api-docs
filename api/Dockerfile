# Multi-stage build for the Stellar RWA API.
FROM rust:1.83-slim AS builder
WORKDIR /app

# Cache dependencies separately from source.
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs && cargo build --release --locked && rm -rf src

COPY src ./src
RUN touch src/main.rs && cargo build --release --locked

FROM debian:bookworm-slim AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/stellar-rwa-api /usr/local/bin/stellar-rwa-api

ENV PORT=8080
EXPOSE 8080
CMD ["stellar-rwa-api"]
