# Issue #463: allowlist-only holder derivation status

## Summary

This issue is currently blocked by an architectural gap in the data available from the RWA contract and the indexer pipeline.

The current holder derivation path only includes addresses present in the allowlist, which means it can miss addresses that were removed, pruned, or otherwise absent from the allowlist snapshot. That produces an incomplete holder list and can diverge from the effective supply model.

## Why this is not safely fixable in a small patch

- The API does not currently have a canonical, contract-level holder enumeration source for all valid holders.
- The allowlist is not a complete representation of historical or pruned holders.
- Any fix that derives holders solely from allowlist membership would still be incomplete and could silently create wrong totals.

## Recommended follow-up

This needs a contract or indexer-level source of truth for holder membership, such as:

- a reliable address-balance enumeration from the underlying contract state,
- a persisted holder ledger snapshot keyed by asset and address, or
- a new indexer pipeline that tracks removals and prunes explicitly.

## Status

This branch documents the blocker and does not claim a code fix for the underlying data inconsistency. The issue remains open pending a contract-level data source.

Fixes #463
