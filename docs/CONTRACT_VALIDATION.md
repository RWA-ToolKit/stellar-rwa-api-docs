# Contract Validation

This document describes how contracts are validated in this project and defines the
jargon you will encounter when reading the integration guide
(`docs/app/docs/integration/page.mdx`). If a term in the integration guide is not
defined inline, it is defined here.

## Glossary

- **TTL (time to live)** — the number of ledgers an entry remains live on-chain
  before it expires. Once an entry's TTL elapses it must be restored (or
  re-created) before it can be read again.
- **Footprint** — the set of ledger entries a transaction reads and writes. The
  footprint is declared up front so the network can lock and validate exactly
  those entries during execution.
- **ScVal** — the Soroban canonical value type. Every value passed to or returned
  from a contract is encoded as an `ScVal` (integers, addresses, maps, vectors,
  bytes, etc.).
- **SAC (Stellar Asset Contract)** — the built-in contract that wraps a classic
  Stellar asset (e.g. XLM or an issued token) so it can be used from Soroban.
  Calling a SAC is how contracts move classic assets.
- **Passphrase** — the network passphrase string that identifies which Stellar
  network (e.g. testnet, mainnet) a transaction or signature is bound to. A
  signature made for one passphrase is invalid on another network.
- **Ledger entry** — a single persisted record on-chain (an account, a contract
  data entry, a contract instance, etc.). Footprints and TTLs apply to ledger
  entries.
- **Contract instance** — the ledger entry that stores a deployed contract's
  executable and its instance storage.

## Validation flow

1. Build the transaction and declare its **footprint** (the ledger entries it
   will read and write).
2. Encode all arguments and return values as **ScVal**.
3. Sign the transaction bound to the correct network **passphrase**.
4. Submit; the network checks the footprint, executes the contract, and updates
   the **TTL** of any entries the transaction touched.

## Notes for newcomers

- If an entry has expired (its **TTL** reached zero), restore it before use.
- When interacting with classic assets, call the **SAC** rather than trying to
  manipulate the underlying account directly.
- Always confirm the **passphrase** matches the network you intend to target;
  mismatches are the most common cause of "invalid signature" errors.
