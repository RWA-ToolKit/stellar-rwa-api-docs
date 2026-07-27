# Contributing

Thanks for your interest in the Stellar RWA docs! Community contributions are
welcome — **in the `docs/` directory only**.

## Scope: contributors work in `docs/` only

This repository contains two projects:

- **`docs/`** — the Next.js + MDX documentation site. **This is open to
  contributions.**
- **`api/`** — the Rust indexing service. **This is maintainer-only.**

> ⚠️ **PRs that modify anything under `api/` will be closed.**
>
> The API is maintained by the core team because it is tied to deployment
> infrastructure and the on-chain indexer. If you spot an API bug or want a new
> endpoint, please **open an issue** describing it rather than sending a PR.

Everything else — fixing typos, clarifying a guide, improving a code example,
adding a new documentation page — is fair game in `docs/`.

## What makes a good docs contribution

- **Accuracy first.** Code examples must be real and correct against the deployed
  contracts and the current API. Don't invent endpoints, fields, or functions.
- **No placeholders.** No "TODO", no "coming soon", no lorem ipsum.
- **Match the voice.** Concise, technical, and honest about limitations.
- **Use the components.** `CalloutBox`, `ApiEndpoint`, and fenced code blocks keep
  pages consistent. See existing pages for patterns.
- **Update navigation.** If you add a page, add it to `docs/components/nav.ts`.

## Local setup

```bash
cd docs
npm install
npm run dev      # http://localhost:3000
npm run build    # must pass before you open a PR
```

## Submitting a PR

1. Fork and branch from `main`.
2. Make your changes **inside `docs/`**.
3. Run `npm run build` in `docs/` and make sure it passes.
4. Open a PR with a clear description and, for content changes, a screenshot.

## Release checklist

When releasing a new version of the API or contracts:

1. **Update `api/Cargo.toml`** with the new version number (if applicable).
2. **Update `CHANGELOG.md`** with all user-facing changes:
   - API endpoint changes, new endpoints, breaking changes.
   - Contract changes (new functions, parameter changes).
   - Document the date and version.
3. **Update `docs/`** to reflect the changes:
   - Add or modify endpoint references in `docs/pages/api/`.
   - Update contract guides in `docs/pages/contracts/` if applicable.
   - Update examples and code snippets throughout.
4. **Run `npm run build` in `docs/`** to ensure all links and references are valid.
5. **Create a tagged release** on GitHub with a summary linking to the CHANGELOG entry.

This ensures readers can map documentation versions to releases and find the
exact API/contract behavior that was current when the docs were published.

## Reporting API issues

Found a problem with the API? Open an issue with:

- the endpoint and request,
- the response you got and the response you expected,
- the API version (from `GET /`).

We'll pick it up from there.
