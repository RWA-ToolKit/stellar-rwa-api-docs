# Multi-stage build for the Stellar RWA API.
FROM rust:1.83-slim AS builder

# Install cargo-chef for efficient dependency caching.
RUN cargo install cargo-chef
WORKDIR /app

# Prepare dependency recipe from manifests.
COPY Cargo.toml Cargo.lock ./
RUN cargo chef prepare --recipe-path recipe.json

# Cache dependencies separately from source.
RUN cargo chef cook --release --locked --recipe-path recipe.json

# Build application with source code.
COPY src ./src
RUN cargo build --release --locked

FROM debian:bookworm-slim AS runtime
# curl is required by HEALTHCHECK below — debian:bookworm-slim ships neither
# curl nor wget by default.
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/* \
    && useradd -m -u 1000 app

WORKDIR /app
COPY --from=builder /app/target/release/stellar-rwa-api /usr/local/bin/stellar-rwa-api
RUN chown app:app /app

ENV PORT=8080
EXPOSE 8080
USER app
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl --fail --silent --show-error http://localhost:8080/health || exit 1
CMD ["stellar-rwa-api"]
