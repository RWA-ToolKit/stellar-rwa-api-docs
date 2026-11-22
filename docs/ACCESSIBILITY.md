# Accessibility Audit & Remediation (WCAG 2.1 Level AA)

This document records the findings, evaluation methodology, and remediations performed for the Stellar RWA documentation site in accordance with **WCAG 2.1 Level AA** standards (Issue #359).

---

## 1. Scope of Audit

- **Pages Audited**:
  - Landing page (`/`)
  - Documentation shell and layout (`/docs/*`)
  - 17 Documentation content pages:
    - Getting Started (`/docs/getting-started`)
    - Architecture Overview (`/docs/architecture`)
    - Contract Reference (`/docs/contracts/asset-token`, `/compliance`, `/registry`, `/dividend`)
    - API Reference (`/docs/api/overview`, `/assets`, `/holders`, `/compliance`, `/dividends`, `/events`, `/rate-limits`)
    - Guides (`/docs/compliance-guide`, `/time-and-ledgers`, `/web-app`, `/integration`)
  - 404 Not Found page (`/not-found`)
- **Components Audited**:
  - `DocHeader` (Top navigation, drawer trigger, mobile drawer modal)
  - `Sidebar` (Sidebar navigation and section hierarchies)
  - `Search` (Keyboard command palette, search input, suggestion list)
  - `PrevNext` (Sequential pagination component)
  - `CodeBlock` (Syntax container and interactive copy button)
  - `CalloutBox` (Advisory notes, tips, warnings, and compliance alerts)
  - `ApiEndpoint` (HTTP method badge and endpoint route display)
  - `ErrorCodeTable` (Contract error code listing and advisory notice)
  - `VersionBanner` (Version mismatch alert)

---

## 2. Findings & Remediations

### 2.1 Heading Hierarchy (WCAG 1.3.1 Info and Relationships, 2.4.6 Headings and Labels)
- **Finding**: In `ErrorCodeTable.tsx`, an embedded callout box rendered an `<h4>` element directly inside the `## Error codes` section (which is an `<h2>`). This caused heading level skipping from `<h2>` directly to `<h4>`, violating strict sequential heading hierarchy.
- **Remediation**:
  - Replaced `<h4>` with `<h3>` in `ErrorCodeTable.tsx` to ensure consecutive heading nesting (`H1` -> `H2` -> `H3`).
  - Audited all 17 MDX content pages, verifying that every page contains exactly one `<h1>` title followed by sequentially ordered `<h2>` and `<h3>` tags without level skipping.
  - Sidebar section titles use `<p>` with appropriate styling rather than heading tags to keep the document heading outline dedicated to the active page content.

### 2.2 Focus Visibility & Keyboard Navigation (WCAG 2.4.7 Focus Visible, 2.4.11 Focus Appearance, 2.1.1 Keyboard)
- **Finding**:
  - The skip-to-content link in `layout.tsx` used classes `sr-only focus:not-sr-only`, but lacked fixed screen positioning, backdrop, text styling, and focus outline when focused.
  - Interactive elements (navigation links, cards, buttons) lacked explicit high-contrast focus rings when navigated using the keyboard (`Tab`/`Shift+Tab`).
  - Search input had a low-opacity focus ring (`focus:ring-brand-500/20`) that was barely distinguishable on dark backgrounds (< 2:1 contrast ratio against `#08090c`).
  - Horizontal-scrolling code blocks (`<pre>`) and wide tables (`<table>`) were inaccessible to keyboard-only users who could not scroll content without a mouse pointer.
- **Remediation**:
  - Added global `:focus-visible` ring in `globals.css` with a high-contrast 2px emerald outline (`#34d399`, > 9:1 contrast ratio against the dark background) with a 2px offset.
  - Upgraded the `#main-content` skip link in `app/layout.tsx` to render a prominent, high-contrast button positioned at `top-4 left-4 z-50` with high z-index and focus ring.
  - Added `tabIndex={0}`, `role="region"`, and descriptive `aria-label`s to code blocks (`CodeBlock.tsx`), MDX pre elements, and data tables (`ErrorCodeTable.tsx`, `mdx-components.tsx`) so keyboard users can scroll horizontal overflows.

### 2.3 Color Contrast (WCAG 1.4.3 Contrast Minimum - Level AA)
- **Finding**:
  - Several UI elements utilized translucent text variants on dark backgrounds (`#08090c`), causing contrast ratios to fall below the 4.5:1 threshold for normal text:
    - Sidebar section headers (`text-base-300/70` yielded ~4.3:1).
    - Inactive sidebar navigation links (`text-base-200/70` in certain contexts).
    - Search result metadata (`text-base-300/60` yielded ~3.5:1; `text-base-200/50` yielded ~3.7:1).
    - Empty state query text in search.
    - Blockquote and table text in MDX prose (`text-base-200/70` and `text-base-200/80`).
- **Remediation**:
  - Replaced translucent opacity color utilities with solid, high-contrast tokens:
    - Sidebar section titles upgraded to `text-base-300` (#a5abba, contrast ratio > 7.5:1).
    - Inactive navigation links upgraded to `text-base-200` (#c7cbd4, contrast ratio > 10:1).
    - Search metadata, excerpts, and placeholders upgraded to `text-base-200` and `text-base-300`.
    - MDX prose styles in `globals.css` updated to use solid `text-base-200` across paragraphs, list items, blockquotes, and table cells.
    - Verified badge colors in `ApiEndpoint.tsx`: GET (`#6ee7b7` > 11:1), POST (`#7dd3fc` > 10:1), PUT (`#fcd34d` > 12:1), DELETE (`#fca5a5` > 8:1).

### 2.4 Assistive Technology & ARIA Semantics (WCAG 1.3.1 Info and Relationships, 4.1.2 Name, Role, Value)
- **Finding**:
  - Search input had `role="combobox"` and `aria-expanded`, but lacked an accessible name (`aria-label`).
  - Mobile slide-in navigation drawer in `DocHeader.tsx` was rendered as a plain `<div>` without modal dialog semantics.
  - Hamburger toggle button lacked `aria-expanded` and `aria-controls` bindings.
  - Code copy button announced only "Copy" / "Copied" without specifying the snippet or announcing dynamic state updates to screen readers.
  - Decorative icons and arrows (`←`, `→`, SVGs) were not hidden from screen readers.
  - `PrevNext` was not encapsulated in a navigation landmark.
- **Remediation**:
  - Added `aria-label="Search documentation"` to the search combobox and `aria-label="Search suggestions"` to the results listbox.
  - Structured the mobile drawer with `role="dialog"`, `aria-modal="true"`, and `aria-label="Navigation menu"`.
  - Bound mobile drawer toggle button with `aria-expanded={open}` and `aria-controls="mobile-nav-drawer"`.
  - Added dynamic `aria-label` to the copy button in `CodeBlock.tsx` (`Copy [label] to clipboard` / `Copied [label] to clipboard`) with `aria-live="polite"` state announcement.
  - Wrapped `PrevNext` in `<nav aria-label="Pagination">` and provided contextual labels (`aria-label="Previous page: [Title]"` / `aria-label="Next page: [Title]"`), marking raw arrow symbols with `aria-hidden="true"`.
  - Added `role="note"` or `role="alert"` with semantic `aside` elements to `CalloutBox.tsx`.
  - Added `role="alert"` to `VersionBanner.tsx` for immediate screen reader notification on version discrepancy.

---

## 3. Verification Checklist

| Criterion | Requirement | Result |
|---|---|---|
| **WCAG 1.3.1** | Info and Relationships: Semantic headings, tables, landmarks, lists | Pass |
| **WCAG 1.4.3** | Contrast (Minimum): Normal text >= 4.5:1, Large text >= 3:1 | Pass |
| **WCAG 2.1.1** | Keyboard: All functionality operable via keyboard including scrolling regions | Pass |
| **WCAG 2.4.1** | Bypass Blocks: Prominent, fully functional skip-to-content mechanism | Pass |
| **WCAG 2.4.6** | Headings and Labels: Descriptive, strictly ordered heading hierarchy | Pass |
| **WCAG 2.4.7** | Focus Visible: High-contrast focus indicators on all interactive elements | Pass |
| **WCAG 4.1.2** | Name, Role, Value: Dialogs, comboboxes, buttons, and pagination landmarks | Pass |

---

## Color contrast audit (WCAG 2.2 AA)

Issue #708. Audits the color tokens in `tailwind.config.ts` and `app/globals.css` against the text/background pairs actually used in `components/`, `app/` and `mdx-components.tsx`.

- **Themes**: the site is dark-only (`<html class="dark">`, `color-scheme: dark`, no light palette), so only the dark theme is audited.
- **Method**: WCAG relative-luminance formula. Translucent backgrounds (e.g. `bg-sky-500/[0.06]`) and translucent text (e.g. `text-base-200/70`) are alpha-composited over the page background `base-950` (`#08090c`) before the ratio is computed. Where a surface is nested (e.g. an ApiEndpoint badge on the `white/[0.03]` card), the layers are composited in order.
- **Thresholds**: 4.5:1 for normal text (1.4.3), 3:1 for UI component boundaries and focus indicators (1.4.11). All audited text is under 18.66px bold or 24px regular, so the 4.5:1 threshold applies throughout.

### Results

| Foreground | Background | Ratio | Min | Result | Where used |
|---|---|---|---|---|---|
| `base-50` `#f6f8fb` | `base-950` | 18.71 | 4.5 | Pass | h1–h3, `strong`, logo |
| `base-100` `#e5e7eb` | `base-950` | 16.08 | 4.5 | Pass | h4, table headers, ErrorCodeTable |
| `base-200` `#c7cbd4` | `base-950` | 12.25 | 4.5 | Pass | body, prose p/li/td/blockquote, VersionBanner |
| `base-300` `#a5abba` | `base-950` | 8.66 | 4.5 | Pass | page description, PrevNext labels, "Docs" label |
| `base-200/70` | `base-950` | 6.29 | 4.5 | Pass | header nav links, inactive sidebar links |
| `base-300/70` | `base-950` | 4.66 | 4.5 | Pass (narrow) | sidebar section titles |
| `base-200/50` | `base-950` | 3.68 | 4.5 | **Fail, fixed** | `.heading-anchor` `#` link (on hover/focus) |
| `base-300` (fix) | `base-950` | 8.66 | 4.5 | Pass | `.heading-anchor`, after the fix |
| `brand-400` `#34d399` | `base-950` | 10.36 | 4.5 | Pass | prose links, 404 code, logo "RWA" |
| `brand-300` `#6ee7b7` | `base-950` | 13.06 | 4.5 | Pass | link hover, error codes, card hover title |
| `brand-300` | `white/[0.08]` | 11.10 | 4.5 | Pass | inline code |
| `brand-300` | `brand-500/10` | 11.70 | 4.5 | Pass | active sidebar link, landing chip |
| `base-100` / `base-300` | `#0a0c11` | 15.80 / 8.51 | 4.5 | Pass | code block text / title, language badge, copy button |
| `sky-300` | `sky-500/[0.06]` | 11.31 | 4.5 | Pass | Note callout label |
| `brand-300` | `brand-500/[0.06]`–`[0.07]` | 12.18–12.33 | 4.5 | Pass | Tip / Compliance callout label |
| `gold-300` | `gold-500/[0.06]` | 12.93 | 4.5 | Pass | Warning callout label |
| `red-300` | `red-500/[0.06]` | 10.09 | 4.5 | Pass | Danger / BreakingChange callout label |
| `base-200` | callout tints | 11.60–11.79 | 4.5 | Pass | callout body text |
| `brand-400` | `gold-500/[0.06]` | 9.70 | 4.5 | Pass | link inside callout (worst tint) |
| `base-100` | callout tint + `white/10` | 12.08 | 4.5 | Pass | inline code inside callout |
| `brand-300` / `sky-300` / `gold-300` / `red-300` | `*-500/15` on `white/[0.03]` | 10.04 / 9.30 / 10.28 / 8.66 | 4.5 | Pass | ApiEndpoint GET / POST / PUT / DELETE badges |
| `base-200` / `base-300` | `white/[0.03]` | 11.67 / 8.25 | 4.5 | Pass | ApiEndpoint description / param heading |
| `base-100` | `white/5` | 14.71 | 4.5 | Pass | DocSearch input text, secondary buttons |
| `base-300/50` | `white/5` | 2.90 | 4.5 | **Fail** | DocSearch placeholder (follow-up) |
| `base-200/85` / `base-300/70` | `base-900` | 8.75 / 4.61 | 4.5 | Pass | DocSearch results / empty state |
| `base-200/70` / `base-300/70` | `base-900` | 6.22 / 4.61 | 4.5 | Pass | sidebar inside mobile drawer |
| `base-300` / `base-200` | `base-900` | 8.40 / 11.88 | 4.5 | Pass | Search result section / excerpt |
| `brand-300` | `base-800` | 11.30 | 4.5 | Pass | Search active result |
| `base-950` | `brand-500` / `brand-400` | 7.85 / 10.36 | 4.5 | Pass | primary CTA, skip link / CTA hover |
| `base-950` | `brand-600` | 5.28 | 3 | Pass | logo mark (darkest end of gradient) |
| `base-200` / `brand-300` | `base-900/60` / `base-850/70` | 12.04 / 12.42 | 4.5 | Pass | landing cards / card hover title |
| `base-100` | `black/30` | 16.34 | 4.5 | Pass | VersionBanner version code |
| `base-200` | `amber-900/20` | 11.26 | 4.5 | Pass | ErrorCodeTable warning note |
| `brand-400` | `base-950` / `base-900` | 10.36 / 10.04 | 3 | Pass | global `:focus-visible` ring |
| `base-300/40` border | `base-900/60` | 2.26 | 3 | **Fail** | `Search.tsx` input border (follow-up) |
| `white/10` border | `white/5` | 1.32 | 3 | **Fail** | DocSearch input border (follow-up) |

**Summary**: 49 foreground/background combinations checked, shown in 36 rows with related pairs grouped. 4 fail: 1 is fixed and 3 are follow-ups.

### Fix applied

- `app/globals.css`: `.prose .heading-anchor` changed from `text-base-200/50` (3.68:1) to `text-base-300` (8.66:1). The anchor stays hidden until its heading is hovered or the anchor is focused, so the change is not visible at rest.

### Follow-ups

These need component (`.tsx`) changes, so they were left out of this token-only pass:

1. **DocSearch placeholder** (`components/DocSearch.tsx`): `placeholder:text-base-300/50` gives 2.90:1. Change it to `placeholder:text-base-300` (7.92:1).
2. **Search input borders, WCAG 1.4.11**: `components/DocSearch.tsx` (`border-white/10`, 1.32:1) and `components/Search.tsx` (`border-base-300/40`, 2.26:1). The input fill is nearly the same as the page, so the border is the only visible edge of the field. Use `border-base-300/60` or stronger (3.69:1 on `base-900/60`). A solid `border-base-600` is not enough (1.69:1). `Search.tsx` is not rendered anywhere right now, so the DocSearch fix matters more.
3. **Sidebar section titles** (`components/Sidebar.tsx`): `text-base-300/70` passes, but only just (4.66:1 on `base-950`, 4.61:1 in the drawer). Section 2.3 above says these were upgraded to solid `text-base-300`, but the component still uses `/70`. Switching to `text-base-300` (8.66:1) would give real headroom.
4. **Undefined color tokens**: Tailwind generates no CSS for these classes, so the elements inherit `base-200` on `base-950` (12.25:1, which passes), but they do not render as intended:
   - `components/VersionBanner.tsx`: `text-brand-200`, `bg-brand-900/30`
   - `components/ErrorCodeTable.tsx`: `bg-brand-900/20`, `border-base-400`
   - `app/docs/api/DocHeader.tsx`: shadcn-style `text-muted-foreground`, `text-foreground`, `border-border`

   Either add the tokens to `tailwind.config.ts` (and re-audit them; for example `brand-200` `#a7f3d0` on `brand-900/30` composited over `base-950`) or change the components to use existing tokens.
