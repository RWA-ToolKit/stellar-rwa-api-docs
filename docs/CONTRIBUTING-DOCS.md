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

## Style

Follow the [documentation style guide](./STYLE_GUIDE.md).
