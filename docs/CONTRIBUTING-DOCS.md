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

## OpenAPI spec validation

The `OpenAPI` workflow (`.github/workflows/openapi.yml`) validates
`docs/public/openapi-v1.yaml` with `@apidevtools/swagger-cli`. The version is
pinned to an exact release in that workflow so CI results are reproducible.
Run the same check locally:

```bash
npx --yes @apidevtools/swagger-cli@4.0.4 validate docs/public/openapi-v1.yaml
```

### Bumping swagger-cli

1. Check the available releases with `npm view @apidevtools/swagger-cli versions`.
2. Change the version in `.github/workflows/openapi.yml` (for example
   `@apidevtools/swagger-cli@4.0.4` to the new exact version). Do not use a
   floating range such as `@4`.
3. Run the local command above against `docs/public/openapi-v1.yaml` and confirm
   it passes.
4. Note the bump in the PR description so reviewers can see which version CI now uses.

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

## Internationalization (i18n) plan

The site is English-only today: `app/layout.tsx` hardcodes `<html lang="en">`,
there is no locale routing, and `components/nav-data.json`, `lib/search-index.json`,
and `app/sitemap.ts` each describe a single language. This plan is groundwork for
translation contributions; Phase 0 (this section) changes no code.

### Goals and non-goals

- **Goals:** keep every existing English URL working, let contributors translate
  one page at a time, and never show a reader a broken or empty page.
- **Non-goals:** machine-translating the site, translating the API itself,
  translating the changelog or ADRs, or adding a runtime i18n library for MDX.

### Chosen approach: pilot layout first, `[locale]` later

Next.js 14 App Router i18n means a locale route segment. Moving `app/docs` to
`app/[locale]/docs` immediately would touch every page, `redirects.json`, nav, and
search. Instead, use a pilot that is compatible with the current routes:

- English (default locale `en`) stays at `/docs/...` with no URL change.
- Other locales live under `/docs/<locale>/...` (first pilot: `es`), as static
  segments inside `app/docs`, so no existing route is affected.
- Phase 2 migrates to `app/[locale]/docs` with `en` served unprefixed through a
  middleware rewrite, and the pilot pages move with a directory rename.

```text
docs/
  app/
    layout.tsx                      # root layout; <html lang> derived per locale
    docs/
      getting-started/page.mdx      # English, unchanged (/docs/getting-started)
      es/
        layout.tsx                  # sets lang="es", wraps pages with TranslationBanner
        getting-started/page.mdx    # /docs/es/getting-started
    # Phase 2 target:
    # [locale]/docs/<section>/<page>/page.mdx
  components/
    nav-data.json                   # English labels (source of truth for structure)
    nav-data.es.json                # translated titles, same hrefs minus locale prefix
  lib/
    i18n.ts                         # LOCALES, DEFAULT_LOCALE, localizeHref(), getNav(locale)
    search-index.json               # English
    search-index.es.json            # generated per locale
  messages/es.json                  # UI strings: search placeholder, banner, switcher
```

### Locale config and rendering

- `lib/i18n.ts` exports `LOCALES = ["en", "es"]` and `DEFAULT_LOCALE = "en"`; all
  other code reads from it.
- `<html lang>`: because `<html>` lives in the root layout, the pilot sets it from
  a request header written by middleware (or, in Phase 2, from the `[locale]`
  param). Do not hardcode `lang` once a second locale ships.
- `generateStaticParams` in the Phase 2 `[locale]` layout returns `LOCALES`, so every
  locale is statically built, matching the existing `output: "standalone"` setup.
- Middleware is not needed for the pilot (URLs are explicit). In Phase 2 it only
  rewrites unprefixed paths to `/en/...`; it must not redirect on `Accept-Language`
  alone, so shared links stay stable.

### Navigation, search, sitemap

- Each locale has its own `nav-data.<locale>.json`. Hrefs mirror the English file;
  `getNav(locale)` prefixes them. A missing title falls back to the English one.
- `scripts/build-search-index.mjs` writes one `search-index.<locale>.json` per locale
  from that locale's pages, and `lib/search.ts` loads the index for the current locale.
- `app/sitemap.ts` emits an entry per translated page and adds
  `alternates.languages` (`en`, `es`, `x-default` pointing to English) so search
  engines get hreflang links. Untranslated pages appear only under English.

### What stays untranslated

Code blocks and comments in samples, API paths and parameters, JSON field names,
contract IDs and function names, error codes and enum values, CLI commands, and
product or protocol names (Stellar, Soroban). Translate surrounding prose and
link text only. `npm run check:mdx-samples` still applies to translated pages.

### Translation workflow and review rules

1. Copy the English `page.mdx` into the locale directory at the same relative path.
2. Translate prose; keep headings' anchor ids stable (add explicit ids if the
   translated heading would change the slug) so cross-locale links work.
3. Set `metadata.lastReviewed` and add `sourceReviewed` with the English page's
   `lastReviewed` the translation was based on. `scripts/check-last-reviewed.mjs`
   already enforces the 90-day staleness rule.
4. Add the page to `nav-data.<locale>.json`, then run `npm run check:mdx-samples`
   and `npm run build`.
5. A native-speaking reviewer must approve the PR. A translation whose
   `sourceReviewed` is older than the English page is flagged stale in review.

**Fallback:** if a page has no translation, `/docs/es/<page>` redirects to the
English page, which shows a banner ("This page is not yet available in Spanish")
via the locale layout. A translated page older than its source shows a
"may be out of date" banner linking to the English version.

### Locale switcher

A small control in `DocHeader` lists `LOCALES`. It links to the same page in the
other locale when it exists and to that locale's docs home otherwise, and it
stores no cookie. Labels are endonyms ("English", "Espanol").

### Phased rollout

- **Phase 0 (this plan):** agree on the approach and the directory convention.
- **Phase 1 (pilot):** add `lib/i18n.ts`, the `es` layout, banner, switcher, and a
  Spanish Getting Started page with its nav, search, and sitemap entries.
- **Phase 2 (migration):** move to `app/[locale]/docs`, add the rewrite middleware,
  update `redirects.json`, and open translation to further pages and locales.

## Style

Follow the [documentation style guide](./STYLE_GUIDE.md).

## Accessibility checklist

The site targets WCAG 2.1 Level AA; see [ACCESSIBILITY.md](./ACCESSIBILITY.md) for
the audit and rationale. Tick each item that applies before you open a PR.

- [ ] Every image has alt text (`![alt](...)`) that describes its content or
      purpose. Do not start with "image of" or "picture of".
- [ ] Purely decorative images use empty alt text (`![](...)`).
- [ ] Every diagram (SVG, Mermaid, or image) has alt text or an adjacent text
      description. The text alternative must carry the same information as the
      diagram, not just its title.
- [ ] No information is conveyed by color alone, in diagrams, callouts, or
      tables. Add a label, icon, or text cue as well.
- [ ] Link text is meaningful out of context. Avoid "click here" and "read more".
- [ ] Heading levels do not skip: one `h1` per page, then `h2`, then `h3` in order.
- [ ] Every fenced code block has a language tag (for example `ts` or `bash`).
- [ ] Tables have a header row.
- [ ] If available, run the heading check and accessibility tests locally and
      fix anything they report.

## Glossary and jargon

When a page introduces a term a newcomer may not know (for example `TTL`,
`footprint`, `ScVal`, `SAC`, or `passphrase`), define it inline on first use or
link to the shared glossary. Prefer a short parenthetical definition over an
unexplained acronym, and keep the glossary entry in sync when the contract or
API surface changes.
