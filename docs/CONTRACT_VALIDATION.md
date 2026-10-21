# Contract Documentation Validation

This documentation includes a contract specification and validation system to ensure that documented contract methods stay in sync with the actual deployed contracts.

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
