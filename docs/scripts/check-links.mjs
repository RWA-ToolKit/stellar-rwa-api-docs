#!/usr/bin/env node
/**
 * Build-time link checker for the docs site (see .kiro/specs/build-time-link-checker).
 *
 * Scans every `docs/app/docs/**\/*.mdx` file for Markdown links `[text](target)`
 * and JSX `href="target"` attributes, and fails if any internal link does not
 * resolve to a page, static file in `public/`, or redirect source in
 * `redirects.json`. External links (http, mailto, ...) and same-page anchors
 * (`#section`) are skipped. Query strings and fragments are ignored: only the
 * path component is validated.
 *
 * Usage: node scripts/check-links.mjs
 *
 * Exit code is non-zero if any internal link is broken, so this is safe to wire
 * directly into CI as a build-failing step.
 */

import { existsSync, readFileSync, readdirSync, statSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const DOCS_ROOT = path.join(path.dirname(fileURLToPath(import.meta.url)), "..");
const APP_ROOT = path.join(DOCS_ROOT, "app");
const APP_DOCS_DIR = path.join(APP_ROOT, "docs");
const PUBLIC_DIR = path.join(DOCS_ROOT, "public");
const REDIRECTS_FILE = path.join(DOCS_ROOT, "redirects.json");

const MARKDOWN_LINK_RE = /\]\(\s*([^)\s]+)(?:\s+"[^"]*")?\s*\)/g;
const JSX_HREF_RE = /href=["']([^"']+)["']/g;
const EXTERNAL_OR_ANCHOR_RE = /^(?:[a-z][a-z0-9+.-]*:|\/\/|#)/i;
const HEADING_RE = /^(#{1,6})\s+(.+?)\s*#*\s*$/;
const EXPLICIT_ID_RE = /\s*\{#([^}]+)\}\s*$/;
const JSX_HEADING_ID_RE = /<h[1-6][^>]*\bid=["']([^"']+)["']/g;

function findMdxFiles(dir) {
  const files = [];
  for (const entry of readdirSync(dir)) {
    const full = path.join(dir, entry);
    if (statSync(full).isDirectory()) {
      files.push(...findMdxFiles(full));
    } else if (entry.endsWith(".mdx")) {
      files.push(full);
    }
  }
  return files.sort();
}

function isFile(p) {
  return existsSync(p) && statSync(p).isFile();
}

function redirectSources() {
  const { redirects } = JSON.parse(readFileSync(REDIRECTS_FILE, "utf8"));
  return new Set(redirects.map((r) => r.source.replace(/\/+$/, "")));
}

const REDIRECTS = redirectSources();

// Returns the MDX page file a target resolves to, or null if it is not an MDX page.
function resolveMdxPage(sourceFile, target) {
  const pathPart = target.split(/[?#]/)[0];

  if (pathPart.startsWith("/")) {
    const route = pathPart.replace(/\/+$/, "");
    if (REDIRECTS.has(route)) return null;
    const routeDir = path.join(APP_ROOT, route);
    const page = path.join(routeDir, "page.mdx");
    return isFile(page) ? page : null;
  }

  const resolved = path.resolve(path.dirname(sourceFile), pathPart);
  const candidates = [resolved, path.join(resolved, "page.mdx"), `${resolved}.mdx`];
  return candidates.find((c) => c.endsWith(".mdx") && isFile(c)) ?? null;
}

// Returns null when the target resolves, otherwise the path it was expected at.
function resolveTarget(sourceFile, target) {
  const pathPart = target.split(/[?#]/)[0];

  if (pathPart.startsWith("/")) {
    const route = pathPart.replace(/\/+$/, "");
    if (REDIRECTS.has(route)) return null;
    const routeDir = path.join(APP_ROOT, route);
    const candidates = [path.join(routeDir, "page.mdx"), path.join(routeDir, "page.tsx")];
    const staticFile = path.join(PUBLIC_DIR, route);
    if (candidates.some(isFile) || isFile(staticFile)) return null;
    return path.relative(DOCS_ROOT, candidates[0]);
  }

  const resolved = path.resolve(path.dirname(sourceFile), pathPart);
  const candidates = [resolved, path.join(resolved, "page.mdx"), `${resolved}.mdx`];
  if (candidates.some(isFile)) return null;
  return path.relative(DOCS_ROOT, candidates[1]);
}

// Slugifies heading text the same way the docs renderer generates anchor ids.
function slugify(text) {
  return text
    .toLowerCase()
    .trim()
    .replace(/[^\w\s-]/g, "")
    .replace(/\s+/g, "-")
    .replace(/-+/g, "-")
    .replace(/^-|-$/g, "");
}

// Collects the set of anchor ids a page exposes (heading slugs + explicit ids).
function headingIds(pageFile) {
  const ids = new Set();
  for (const line of prose(readFileSync(pageFile, "utf8"))) {
    const heading = line.match(HEADING_RE);
    if (heading) {
      let text = heading[2];
      const explicit = text.match(EXPLICIT_ID_RE);
      if (explicit) {
        ids.add(explicit[1].trim());
        text = text.replace(EXPLICIT_ID_RE, "");
      }
      const slug = slugify(text);
      if (slug) ids.add(slug);
    }
    for (const match of line.matchAll(JSX_HEADING_ID_RE)) {
      ids.add(match[1]);
    }
  }
  return ids;
}

// Returns null when the fragment resolves, otherwise a description of the failure.
function checkFragment(sourceFile, target) {
  const hashIndex = target.indexOf("#");
  if (hashIndex === -1) return null;
  const fragment = decodeURIComponent(target.slice(hashIndex + 1));
  if (!fragment) return null;

  const page = resolveMdxPage(sourceFile, target);
  if (!page) return null; // Non-MDX targets (static files, redirects) are not anchor-checked.

  const ids = headingIds(page);
  if (ids.has(fragment)) return null;
  return `missing anchor #${fragment} in ${path.relative(DOCS_ROOT, page)}`;
}

// Blanks out fenced code blocks and inline code so example links are not checked.
function prose(source) {
  let inFence = false;
  return source.split("\n").map((line) => {
    if (/^\s*```/.test(line)) {
      inFence = !inFence;
      return "";
    }
    if (inFence) return "";
    return line.replace(/`[^`]*`/g, (m) => " ".repeat(m.length));
  });
}

function main() {
  const files = findMdxFiles(APP_DOCS_DIR);
  const broken = [];
  let checked = 0;

  for (const file of files) {
    const lines = prose(readFileSync(file, "utf8"));
    lines.forEach((line, index) => {
      for (const re of [MARKDOWN_LINK_RE, JSX_HREF_RE]) {
        for (const match of line.matchAll(re)) {
          const target = match[1];
          if (EXTERNAL_OR_ANCHOR_RE.test(target)) continue;
          checked += 1;
          const expected = resolveTarget(file, target);
          if (expected) {
            broken.push({ file: path.relative(DOCS_ROOT, file), line: index + 1, target, expected });
            continue;
          }
          const fragmentError = checkFragment(file, target);
          if (fragmentError) {
            broken.push({ file: path.relative(DOCS_ROOT, file), line: index + 1, target, expected: fragmentError });
          }
        }
      }
    });
  }

  if (broken.length > 0) {
    const sorted = broken.sort((a, b) => a.file.localeCompare(b.file) || a.line - b.line);
    console.error(`Broken internal links (${sorted.length}):\n`);
    for (const b of sorted) {
      console.error(`  ${b.file}:${b.line}  ${b.target}`);
      console.error(`      expected: ${b.expected}`);
    }
    process.exitCode = 1;
  } else {
    console.log(`Checked ${checked} internal link(s) across ${files.length} MDX file(s): no broken links.`);
  }
}

main();
