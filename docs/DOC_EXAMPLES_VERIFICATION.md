# Documentation Examples Verification

This document tracks worked examples in the docs that must stay numerically
consistent with the implementation. Each example is verified against the
compliance-guide and its corresponding test in
`api/tests/compliance_guide_examples.rs`.

## Compliance Guide: KYC Approval Expiry

The compliance-guide FAQ ("What happens when an approval expires?") includes a
fully worked numeric example. It compares an explicit ledger timestamp against
an explicit expiry timestamp and derives the resulting approval status.

### Worked example

Given an approval with an explicit expiry timestamp and a ledger evaluated at an
explicit ledger timestamp:

| Field              | Value          |
| ------------------ | -------------- |
| `expiry_timestamp` | `1700000000`   |
| `ledger_timestamp` | `1700000000`   |
| Status             | `Valid`        |

| Field              | Value          |
| ------------------ | -------------- |
| `expiry_timestamp` | `1700000000`   |
| `ledger_timestamp` | `1700000001`   |
| Status             | `Expired`      |

Comparison rule: an approval is `Expired` when
`ledger_timestamp > expiry_timestamp`; otherwise it is `Valid`. At the exact
expiry timestamp (`ledger_timestamp == expiry_timestamp`) the approval is still
`Valid`.

### Verification

- The example above is mirrored by the expiry test in
  `api/tests/compliance_guide_examples.rs`.
- The numeric comparison matches the expiry check in
  `api/src/routes/compliance.rs`.
- If the expiry rule or the example changes, update both the compliance-guide
  and this verification note together.

## Troubleshooting

The JSON response examples under `docs/app` are checked by
`scripts/verify-doc-examples.mjs`. For every ` ```bash ` or ` ```sh ` fence
containing a `curl https://<host>/<path>` call, it takes the next ` ```json `
fence as the documented response, issues a plain `GET` to
`API_BASE_URL + <path>`, and compares the two by shape (key names and JSON
value types, not values). The script reports drift and never edits the docs.

| Exit code | Meaning                                            |
| --------- | -------------------------------------------------- |
| `0`       | Every checked example matches the live shape       |
| `1`       | At least one example drifted or its request failed |
| `2`       | `API_BASE_URL` is not set                          |

In CI it runs from `.github/workflows/doc-examples-drift.yml` on a weekly
schedule (Mondays 07:00 UTC) and via `workflow_dispatch`, never on pull
requests, so a failure there does not block merges.

### `API_BASE_URL is required` (exit code 2)

- **Symptom:** the script stops immediately with `API_BASE_URL is required`
  and exits with code `2`; nothing is checked.
- **Cause:** `API_BASE_URL` is unset or empty. In CI the variable is populated
  from the repository variable `DOC_EXAMPLES_API_URL`, so a missing or empty
  repository variable produces the same error.
- **Fix:** set it for the run, e.g.
  `API_BASE_URL=http://localhost:8080 node scripts/verify-doc-examples.mjs`,
  or define `DOC_EXAMPLES_API_URL` under the repository's Actions variables.

### Every example reports `request failed` or doubled paths

- **Symptom:** each example prints `DRIFT <file> GET <path>` followed by
  `request failed: fetch failed`, or by a JSON parse error such as
  `Unexpected token '<'`.
- **Cause:** `API_BASE_URL` points at the wrong place. `fetch failed` means
  nothing answered at that host/port (API not running, wrong port, typo). A
  parse error means something answered with non-JSON, typically an HTML 404 or
  proxy page. The script keeps only the path from each documented `curl` URL
  and appends it to `API_BASE_URL`, so a base that already includes a path
  segment the docs also include (e.g. base `.../v1` with documented
  `/v1/assets`) requests `/v1/v1/assets`.
- **Fix:** use the bare origin the documented paths are relative to, e.g.
  `http://localhost:8080`. A trailing slash is stripped automatically. Confirm
  with `curl "$API_BASE_URL/<path>"` that the server returns JSON before
  re-running.

### Shape drift against a mock or local API with stale data

- **Symptom:** lines such as `$.items[0].issuer: documented but missing from
  live response`, `$.status: in live response but not documented`, or
  `$.balance: documented string, live number`.
- **Cause:** either the documented JSON or the API it is compared against is
  out of date. This is common with a local or mock server seeded with old
  fixture data: a field that was added, renamed or retyped since the fixture
  was written shows up as drift even though the real API and the docs agree.
  `null` is its own type to the script, so a fixture that returns `null` for a
  field documented as an object or string is also reported.
- **Fix:** first check the response against the current implementation (or a
  testnet deployment). If the server is stale, refresh its data and re-run;
  if the docs are stale, update the ` ```json ` fence in the `.mdx` page.
  Only shape matters, so example values do not need to match live values.

### Drift hidden by empty arrays or unauthenticated responses

- **Symptom:** an example passes locally but drifts against another
  environment, or reports keys like `$.error: in live response but not
  documented` alongside many `documented but missing from live response` lines.
- **Cause:** arrays are compared by their first element only, and an empty
  array on either side skips the comparison, so a fixture returning `[]`
  hides drift in the element shape. The script also sends a bare `GET` with
  no headers, so an endpoint that needs authentication returns its error body,
  which is then compared against the documented success response.
- **Fix:** seed the target API so list endpoints return at least one element,
  and point `API_BASE_URL` at an environment where documented endpoints are
  readable without credentials.

### `Checked 0 example(s)` or an example is silently skipped

- **Symptom:** the summary line reports fewer examples than expected, or
  `Checked 0 example(s), 0 with drift.`; alternatively the script crashes with
  `ENOENT` from `readdir`.
- **Cause:** the script only reads `docs/app` relative to the current working
  directory, so running it from anywhere other than the repository root fails
  with `ENOENT`. An example is skipped without any message when:
  - the request fence is not tagged `bash` or `sh` (e.g. ` ```shell `), or the
    response fence is not tagged `json`;
  - the `curl` URL is not a literal `http(s)://host/path` (e.g.
    `curl $API_BASE/assets`);
  - a flag that takes a separate value comes before the URL (e.g.
    `curl -H "Accept: application/json" https://...` or `curl -X POST ...`);
    single-token flags such as `-s` or `-i` are fine;
  - any backtick appears in the prose between the two fences (inline code
    such as `` `id` `` breaks the match);
  - the JSON body does not parse (comments, `...` placeholders, trailing
    commas).
- **Fix:** run `node scripts/verify-doc-examples.mjs` from the repository
  root, and write examples as a `bash` fence with a literal URL placed before
  any value-taking flags, followed by a strictly valid ` ```json ` fence with
  no inline code in between.
