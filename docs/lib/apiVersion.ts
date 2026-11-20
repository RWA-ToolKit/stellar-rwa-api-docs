/**
 * Resolves the deployed API version for the version banner.
 *
 * The value comes from the build: set NEXT_PUBLIC_CONTRACT_VERSION, e.g. from
 * the API's `GET /version` endpoint (see docs/scripts/fetch-api-version.mjs).
 * If the value is missing or not a version string, fall back to `fallback`
 * so the banner degrades to hidden instead of showing garbage.
 */
const VERSION_RE = /^v?\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?$/;

export function resolveDeployedVersion(fallback: string, raw?: string): string {
  const candidate = (raw ?? "").trim();
  return VERSION_RE.test(candidate) ? candidate : fallback;
}
