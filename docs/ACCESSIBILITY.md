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
