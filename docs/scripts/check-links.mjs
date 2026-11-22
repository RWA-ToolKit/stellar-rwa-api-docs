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
