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
