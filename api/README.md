# Stellar RWA API

A read-only REST service that indexes all tokenized real-world asset activity on
Stellar and serves it as JSON. Written in **Rust** (Axum + tokio).

> **Maintainer-only.** The `api/` directory is not open to contributions — PRs
> will be closed. Open an issue instead. See the root
> [CONTRIBUTING.md](../CONTRIBUTING.md).

## How it works

A background task polls **Soroban RPC** every **10 seconds**. On each tick it
simulates view calls against the four deployed RWA contracts (`get_all_assets`,
`get_metadata`, `get_allowlist`, `balance`, `get_distributions_for_asset`) and
rebuilds an in-memory snapshot. No database, no key material — the service never
signs or submits a transaction.

If the RPC is temporarily unreachable the indexer logs the error, keeps serving
the last good snapshot, and retries on the next tick.

## Endpoints

All **data** endpoints are nested under `/v1`. Utility endpoints sit at the root.

| Method & path | Description |
|---------------|-------------|
| `GET /` | Self-describing JSON index of all endpoints |
| `GET /version` | Crate version and release label |
| `GET /health` | Liveness probe — `200 ok` / `503 degraded` |
| `GET /metrics` | Prometheus scrape endpoint (requires `Authorization: Bearer <token>`) |
| `GET /v1/stats` | Platform stats: total assets, TVL, holder count |
| `GET /v1/assets` | All tokenized assets (filterable by `asset_type`, `active`) |
| `GET /v1/assets/:id` | Full asset detail by registry id |
| `GET /v1/assets/:id/holders` | Holder list with balances, sorted by balance descending |
| `GET /v1/assets/:id/compliance` | KYC allowlist aggregate (counts only — no PII) |
| `GET /v1/assets/:id/dividends` | Distribution history, newest ledger first |

## Running locally

```bash
cd api
cp .env.example .env      # Testnet defaults are pre-filled
cargo run
# → listening on 0.0.0.0:8080
```

```bash
curl http://localhost:8080/health
curl http://localhost:8080/v1/stats
curl http://localhost:8080/v1/assets
```

## Running with Docker

```bash
docker build -t stellar-rwa-api .
docker run -p 8080:8080 --env-file .env stellar-rwa-api
```

Or pass variables individually:

```bash
docker run -p 8080:8080 \
  -e RWA_RPC_URL=https://soroban-testnet.stellar.org \
  -e RWA_REGISTRY_ID=CBX5SMLTXX6JP4HA5GQIO2V6QM7WCUGL2GZ6D4U773HMRI6RXISKPUR3 \
  -e RWA_DIVIDEND_ID=CAR4XY3CEBQWFOL27JEWFW34KXSIZA7RFKDQMEIV7ZU723RWY37I2SYX \
  -e RWA_READ_SOURCE=GAIQGTOBTTLLDJ4SWGGESM7UWJ2DI4K3ZNHUSHPDKJL2IE5FKY3BSRAA \
  stellar-rwa-api
```

## Environment variables

| Variable | Default | Description |
|----------|---------|-------------|
| `RWA_RPC_URL` | `https://soroban-testnet.stellar.org` | Soroban RPC endpoint |
| `RWA_REGISTRY_ID` | _(Testnet id)_ | Deployed registry contract id |
| `RWA_DIVIDEND_ID` | _(Testnet id)_ | Deployed dividend contract id |
| `RWA_READ_SOURCE` | _(Testnet account)_ | Funded account used as source for read simulations |
| `RWA_CORS_ALLOWED_ORIGINS` | `http://localhost:3000` | Comma-separated browser origin allowlist |
| `RWA_RATE_LIMIT_PER_SECOND` | `5` | Sustained requests per client IP per second |
| `RWA_RATE_LIMIT_BURST` | `20` | Maximum per-client burst |
| `RWA_REQUEST_TIMEOUT_SECS` | `30` | Maximum request duration in seconds |
| `RWA_MAX_BODY_BYTES` | `1048576` | Maximum request body size (bytes) |
| `RWA_METRICS_TOKEN` | _(none)_ | Bearer token required to scrape `/metrics`; requests are denied when unset |
| `PORT` | `8080` | HTTP server port |
| `RUST_LOG` | `stellar_rwa_api=info,tower_http=warn` | Log filter (tracing-subscriber format) |

Copy `.env.example` for Testnet defaults. When `RWA_RPC_URL` is the default
Testnet RPC the contract ids are pre-populated; for any other RPC both contract
ids must be set explicitly.

## Rate limiting and CORS

All endpoints are rate-limited per client IP: **5 requests/second** sustained with
a **burst of 20**. Responses beyond the limit receive `429 Too Many Requests`.

CORS is restricted to the origins configured in `RWA_CORS_ALLOWED_ORIGINS`
(default: `http://localhost:3000`). Only `GET` requests with a `Content-Type`
header are allowed cross-origin.

## Response conventions

- **Large integers are strings.** Token amounts (`total_supply`, `balance`,
  `total_amount`, `distributed`) and valuations (`valuation_cents`, `tvl_cents`)
  are `i128` on-chain and are serialized as decimal **strings** to preserve
  precision in JavaScript consumers.
- **Convenience floats.** Valuations include a `valuation_usd` / `tvl_usd` (f64
  dollars) and percentages (`share_percent`, `claimed_percent`) for display.
- **Amounts are raw base units.** Divide by `10 ** decimals` to get a display
  value.
- **Cents.** `valuation_cents` is USD cents; `valuation_usd = valuation_cents / 100`.
- **ETags and caching.** Data routes include `Cache-Control` and `ETag` headers
  (keyed on `last_indexed_ledger`). Clients that send `If-None-Match` get `304 Not
  Modified` when the snapshot has not advanced.

## Metrics

`GET /metrics` returns a Prometheus text exposition. It requires a `Bearer`
token set via `RWA_METRICS_TOKEN`. Exposed metrics include indexer refresh
latency, failure counts, last success timestamp, and per-asset read errors.

## Tech stack

Rust · Axum · tokio · reqwest · serde · stellar-xdr · stellar-strkey ·
tower-governor (rate limiting) · metrics-exporter-prometheus

## Tests

```bash
cargo test
```

Integration tests live in `tests/` and cover the full round-trip response shapes
that the docs and web app rely on.
