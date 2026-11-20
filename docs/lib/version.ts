/** Version these docs describe (bumped when contract interfaces/error codes change). */
export const DOCS_VERSION = process.env.NEXT_PUBLIC_API_VERSION || "v1.0.0";

/** Version currently deployed on-chain/in the API. Defaults to DOCS_VERSION (i.e. not stale). */
export const CONTRACT_VERSION = process.env.NEXT_PUBLIC_CONTRACT_VERSION || DOCS_VERSION;

/** Parses a "v1.2.3" / "1.2.3" version string into a [major, minor, patch] tuple. */
function parseVersion(version: string): [number, number, number] {
  const normalized = version.trim().replace(/^v/i, "");
  const [major = 0, minor = 0, patch = 0] = normalized.split(".").map((part) => {
    const n = parseInt(part, 10);
    return Number.isNaN(n) ? 0 : n;
  });
  return [major, minor, patch];
}

/**
 * Returns true when `docsVersion` is behind `contractVersion`, i.e. the
 * deployed contract/API has moved on to a newer version than these docs
 * describe.
 */
export function isVersionStale(docsVersion: string, contractVersion: string): boolean {
  const docs = parseVersion(docsVersion);
  const contract = parseVersion(contractVersion);

  for (let i = 0; i < 3; i++) {
    if (docs[i] < contract[i]) return true;
    if (docs[i] > contract[i]) return false;
  }
  return false;
}
