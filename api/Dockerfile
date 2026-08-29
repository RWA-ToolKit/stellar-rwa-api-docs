# Multi-stage build for the Stellar RWA API.
#
# Base image digests are pinned for reproducible builds. To refresh them
# after an intentional version bump, resolve the new digest with:
#   docker buildx imagetools inspect rust:1.83-slim
#   docker buildx imagetools inspect debian:bookworm-slim
FROM rust:1.83-slim@sha256:540c902e99c384163b688bbd8b5b8520e94e7731b27f7bd0eaa56ae1960627ab AS builder

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

FROM debian:bookworm-slim@sha256:88200866dfff7ea7f5cbcb6ec7c8a701889efe6fe859fe64d6990e4b07ea4171 AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd -m -u 1000 app

WORKDIR /app
COPY --from=builder /app/target/release/stellar-rwa-api /usr/local/bin/stellar-rwa-api
RUN chown app:app /app

ENV PORT=8080
EXPOSE 8080
USER app
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD wget --quiet --tries=1 --spider http://localhost:8080/health || exit 1
CMD ["stellar-rwa-api"]
