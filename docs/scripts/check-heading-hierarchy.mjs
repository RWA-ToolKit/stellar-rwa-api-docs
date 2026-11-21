#!/usr/bin/env node
/**
 * Audits heading hierarchy in every `docs/app/docs/**\/page.mdx`.
 *
 * `# Title` renders as the page's single native <h1> (mdx-components.tsx only
 * remaps h2-h6 to HeadingAnchor), so each page must have exactly one h1, it
 * must come first, and headings may not skip a level going down (h2 -> h4).
 * Going back up any number of levels is fine. Literal HTML <h1>-<h6> tags
 * are flagged because they bypass the anchor component.
 *
 * Usage: node scripts/check-heading-hierarchy.mjs
 * Exits 1 if any violation is found.
 */

import { readFileSync, readdirSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const DOCS_ROOT = path.join(path.dirname(fileURLToPath(import.meta.url)), "..");
const APP_DOCS_DIR = path.join(DOCS_ROOT, "app", "docs");

function findPages(dir) {
  const pages = [];
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) pages.push(...findPages(full));
    else if (entry.name === "page.mdx") pages.push(full);
  }
  return pages.sort();
}

/** Returns [{line, level, text}] and [{line, tag}] for HTML heading tags. */
function parse(source) {
  const lines = source.split("\n");
  const headings = [];
  const htmlTags = [];
  let fence = null;
  let inMetadata = false;
  let inComment = false;
  lines.forEach((raw, i) => {
    const line = raw.replace(/\r$/, "");
    const n = i + 1;
    const fm = line.match(/^\s*(`{3,}|~{3,})/);
    if (fence) {
      if (fm && fm[1][0] === fence[0] && fm[1].length >= fence.length) fence = null;
      return;
    }
    if (fm) {
      fence = fm[1];
      return;
    }
    if (inMetadata) {
      if (/^\}\s*;?\s*$/.test(line)) inMetadata = false;
      return;
    }
    if (/^export\s+const\s+metadata\b/.test(line)) {
      if (!/\}\s*;?\s*$/.test(line) || !line.includes("{")) inMetadata = true;
      return;
    }
    let text = line;
    if (inComment) {
      const end = text.indexOf("*/}");
      if (end === -1) return;
      text = text.slice(end + 3);
      inComment = false;
    }
    text = text.replace(/\{\/\*.*?\*\/\}/g, "");
    const start = text.indexOf("{/*");
    if (start !== -1) {
      inComment = true;
      text = text.slice(0, start);
    }
    const h = text.match(/^(#{1,6})\s+(.*)$/);
    if (h) headings.push({ line: n, level: h[1].length, text: h[2].trim() });
    const stripped = text.replace(/`[^`]*`/g, "");
    for (const m of stripped.matchAll(/<(h[1-6])(?=[\s>/])/gi)) {
      htmlTags.push({ line: n, tag: m[1].toLowerCase() });
    }
  });
  return { headings, htmlTags };
}

const pages = findPages(APP_DOCS_DIR);
const violations = [];
const counts = {};
function report(file, line, type, msg) {
  counts[type] = (counts[type] ?? 0) + 1;
  violations.push(`${path.relative(DOCS_ROOT, file)}:${line}  ${msg}`);
}

for (const file of pages) {
  const { headings, htmlTags } = parse(readFileSync(file, "utf8"));
  const h1s = headings.filter((h) => h.level === 1);
  if (h1s.length === 0) report(file, 1, "missing-h1", "missing h1 (`# Title`)");
  for (const h of h1s.slice(1)) {
    report(file, h.line, "multiple-h1", `more than one h1: "${h.text}"`);
  }
  if (headings.length > 0 && headings[0].level !== 1) {
    report(file, headings[0].line, "first-not-h1", `first heading is h${headings[0].level}, expected h1: "${headings[0].text}"`);
  }
  for (let i = 1; i < headings.length; i++) {
    const prev = headings[i - 1];
    const cur = headings[i];
    if (cur.level > prev.level + 1) {
      report(file, cur.line, "skipped-level", `h${prev.level} -> h${cur.level} skips a level: "${cur.text}"`);
    }
  }
  for (const t of htmlTags) {
    report(file, t.line, "html-heading", `literal <${t.tag}> tag; use markdown headings`);
  }
}

console.log(`Scanned ${pages.length} pages.`);
if (violations.length > 0) {
  console.log(violations.join("\n"));
  console.log(`\n${violations.length} violation(s): ${JSON.stringify(counts)}`);
  process.exit(1);
}
console.log("Heading hierarchy OK.");
