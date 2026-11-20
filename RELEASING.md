# Releasing

This is the maintainer runbook for cutting a new version of the
`stellar-rwa-api` crate. It addresses issue #87 by replacing ad-hoc versioning
with a documented checklist plus a `cliff.toml` configuration that turns the
existing conventional-commits history into a Keep a Changelog 1.1.0 section.

## When to release

Cut a release when one of the following is true since the last tagged release:

- A user-facing change has landed (`feat:` or `fix:`).
- A breaking change (`!` suffix on a conventional type, or a `BREAKING
  CHANGE:` footer in the body) needs to be signalled through SemVer.

When in doubt: cut a release. Tags are cheap; silence on the API surface is
not.

## Prerequisites

- Working tree is clean and you are on the latest `main`.
- `git tag -l 'v*'` shows the previous tag.
- `git-cliff` is installed locally. It's a single Rust binary
  (`cargo install git-cliff`) or brew/apt; no network access is needed at
  release time because the tool reads the local git history.
- Conventional commits are already used across the repo, so no message-style
  changes are needed before cutting.

## Checklist

From a clean `main` checkout:

1. **Decide the bump** by scanning commits since the last tag:
   ```sh
   git log --oneline <last-tag>..HEAD
   ```
   Apply the highest bump in this table — bumps do not stack within a single
   release:

   | Conventional commit in the range             | Bump   |
   | --------------------------------------------- | ------ |
   | `!` suffix anywhere, or `BREAKING CHANGE:` body | major  |
   | `feat:`                                       | minor  |
   | Anything else (`fix:`, `chore:`, `docs:`, …)  | patch  |

   Document the chosen bump in the section title (`v<X>.<Y>.<Z>`) so a reader
   of `CHANGELOG.md` doesn't need to reconstruct it from the diff.

2. **Bump `version` in `api/Cargo.toml`**. This is the single source of
   truth — `GET /` reads `CARGO_PKG_VERSION` from it, so a forgotten bump
   silently desynchronises the running API from `CHANGELOG.md`.

3. **Generate the changelog draft.**
   ```sh
   git-cliff --tag v<X>.<Y>.<Z> --prepend CHANGELOG.md
   ```
   `cliff.toml` groups commits by conventional-commit type and emits a
   `## [v<X>.<Y>.<Z>]` section in Keep a Changelog 1.1.0 format. The
   `--prepend` flag keeps the existing history intact (the 0.1.0 entry,
   earlier hand-curated content) and inserts the new section above it.

   Inspect the result with `git diff CHANGELOG.md` and edit by hand if any
   single-commit bullet doesn't read sensibly as a user-facing note — git-cliff
   preserves each commit's first line, which is conventional-commit terse by
   design. Group related bullets, edit `**(BREAKING)**` markers, and pull in
   PR numbers from the GitHub UI as needed.

4. **Commit the bump + new section:**
   ```sh
   git commit -am 'chore(release): v<X>.<Y>.<Z>'
   ```
   The `cliff.toml` skip-rule on `^chore\(release\)` keeps this commit out of
   the next changelog run, so the v<X+1>.Y.<Z+1> section won't end up with a
   "Chores: chore(release): v<X>.<Y>.<Z>" bullet describing the bump itself.

5. **Tag and push.**
   ```sh
   git tag -s v<X>.<Y>.<Z> -m 'v<X>.<Y>.<Z>'   # sign if you have a GPG key configured
   git push origin main --follow-tags
   ```
   `--follow-tags` pushes the annotated tag alongside the commit in one
   operation, so CI on `main` and any tag-driven release workflows never see a
   half-pushed state.

6. **Sanity-check the deployed build.**
   ```sh
   curl -s https://<your-deploy-host>/ | jq -r .version
   ```
   The response's `version` field must read `v<X>.<Y>.<Z>` (or `<X>.<Y>.<Z>`,
   depending on how your formatter strips the leading `v`). Anything else
   means `api/Cargo.toml` was bumped but the deployed build still has the old
   baked-in `CARGO_PKG_VERSION` — re-cut the tag, don't hand-edit the
   deployed host.

## Optional: cut a GitHub Release

From the pushed tag, open **Draft a new release** in the GitHub UI. The body
can be the new `## [v<X>.<Y>.<Z>]` block you wrote in step 3, verbatim. This
is also where prebuilt release artifacts attach in the future if the crate
ever starts shipping binaries (e.g. a static `musl` build of `stellar-rwa-api`).

## Why a manual checklist, not full automation

We considered four options before landing on this one:

- **release-please (GitHub Action)**: opens a release PR automatically on
  every push to `main`. The cost is one more CI black-box to reason about,
  including a path-mapping config so it touches only `api/Cargo.toml` and not
  the Next.js side. Not worth it for a release-cadence of roughly one per
  quarter.
- **cocogitto (Rust CLI)**: all-in-one conventional-commits bump + changelog
  + tag. Requires every maintainer to install and remember it.
- **release-only git-cliff (this option)**: changelog is a deterministic
  output of commit history; the only hand-crafting is reading the draft
  and editing for clarity.
- **Pure hand-curated CHANGELOG + checklist**: the changelog drifts the first
  time a release is cut in a hurry.

The release cadence (currently 0.1.0 → 0.2.0) and the maintainer-only
contributor policy in [`CONTRIBUTING.md`](./CONTRIBUTING.md) make a
mechanical checklist the right amount of rigor.

## Pointers

- Conventional Commits specification: <https://www.conventionalcommits.org/>
- Keep a Changelog 1.1.0: <https://keepachangelog.com/en/1.1.0/>
- git-cliff: <https://git-cliff.org/>
