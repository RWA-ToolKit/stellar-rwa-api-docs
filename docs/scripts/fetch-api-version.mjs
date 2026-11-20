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
const base = (process.env.API_BASE_URL || "").replace(/\/$/, "");
if (base) {
  try {
    const res = await fetch(`${base}/version`, { signal: AbortSignal.timeout(5000) });
    const body = await res.json();
    if (typeof body.release === "string") process.stdout.write(body.release);
  } catch {
    /* unreadable: print nothing */
  }
}
