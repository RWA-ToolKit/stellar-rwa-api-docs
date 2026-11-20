# Verifying documented API examples

`scripts/verify-doc-examples.mjs` (repo root) extracts each `curl` + `json`
example pair from the `.mdx` pages and compares the documented response with a
live response by shape (keys and value types).

```bash
API_BASE_URL=https://<testnet-api-host> node scripts/verify-doc-examples.mjs
```

Point `API_BASE_URL` at a local instance (`http://localhost:8080`) or a mock to
run it offline. The script exits 1 when drift is found.

A scheduled workflow, `.github/workflows/doc-examples-drift.yml`, runs it weekly
against the URL in the repository variable `DOC_EXAMPLES_API_URL`. It has no
pull request trigger, so drift is reported without blocking merges. The Rust
tests in `api/tests/` continue to validate examples against the response models.

## Asset-token worked example

The runnable example on `docs/app/docs/contracts/asset-token/page.mdx` is
verified against the current Testnet deployment. To re-check it after a
contract or API change:

```bash
# 1. Confirm the documented example still matches the live Testnet response.
API_BASE_URL=https://<testnet-api-host> node scripts/verify-doc-examples.mjs

# 2. Run the example's request against Testnet and compare the returned
#    fields (token id, decimals, symbol, balances) with the page.
curl -s "https://<testnet-api-host>/asset-token/<token-id>" | jq .
```

If the shape or values drift, update the example on the asset-token page and
re-run step 1 until the script exits 0. Record the Testnet deployment the
example was checked against in the page's example section so readers know it is
current.
