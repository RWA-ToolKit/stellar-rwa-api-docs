#!/usr/bin/env node
/**
 * Scans every `docs/app/docs/**\/page.mdx` for its exported `metadata.lastReviewed`
 * date and fails if any page is missing one, has one in the wrong format, or is
 * older than the staleness threshold (90 days by default).
 *
 * Usage:
 *   node scripts/check-last-reviewed.mjs [--max-age-days=90]
 *
 * Exit code is non-zero if any page is missing, malformed, or stale, so this
 * is safe to wire directly into CI as a build-failing step.
 */

import { readFileSync, readdirSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const DOCS_ROOT = path.dirname(fileURLToPath(import.meta.url));
const APP_DOCS_DIR = path.join(DOCS_ROOT, "..", "app", "docs");
const DEFAULT_MAX_AGE_DAYS = 90;
const LAST_REVIEWED_RE = /lastReviewed:\s*["'](\d{4}-\d{2}-\d{2})["']/;
const DATE_RE = /^\d{4}-\d{2}-\d{2}$/;

function parseMaxAgeDays(argv) {
  const flag = argv.find((a) => a.startsWith("--max-age-days="));
  if (!flag) return DEFAULT_MAX_AGE_DAYS;
  const value = Number(flag.split("=")[1]);
  if (!Number.isFinite(value) || value <= 0) {
    throw new Error(`Invalid --max-age-days value: ${flag}`);
  }
  return value;
}

function daysSince(dateStr, now) {
  const then = new Date(`${dateStr}T00:00:00Z`);
  return Math.floor((now.getTime() - then.getTime()) / (1000 * 60 * 60 * 24));
}

function findPages(dir) {
  const pages = [];
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      pages.push(...findPages(full));
    } else if (entry.isFile() && entry.name === "page.mdx") {
      pages.push(full);
    }
  }
  return pages.sort();
}

async function main() {
  const maxAgeDays = parseMaxAgeDays(process.argv.slice(2));
  const now = new Date();
  const pages = findPages(APP_DOCS_DIR);

  if (pages.length === 0) {
    console.error(`No page.mdx files found under ${APP_DOCS_DIR}`);
    process.exit(1);
  }

  const problems = [];

  for (const file of pages) {
    const rel = path.relative(path.join(DOCS_ROOT, ".."), file);
    const source = readFileSync(file, "utf8");
    const match = source.match(LAST_REVIEWED_RE);

    if (!match) {
      problems.push(`${rel}: missing metadata.lastReviewed`);
      continue;
    }

    const dateStr = match[1];
    if (!DATE_RE.test(dateStr)) {
      problems.push(`${rel}: lastReviewed "${dateStr}" is not YYYY-MM-DD`);
      continue;
    }

    const age = daysSince(dateStr, now);
    if (age > maxAgeDays) {
      problems.push(
        `${rel}: lastReviewed ${dateStr} is ${age} days old (max ${maxAgeDays})`,
      );
    }
  }

  if (problems.length > 0) {
    console.error(`Stale or missing lastReviewed metadata (${problems.length}):\n`);
    for (const problem of problems) console.error(`  - ${problem}`);
    console.error(
      "\nReview the page's content against the current contracts/API, then " +
        "bump its metadata.lastReviewed date.",
    );
    process.exit(1);
  }

  console.log(`All ${pages.length} doc pages have a lastReviewed date within ${maxAgeDays} days.`);
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
