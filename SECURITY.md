# Security Policy

This policy covers the Stellar RWA Toolkit API (`api/`) and the documentation
site (`docs/`) in this repository. The Soroban contracts live in their own
repository and have their own policy.

> The published deployment is Testnet-only and is not suitable for custody of
> real-world value.

## Supported versions

Security fixes are made on the `main` branch. Tagged releases are listed in
[`CHANGELOG.md`](CHANGELOG.md); please reproduce an issue against the latest
`main` or the latest release before reporting it.

## Reporting a vulnerability

**Please do not open a public issue, pull request or discussion for a
vulnerability.**

Report it privately through GitHub's private vulnerability reporting:

1. Go to the **Security** tab of this repository.
2. Choose **Report a vulnerability** (or open
   `https://github.com/RWA-ToolKit/stellar-rwa-api-docs/security/advisories/new`).
3. Describe the issue and include, where you can:
   - the affected component (API endpoint, configuration, docs site page or dependency),
   - the version or commit,
   - steps to reproduce or a proof of concept,
   - the impact you expect.

Please do not include real secrets, private keys or personal data in a report.

## What to expect

This is a best-effort open-source project. The maintainers aim to:

- acknowledge a report within **5 business days**,
- confirm whether it is accepted and share a rough plan within **14 days**,
- publish a fix and a GitHub Security Advisory, crediting the reporter if they
  want to be named, once a fix is available.

Please give the maintainers a reasonable time to fix an issue before disclosing
it publicly (coordinated disclosure).

## Scope

In scope: the Rust API in `api/`, the docs site in `docs/`, and the CI
configuration in `.github/`.

Out of scope: the contracts themselves (report those in the contracts
repository), third-party services such as Soroban RPC providers, and findings
that need physical access or social engineering.

## Dependencies

Vulnerabilities in third-party dependencies are tracked as described in the
[Security considerations](docs/app/docs/security-considerations/page.mdx) page.
