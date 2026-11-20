# Contributing to the docs site

The repository-wide rules are in [CONTRIBUTING.md](../CONTRIBUTING.md). This guide covers the docs site in `docs/`.

## Local development

```bash
cd docs
npm ci
npm run dev      # http://localhost:3000
npm run lint
npm run build    # must pass before you open a PR
```

## Local CI checks

Run both of these before pushing. They are the same checks CI runs on every PR.

### MDX sample checker

```bash
cd docs
npm run check:mdx-samples
```

The script (`docs/scripts/check-mdx-code-samples.mjs`) extracts every fenced
`ts`, `tsx`, `js`, and `jsx` code block from `.mdx` files under `docs/app/` and
verifies that each one parses as valid JavaScript or TypeScript. It catches:

- Syntax errors (missing brackets, invalid keywords, malformed arrow functions).
- Copy-paste mistakes that would make a TypeScript example unparseable.

It does **not** run the samples, resolve imports, or type-check them. Blocks
fenced as `bash`, `json`, `rust`, or other languages are skipped. Run it any time
you add or change a TypeScript example in an MDX file.

### Production build

```bash
cd docs
npm run build
```

Runs the Next.js production build. This confirms that all MDX pages compile,
internal links resolve, and Next.js can generate the static output. A PR cannot
be merged if this command fails.

## Sample checker

`npm run check:mdx-samples` — see [Local CI checks](#local-ci-checks) above.

## Adding a page

1. Create `docs/app/docs/<section>/<page>/page.mdx`, following an existing page in the same section.
2. Add an entry to the matching section in `docs/components/nav.ts`. Order in that array is the sidebar order, and it also drives previous/next links and search metadata. Place the page where a reader would expect to find it in the section.
3. Run `npm run check:mdx-samples` and `npm run build`.

## Changelog entries

The changelog follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and the project uses [Semantic Versioning](https://semver.org/). The canonical
list lives in [`CHANGELOG.md`](../CHANGELOG.md); the docs site renders the same
content at `docs/app/docs/changelog/page.mdx`. Keep both in sync when you add an
entry.

### Format

- Group entries under a version heading (`## [Unreleased]` or `## [x.y.z] - YYYY-MM-DD`).
- Within a version, use the standard change-type subheadings, in this order:
  `### Added`, `### Changed`, `### Deprecated`, `### Removed`, `### Fixed`,
  `### Security`. Omit any section that has no entries.
- Write one bullet per change, in the imperative or past tense, and reference the
  issue or PR when one exists.

```markdown
## [Unreleased]

### Added

- Changelog entry guidance for contributors (#743)

### Fixed

- Corrected the `simulateTransaction` example (#742)
```

### Rules

- Add new work under `## [Unreleased]`; do not edit already-released sections.
- Use the change type that matches the impact: `Added` for new features,
  `Changed` for behavior changes, `Fixed` for bug fixes, and so on.
- Keep bullets short and user-facing — describe the effect, not the internals.
- When a release is cut, move the `Unreleased` entries under a new version
  heading with the release date.

## Scratch and planning documents

Do not commit working notes, batch plans, or scratch documents (for example
`BATCH_*_*.md`) to the repository root. The root is reserved for the canonical
project files (`README.md`, `CHANGELOG.md`, `CONTRIBUTING.md`, `RELEASING.md`,
and similar).

- If a scratch document describes work that is still actionable, capture it as
  a GitHub issue and delete the file.
- If it records shipped work, fold the user-facing parts into
  [`CHANGELOG.md`](../CHANGELOG.md) and delete the file.
- If it has lasting reference value, move it under `docs/` with a short header
  explaining its origin and status (for example, whether it is historical).

## Style

Follow the [documentation style guide](./STYLE_GUIDE.md).

## Glossary and jargon

When a page introduces a term a newcomer may not know (for example `TTL`,
`footprint`, `ScVal`, `SAC`, or `passphrase`), define it inline on first use or
link to the shared glossary. Prefer a short parenthetical definition over an
unexplained acronym, and keep the glossary entry in sync when the contract or
API surface changes.
