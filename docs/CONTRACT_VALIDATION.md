# Contract Documentation Validation

This documentation includes a contract specification and validation system to ensure that documented contract methods stay in sync with the actual deployed contracts.

## Glossary

New to Stellar/Soroban? These terms appear throughout the contract reference:

- **TTL (time to live):** The number of ledgers a piece of contract storage (or a contract instance) remains valid before it must be extended. Soroban storage entries expire if their TTL is not bumped.
- **Footprint:** The set of ledger entries (contract data, contract code, accounts) that a transaction reads and/or writes. The footprint must be declared up front so the network can schedule and validate the transaction.
- **ScVal:** Soroban's canonical value type (`ScVal`) used to encode arguments and return values crossing the host/contract boundary (e.g. `ScVal::I128`, `ScVal::Address`).
- **SAC (Stellar Asset Contract):** The built-in contract that wraps a classic Stellar asset, exposing it to Soroban with a standard token interface (`transfer`, `balance`, etc.).
- **Passphrase:** The network identifier string that distinguishes networks (e.g. Testnet vs. Mainnet). Transactions signed for one passphrase are invalid on another.

## How it works

1. **Contract specifications** are defined in `lib/contracts.ts` with all method signatures, parameters, auth requirements, and error codes.

2. **Tests** in `__tests__/contracts.test.ts` validate that:
   - All documented methods exist in the spec
   - All spec methods are documented
   - Method signatures (parameter counts) match
   - Contract addresses are correct

3. **CI integration**: Run `npm test` to validate all contract documentation before deployment.

## Adding or updating contract methods

When a contract is updated:

1. Update the method signature in `lib/contracts.ts` (add/modify/remove methods)
2. Update the corresponding `.mdx` file with the new documentation
3. Run `npm test` to validate
4. Commit both files together

## Example: Adding a new method

In `lib/contracts.ts`, add to `ASSET_TOKEN_CONTRACT.methods`:

```typescript
{
  name: "new_method",
  returns: "i128",
  params: [
    { name: "admin", type: "Address" },
    { name: "value", type: "i128" },
  ],
  auth: "admin",
  errors: ["Unauthorized (3)"],
}
```

Then add a section in `docs/app/docs/contracts/asset-token/page.mdx`:

```mdx
### `new_method`

\`\`\`rust
fn new_method(env: Env, admin: Address, value: i128)
\`\`\`

Description of what the method does...

- **Auth:** `admin`
- **Errors:** `Unauthorized (3)`
```

Run `npm test` to verify they match.

## Worked examples must be runnable

The asset-token reference page (`docs/app/docs/contracts/asset-token/page.mdx`) includes a runnable worked example. To keep it honest, every example must be verified against the current Testnet deployment rather than left as prose.

When you add or refresh a worked example:

1. Confirm the method signatures used in the example match `ASSET_TOKEN_CONTRACT.methods` in `lib/contracts.ts`.
2. Confirm the contract address used in the example matches the deployed Asset Token address below.
3. Actually run the example against Testnet (e.g. via `stellar contract invoke` or the Stellar CLI) and confirm it succeeds.
4. Note in the example that it was verified against the current Testnet deployment.

If a method signature or address changes, update the worked example in the same commit so the page keeps reflecting the current surface.

## Worked example: verifying the Registry reference against Testnet

The Registry contract reference page (`docs/app/docs/contracts/registry/page.mdx`) must ship a runnable worked example that is verified against the current Testnet deployment, not left as prose. Use the following procedure whenever the Registry surface changes.

### 1. Confirm the deployed address

The Registry is deployed on **Testnet** at:

```
CBX5SMLTXX6JP4HA5GQIO2V6QM7WCUGL2GZ6D4U773HMRI6RXISKPUR3
```

This address is asserted by the validation suite (see `__tests__/contracts.test.ts`), so a mismatch fails `npm test` before the docs can drift.

### 2. Run the example against Testnet

The worked example on the Registry page uses the Stellar CLI to invoke the live contract. Run it end-to-end and confirm the output matches what the page documents:

```bash
stellar contract invoke \
  --id CBX5SMLTXX6JP4HA5GQIO2V6QM7WCUGL2GZ6D4U773HMRI6RXISKPUR3 \
  --network testnet \
  -- \
  get_asset_count
```

Expected: the command returns the current number of registered assets as an integer. If the method name, argument list, or return shape differs from the page, update the page (and `lib/contracts.ts`) together.

### 3. Keep the example in sync

- The example must call methods that exist in `REGISTRY_CONTRACT.methods` in `lib/contracts.ts`.
- Any auth requirements shown in the example must match the `auth` field in the spec.
- Any error codes referenced must match the `errors` field in the spec.

Run `npm test` after editing to confirm the page and the spec still agree.

## API endpoint contract tests

The checks above cover the **contract** reference pages. A separate Rust test, `api/tests/contract_endpoint_coverage.rs`, does the same job for the **REST API** reference pages under `docs/app/docs/api/`. It pins down the JSON wire format those pages document.

### What it guarantees

The test makes no HTTP calls and needs no env vars, fixtures, database or network. Each test builds a model from `stellar_rwa_api::models` directly, serialises it with `serde_json::to_value`, and asserts on the resulting JSON:

| Endpoint | Model | What is asserted |
| --- | --- | --- |
| `GET /stats` | `Stats` | All 8 fields have their documented JSON types; `tvl_cents` is a string that parses as `i128`; `last_updated` serialises as `null` when unset |
| `GET /assets`, `GET /assets/:id` | `Asset` | The list is a JSON array; each field has its documented type; `valuation_cents` and `total_supply` are `i128`-parseable strings; `index_error` is `null` or a string; `asset_type` round-trips `real_estate`, `invoice` and `commodity` |
| `GET /assets/:id/holders` | `Holder` | **Exactly** the keys `address`, `balance` and `share_percent`; `balance` is an `i128`-parseable string |
| `GET /assets/:id/compliance` | `ComplianceSummary` | Count fields are numbers; `jurisdictions` is an array of `{ jurisdiction, count }`; no `addresses`/`records` keys at the top level and no `address`/`status` keys per jurisdiction (aggregate counts only, no PII) |
| `GET /assets/:id/dividends` | `Distribution` | **Exactly** the 9 documented keys; `total_amount` and `distributed` are `i128`-parseable strings; `overflow_detected` is a boolean |
| `GET /health` | (none) | `status`, `last_indexed_ledger` and `consecutive_failures` are present for both `ok` and `degraded` payloads |

Limits to keep in mind:

- The test does not parse the `.mdx` pages or the OpenAPI spec. The expected shapes are written by hand in the test, so they only stay accurate if they are updated together with the docs.
- The `/health` tests check a hand-built `serde_json::json!` payload, not the route handler, so a change to the handler alone will not fail them.
- Only `Holder` and `Distribution` are checked for an exact key set. Adding a field to `Stats`, `Asset` or `ComplianceSummary` will not fail the test, but renaming, removing or retyping a field will.

### When it fails

A failure means a model's serialised shape no longer matches what the API pages document. Common causes:

- A field was renamed (including via `#[serde(rename = ...)]`), removed, or changed type.
- An `i128` amount started serialising as a JSON number instead of a string.
- A field was added to `Holder` or `Distribution`.

To fix it, decide which side is correct. If the change was unintended, revert the model. If it was intended, update the matching page under `docs/app/docs/api/` and the assertions in `contract_endpoint_coverage.rs` in the same commit, so the docs and the test change together.

### Running it locally

From the repository root:

```bash
cd api
cargo test --test contract_endpoint_coverage
```

To run a single case, add a name filter, e.g. `cargo test --test contract_endpoint_coverage holder_`.

CI runs this test as part of `cargo test` in the `api` working directory (`.github/workflows/api.yml`), so a mismatch fails the API workflow on the PR.

## Contracts

- **Asset Token**: `CBMCWLSQSWUTLUJFCNBHNBSXMUM3XU7NAQ5TSNERW4HA4ZZBYHLG4ECZ`
- **Compliance**: `CBUERYDM7DXTZLLKDBRJKUBPFJ7M4OSUN4T7XKUARU345RLXNAIQD2IU`
- **Registry**: `CBX5SMLTXX6JP4HA5GQIO2V6QM7WCUGL2GZ6D4U773HMRI6RXISKPUR3`
- **Dividend**: `CAR4XY3CEBQWFOL27JEWFW34KXSIZA7RFKDQMEIV7ZU723RWY37I2SYX`

All contracts are deployed on **Testnet** with network passphrase `Test SDF Network ; September 2015`.

## Validation checks

The test suite verifies:

- ✅ Contract addresses match deployed contracts
- ✅ All documented methods exist in the spec
- ✅ All spec methods are documented
- ✅ Method parameter counts match
- ✅ Auth requirements are documented
- ✅ Error codes are up to date
- ✅ Critical methods (transfer, is_allowed, get_all_assets) are present
