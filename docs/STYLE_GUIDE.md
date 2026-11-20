# Documentation Style Guide

This guide describes how we write and structure documentation across the repo.

## General principles

- Write for a reader who is integrating with the API for the first time.
- Prefer concrete, runnable examples over abstract descriptions.
- Keep terminology consistent with the API reference (e.g. `jurisdiction`, `contract`, `status`).
- Use fenced code blocks with an explicit language tag.

## Worked examples

When a rule is non-obvious, include a worked example that walks through a single
concrete case end to end: the input, the decision, and the resulting behavior.

### Example: blocking a jurisdiction

Suppose a merchant operates in a jurisdiction that is not permitted for a given
product. The compliance guide should show the full flow rather than only stating
the rule.

**1. The request**

A client submits a contract creation request with the sample jurisdiction code
`XX` (a placeholder for a blocked jurisdiction):

```http
POST /v1/contracts
Content-Type: application/json

{
  "jurisdiction": "XX",
  "product": "payments",
  "amount": 1000,
  "currency": "USD"
}
```

**2. The decision**

The compliance layer evaluates the jurisdiction against the product's allowlist.
Because `XX` is not on the allowlist for `payments`, the request is blocked
before any contract is created.

**3. The response**

The API rejects the request with a `403 Forbidden` and a structured error body:

```json
{
  "error": {
    "code": "jurisdiction_blocked",
    "message": "Jurisdiction 'XX' is not permitted for product 'payments'.",
    "jurisdiction": "XX",
    "product": "payments"
  }
}
```

**4. The contract behavior**

No contract record is persisted. Any client-side state that assumed a contract
was created must be rolled back. Retrying with the same jurisdiction code will
produce the same `403` response until the jurisdiction is enabled for the
product.

This worked example should be mirrored in the compliance guide so readers can
see the request, the block decision, and the resulting API/contract behavior in
one place.
