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

This repository hosts two independently deployable projects:

### `api/` — Rust REST API (maintainer-only)

Read-only indexer for RWA contracts on Stellar. Polls Soroban RPC, maintains an in-memory snapshot, and serves state as JSON.

**Build & test:**
```bash
cd api
cargo fmt --check    # Format check
cargo clippy         # Linting
cargo test           # Run tests
cargo build --release
docker build -t stellar-rwa-api:latest .  # Build Docker image
```

**Directory structure:**
```
api/src/
  main.rs            # Server bootstrap
  routes/            # Request handlers (assets, holders, compliance, dividends, stats)
  indexer/           # Soroban RPC poller + XDR decoding + in-memory snapshot
  models/            # Serializable domain models
api/Dockerfile       # Multi-stage build for production
```

### `docs/` — Next.js + MDX documentation site (open to contributions)

Public documentation covering contracts, API references, and compliance guides. Deployed to Vercel.

**Build & develop:**
```bash
cd docs
npm install
npm run dev          # http://localhost:3000
npm run build        # Production build
```

**Directory structure:**
```
docs/app/            # Landing page + docs/** MDX pages (app router)
docs/components/     # UI components (Sidebar, DocHeader, CodeBlock, etc.)
docs/package.json    # Next.js + dependencies
```

---

## Contributing

Contributions are welcome **in `docs/` only**. The `api/` directory is
maintainer-only and PRs touching it will be closed — please open an issue instead.
See [CONTRIBUTING.md](CONTRIBUTING.md).

## License

Apache-2.0.
