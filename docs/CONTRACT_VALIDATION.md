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
