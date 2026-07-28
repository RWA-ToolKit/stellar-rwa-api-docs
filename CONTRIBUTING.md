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

This monorepo contains two independent projects with separate toolchains:

### Docs (Next.js + MDX) — Open to contributions

```bash
cd docs
npm install
npm run dev      # http://localhost:3000
npm run build    # must pass before you open a PR
npm run lint     # Check for style issues
```

### API (Rust) — Maintainer-only

The API uses standard Rust tooling. While PRs modifying `api/` will not be accepted,
you can set up the environment locally to understand the codebase or verify builds:

```bash
cd api
cargo build           # Compile the API
cargo test            # Run unit tests
cargo fmt --check     # Check code formatting (Rust style)
cargo clippy          # Lint for common mistakes and idioms
cargo fmt             # Auto-format code (apply changes)
```

Our CI runs these checks on every push (see `.github/workflows/`), so understanding
these commands helps you see what the automated checks look for.

## Submitting a PR

1. Fork and branch from `main`.
2. Make your changes **inside `docs/`**.
3. Run `npm run build` in `docs/` and make sure it passes.
4. Open a PR with a clear description and, for content changes, a screenshot.

## Reporting API issues

Found a problem with the API? Open an issue with:

- the endpoint and request,
- the response you got and the response you expected,
- the API version (from `GET /`).

We'll pick it up from there.

## Releasing (maintainers only)

The api crate is versioned from `api/Cargo.toml`, and the `/` endpoint surfaces
that same version to consumers. The release process is documented in
[RELEASING.md](./RELEASING.md): a maintainer runs a small `git-cliff` step,
bumps the version, and tags. PRs that touch the api crate are still
maintainer-only per the scope rule above — a release PR is no exception.
