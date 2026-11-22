#!/usr/bin/env node
/**
 * Prints the deployed API version from `GET <API_BASE_URL>/version` (the
 * `release` field) so it can feed the docs build:
 *
 *   NEXT_PUBLIC_CONTRACT_VERSION=$(API_BASE_URL=https://host node scripts/fetch-api-version.mjs) npm run build
 *
 * On any failure it prints nothing and exits 0, so the banner degrades
 * gracefully and the build is never blocked.
 */
import { pathToFileURL } from "node:url";

/**
 * Returns the `release` string from `GET <baseUrl>/version`, or "" on any
 * failure (empty base, network error, bad JSON, missing/non-string release).
 */
export async function fetchApiVersion(baseUrl, fetchImpl = fetch) {
  const base = (baseUrl || "").replace(/\/$/, "");
  if (!base) return "";
  try {
    const res = await fetchImpl(`${base}/version`, { signal: AbortSignal.timeout(5000) });
    const body = await res.json();
    return typeof body?.release === "string" ? body.release : "";
  } catch {
    /* unreadable: print nothing */
    return "";
  }
}

export async function main() {
  process.stdout.write(await fetchApiVersion(process.env.API_BASE_URL));
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) await main();
