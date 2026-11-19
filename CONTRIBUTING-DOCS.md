# Contributing to the Docs — PR Guidelines

This file supplements [`CONTRIBUTING.md`](./CONTRIBUTING.md) with guidance
specific to writing the pull request itself, once your change is ready.

## Write your PR description as acceptance criteria

A docs PR is easy to review when the description lets a reviewer check the
change against a short list of pass/fail statements, instead of re-deriving
what "done" means from the diff. Write the description as **acceptance
criteria**: concrete, checkable statements about what the finished page or
edit must do, not a narration of the files you touched.

### Format

```md
## What changed
One or two sentences on the page/section and why it needed this change.

## Acceptance criteria
- [ ] <specific, checkable statement>
- [ ] <specific, checkable statement>
- [ ] <specific, checkable statement>

## How I verified this
- `npm run build` passes in `docs/`
- <manual check, e.g. "clicked every new link in the rendered page">
```

### What makes a criterion good

- **Checkable, not vague.** "Explains CORS errors" is vague. "Lists the
  actual `CorsLayer` config (`allow_origin(Any)`) so readers know CORS is
  already permissive and don't waste time configuring it" is checkable.
- **Scoped to observable behavior of the docs site**, not your process.
  Prefer "the new page appears under Guides in the sidebar and renders
  without a 404" over "I added the page and updated the nav file."
- **One criterion per independently-reviewable fact.** Split "accurate and
  well-formatted" into an accuracy criterion and a formatting criterion —
  a reviewer should be able to check each one off independently.
- **Tie accuracy criteria to source of truth.** If a criterion claims
  something about the API or contracts, name the file it was checked
  against (e.g. "matches the error shape in `api/src/models.rs`").

### Example

For a change adding a troubleshooting entry:

```md
## What changed
Adds a troubleshooting entry for browser CORS errors, since the API's CORS
policy wasn't documented anywhere and users were guessing.

## Acceptance criteria
- [ ] New page is reachable at `/docs/troubleshooting` and listed under Guides
      in `docs/components/nav.ts`
- [ ] Describes the API's actual CORS policy (`CorsLayer` in
      `api/src/routes/mod.rs`), not a generic CORS explanation
- [ ] Gives at least one concrete diagnostic step (e.g. a `curl` command)
      a reader can run themselves
- [ ] No invented endpoints, headers, or config options

## How I verified this
- Read `api/src/routes/mod.rs` to confirm the CORS policy described matches
  the code
- `npm run build` passes in `docs/`
```

### Why this matters

Acceptance-criteria PRs are faster to review: a maintainer can read the list
and check it against the diff line by line, instead of inferring intent.
They also make scope creep visible — if the diff does something no
criterion mentions, that's worth a question before merge.

This does not replace the checklist in `CONTRIBUTING.md` — it is about how
you *describe* the change you already made following that checklist.
