# Architecture Decision Records (ADRs)

This directory records significant architectural decisions for the
`stellar-rwa-api-docs` project — the "why" behind choices that aren't obvious
from the code alone.

## Why keep ADRs

Decisions like "why doesn't the API use a database" or "why does the indexer
do a full re-read every cycle instead of replaying events" come up repeatedly
in issues and PR discussions. Writing them down once, with the reasoning and
the alternatives that were rejected, means future contributors (and the
current team) don't have to re-litigate them or guess from the code.

## When to write one

Add an ADR when a change:

- Introduces a new dependency, storage mechanism, or external service.
- Changes how two of the three repositories (`stellar-rwa-contracts`,
  `stellar-rwa-api-docs`, `stellar-rwa-web`) interact.
- Reverses or significantly qualifies an earlier decision.
- Was discussed and debated — if people disagreed, the resolution is worth
  recording.

Small, reversible, or purely cosmetic changes don't need one.

## Format

Each ADR is a numbered Markdown file: `NNNN-short-title.md`, using the next
available number. Use this structure:

```markdown
# NNNN. Title

- **Status:** Proposed | Accepted | Superseded by NNNN | Deprecated
- **Date:** YYYY-MM-DD

## Context

What problem or question forced this decision? What constraints applied?

## Decision

What was decided, stated plainly.

## Alternatives considered

What else was on the table, and why it was rejected.

## Consequences

What this makes easier, what it makes harder, and what it rules out.
```

## Index

| # | Title | Status |
|---|---|---|
| [0001](./0001-in-memory-snapshot-no-database.md) | In-memory snapshot instead of a database | Accepted |

## Process

1. Open a PR adding the new ADR file and an entry in the index table above.
2. ADRs for `docs/` content follow the normal contribution flow in
   [CONTRIBUTING.md](../../../../CONTRIBUTING.md). ADRs that document a
   decision inside `api/` are still written here (they're documentation, not
   API code) but should be reviewed by a maintainer who worked on that
   decision.
3. Once merged, an ADR is a historical record — don't edit it to reflect new
   decisions. If a later decision reverses it, write a new ADR and mark the
   old one "Superseded by NNNN".
