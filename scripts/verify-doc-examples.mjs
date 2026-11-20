#!/usr/bin/env node
/**
 * Compares the JSON response examples documented under docs/app against a
 * live (or mock) API. Drift is reported, never fixed.
 *
 * For each `curl <API_BASE>/<path>` fence in an .mdx page, the next ```json
 * fence is taken as the documented response. The live response is fetched and
 * the two are compared by shape (key names and JSON value types), since
 * values such as balances or timestamps legitimately differ.
 *
 * Usage:
 *   API_BASE_URL=https://your-testnet-api.example node scripts/verify-doc-examples.mjs
 *   API_BASE_URL=http://localhost:8080 node scripts/verify-doc-examples.mjs   # mock/local
 *
 * Exit code 1 when drift is found, 2 when API_BASE_URL is missing.
 * Not run automatically on pull requests; see .github/workflows/doc-examples-drift.yml.
 */
import { readFileSync, readdirSync, statSync } from "node:fs";
import { join } from "node:path";

const base = (process.env.API_BASE_URL || "").replace(/\/$/, "");
if (!base) {
  console.error("API_BASE_URL is required");
  process.exit(2);
}
const ROOT = join(process.cwd(), "docs", "app");

function walk(dir) {
  return readdirSync(dir).flatMap((f) => {
    const p = join(dir, f);
    return statSync(p).isDirectory() ? walk(p) : p.endsWith(".mdx") ? [p] : [];
  });
}

function shape(v) {
  if (Array.isArray(v)) return v.length ? [shape(v[0])] : [];
  if (v && typeof v === "object") {
    return Object.fromEntries(Object.keys(v).sort().map((k) => [k, shape(v[k])]));
  }
  return v === null ? "null" : typeof v;
}

function diff(a, b, path = "$") {
  if (typeof a !== typeof b) return [`${path}: documented ${JSON.stringify(a)}, live ${JSON.stringify(b)}`];
  if (Array.isArray(a)) return a.length && b.length ? diff(a[0], b[0], `${path}[0]`) : [];
  if (a && typeof a === "object") {
    const out = [];
    for (const k of new Set([...Object.keys(a), ...Object.keys(b)])) {
      if (!(k in b)) out.push(`${path}.${k}: documented but missing from live response`);
      else if (!(k in a)) out.push(`${path}.${k}: in live response but not documented`);
      else out.push(...diff(a[k], b[k], `${path}.${k}`));
    }
    return out;
  }
  return a === b ? [] : [`${path}: documented ${a}, live ${b}`];
}

let drift = 0;
let checked = 0;
for (const file of walk(ROOT)) {
  const text = readFileSync(file, "utf8");
  const re = /```(?:bash|sh)\n[^`]*?curl\s+(?:-\S+\s+)*"?https?:\/\/[^/\s"]+(\/[^\s"\\]*)"?[^`]*```\s*[^`]*?```json\n([\s\S]*?)```/g;
  let m;
  while ((m = re.exec(text))) {
    const [, path, body] = m;
    let doc;
    try {
      doc = shape(JSON.parse(body));
    } catch {
      continue; // fragment or pseudo-JSON; skip
    }
    checked++;
    try {
      const res = await fetch(base + path);
      const live = shape(await res.json());
      const d = diff(doc, live);
      if (d.length) {
        drift++;
        console.log(`DRIFT ${file} GET ${path}\n  ${d.join("\n  ")}`);
      }
    } catch (e) {
      drift++;
      console.log(`DRIFT ${file} GET ${path}\n  request failed: ${e.message}`);
    }
  }
}
console.log(`Checked ${checked} example(s), ${drift} with drift.`);
process.exit(drift ? 1 : 0);
