# Stellar RWA — API + Docs

A combined repository with two projects for the **Stellar RWA Toolkit**:

- **`api/`** — a Rust/Axum REST API that indexes all tokenized real-world asset
  activity on Stellar (maintainer-only).
- **`docs/`** — a Next.js + MDX documentation site covering the whole platform
  (open to contributions — see [CONTRIBUTING.md](CONTRIBUTING.md)).

## Sister repositories

- **Contracts:** https://github.com/RWA-ToolKit/stellar-rwa-contracts
- **Web app:** https://github.com/RWA-ToolKit/stellar-rwa-web

## Stellar integration

Both projects here are built entirely around Soroban smart contracts.

- The **API** reads on-chain state by simulating contract view calls over Soroban
  RPC (`simulateTransaction`), decoding the returned `ScVal` results with
  `stellar-xdr`. It polls every 10 seconds, holds no keys, and never signs or
  submits a transaction — it only observes.
- The **docs** cover the whole Stellar surface: full references for the four
  Soroban contracts, and a thorough compliance guide explaining the on-chain
  transfer gate (cross-contract `is_allowed` checks), jurisdiction rules, and KYC
  expiry.

### Network & deployed contracts (Testnet)

Network passphrase: `Test SDF Network ; September 2015` · RPC:
`https://soroban-testnet.stellar.org`

| Contract    | Contract ID |
|-------------|-------------|
| registry    | `CBX5SMLTXX6JP4HA5GQIO2V6QM7WCUGL2GZ6D4U773HMRI6RXISKPUR3` |
| compliance  | `CBUERYDM7DXTZLLKDBRJKUBPFJ7M4OSUN4T7XKUARU345RLXNAIQD2IU` |
| dividend    | `CAR4XY3CEBQWFOL27JEWFW34KXSIZA7RFKDQMEIV7ZU723RWY37I2SYX` |
| asset-token (sample) | `CBMCWLSQSWUTLUJFCNBHNBSXMUM3XU7NAQ5TSNERW4HA4ZZBYHLG4ECZ` |

Contract ids are configured via environment variables (see `api/.env.example`).
Per-asset token ids are discovered from the registry at index time.

## API

A read-only REST service that indexes the four RWA contracts and serves their
state as JSON. It polls Soroban RPC every 10 seconds, rebuilds an in-memory
snapshot, and holds no keys — it never signs or submits transactions.

### Endpoints

| Method & path | Description |
|---------------|-------------|
| `GET /stats` | Platform stats: total assets, TVL, holders |
| `GET /assets` | All tokenized assets |
| `GET /assets/:id` | Full asset detail |
| `GET /assets/:id/holders` | Holder list with balances |
| `GET /assets/:id/compliance` | Allowlist summary (counts, no PII) |
| `GET /assets/:id/dividends` | Distribution history |
| `GET /health` | Liveness probe |

### Run it

```bash
cd api
cp .env.example .env      # Testnet defaults are pre-filled
cargo run                 # listens on 0.0.0.0:8080
```

```bash
curl http://localhost:8080/stats
curl http://localhost:8080/assets
```

### Logging

Two env vars control log output:

- `RUST_LOG` — a [`tracing-subscriber` `EnvFilter`](https://docs.rs/tracing-subscriber/latest/tracing_subscriber/filter/struct.EnvFilter.html)
  directive for per-target levels. Defaults to `stellar_rwa_api=info,tower_http=warn`.
- `LOG_FORMAT` — line format: `pretty` (default; human-readable for local dev),
  `compact` (single-line text), or `json` (structured output for production
  log shippers like Loki, Datadog, and Cloud Logging).

Examples:

```bash
RUST_LOG=debug cargo run                        # chatty dev
RUST_LOG=info LOG_FORMAT=json cargo run         # prod-shaped
RUST_LOG=stellar_rwa_api=debug LOG_FORMAT=compact cargo run
```

The container image already sets `LOG_FORMAT=json` so it's ready for structured
log shippers out of the box. To debug a container with human-readable logs,
override at runtime:

```bash
docker run --rm -p 8080:8080 -e LOG_FORMAT=pretty stellar-rwa-api
```

### Tech stack

Rust · Axum · tokio · reqwest · serde · stellar-xdr · stellar-strkey.

## Docs

A documentation site built with Next.js 14 (app router) and MDX. Covers getting
started, full contract references, API references, and guides — including a
thorough **compliance guide**, the core differentiator of an RWA platform.

### Run it

```bash
cd docs
npm install
npm run dev               # http://localhost:3000
```

### Deploying to Vercel

The docs site lives in the `docs/` subdirectory. When importing this repo into
Vercel:

- **Root Directory:** `docs`
- **Framework Preset:** Next.js
- Build command and output are auto-detected.

Set `NEXT_PUBLIC_API_BASE_URL` to your deployed API URL (defaults to
`http://localhost:8080` for local development).

## Repository layout

```
api/           Rust REST API (maintainer-only)
  src/
    main.rs
    routes/    assets, holders, compliance, dividends, stats
    indexer/   Soroban RPC poller + XDR decoding + in-memory snapshot
    models/    serializable domain models
docs/          Next.js + MDX documentation site
  app/         landing + docs/** MDX pages
  components/  Sidebar, DocHeader, CodeBlock, CalloutBox, ApiEndpoint
CONTRIBUTING.md
README.md
```

## Contributing

Contributions are welcome **in `docs/` only**. The `api/` directory is
maintainer-only and PRs touching it will be closed — please open an issue instead.
See [CONTRIBUTING.md](CONTRIBUTING.md).

## License

Apache-2.0.
