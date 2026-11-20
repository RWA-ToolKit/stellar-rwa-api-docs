# Changelog

All notable changes to this repository are documented here. The format is based
on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## Changelog entry format

Contributors should add entries under an `## [Unreleased]` section at the top of
this file, following [Keep a Changelog](https://keepachangelog.com/en/1.1.0/):

- Group changes under one of these headings, in this order: `### Added`,
  `### Changed`, `### Deprecated`, `### Removed`, `### Fixed`, `### Security`.
- Write one bullet per notable change, in the imperative or past tense, and
  reference the related issue or PR where useful (e.g. `(#743)`).
- Only document user-facing or operationally relevant changes; skip internal
  refactors and chores.
- Use ISO dates (`YYYY-MM-DD`) for released versions and keep the newest
  version at the top.

Example:

```markdown
## [Unreleased]

### Added

- Changelog entry guidance for contributors (#743).

### Fixed

- Corrected the `/assets` filter documentation.
```

## [0.1.0] — 2026-07-09

Initial release: the indexing REST API and the documentation site.

### API

- Rust/Axum service indexing tokenized RWA activity on Stellar.
- Soroban RPC indexer polling every 10 seconds, decoding `ScVal` returns to JSON
  and rebuilding an in-memory snapshot (no database).
- Endpoints: `GET /stats`, `/assets` (with `?asset_type` and `?active` filters),
  `/assets/:id`, `/assets/:id/holders`, `/assets/:id/compliance`,
  `/assets/:id/dividends`, `/health`.
- Non-PII compliance summaries (counts and jurisdiction breakdown only).
- Graceful shutdown, request tracing, CORS, and friendly error handling — no
  panics on transient RPC failures.
- Unit tests for `ScVal`→JSON decoding and formatting helpers.
- Multi-stage Dockerfile.

### Docs

- Next.js 14 + MDX documentation site.
- Getting Started; full contract references (asset-token, compliance, registry,
  dividend); API references (overview, assets, holders, compliance, dividends);
  and a thorough Compliance Guide, Web App Guide, and Integration Guide.
- Sidebar navigation, prev/next, callouts, API endpoint banners, robots and
  sitemap, and OpenGraph metadata.

### CI

- GitHub Actions for the API (fmt, clippy, test, release build) and the docs
  (Next.js build), each path-filtered to its own project.
