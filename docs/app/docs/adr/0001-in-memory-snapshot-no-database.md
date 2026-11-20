# 0001. In-memory snapshot instead of a database

- **Status:** Accepted
- **Date:** 2026-07-09

## Context

The API (`api/`, Rust/Axum) needs to serve reads about tokenized RWA activity
on Stellar — assets, holders, compliance summaries, dividends, and aggregate
stats — fast enough for a web front end, without ever being the source of
truth for that data. The Soroban ledger is always the source of truth; the
API only needs a read-optimized view of it.

The initial release (see `CHANGELOG.md`, `[0.1.0] — 2026-07-09`) had to decide
how that view would be stored and kept current: persisted in a database and
updated incrementally, or held in memory and rebuilt wholesale.

## Decision

The indexer polls Soroban RPC every 10 seconds (`POLL_INTERVAL` in
`api/src/indexer/mod.rs`), re-reads the *current* contract state via
simulated view calls on every cycle, and assembles a brand-new in-memory
snapshot from scratch. The new snapshot is swapped in atomically; no database
is used, and nothing is persisted to disk. See
[Architecture Overview](/docs/architecture) and [Snapshot Memory](/docs/snapshot-memory)
for the resulting request-serving model and its memory/cost profile.

## Alternatives considered

- **Database with incremental updates (event-sourced).** Would require a
  persistent event cursor, replay-after-restart logic, and schema migrations.
  Rejected for v1: more moving parts and a bigger attack surface, for data
  that already has a canonical source of truth (the ledger) just one RPC
  call away.
- **Database as a read cache, updated via full re-read.** Would add
  persistence and a migration story without removing the "full re-read every
  cycle" cost, since the API still needs to reconcile with the current chain
  state. Rejected as unnecessary complexity for the request volume in scope.

## Consequences

- **Simplicity.** No database dependency, no migrations, no event cursor to
  fall behind on. A freshly started API is simply empty until its first
  successful poll cycle.
- **Statelessness across restarts.** A restart discards the snapshot entirely;
  there is nothing to recover or repair. The next cycle rebuilds it from the
  chain.
- **Per-request cost.** Every request currently takes a full
  `Snapshot::clone()` (see [Snapshot Memory](/docs/snapshot-memory)), which is
  the main cost this decision defers rather than avoids — tracked there as a
  known optimization target (sharing an `Arc<Snapshot>` instead of cloning).
- **Freshness bound.** Data can lag the ledger by up to one poll interval plus
  cycle time; this is documented on [`/stats`](/docs/api/overview) via
  `last_indexed_ledger` and `last_updated` rather than hidden.
