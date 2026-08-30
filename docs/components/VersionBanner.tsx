import { DOCS_VERSION, CONTRACT_VERSION, isVersionStale } from "@/lib/version";

interface VersionBannerProps {
  /** Defaults to DOCS_VERSION; overridable for testing. */
  docsVersion?: string;
  /** Defaults to CONTRACT_VERSION; overridable for testing. */
  contractVersion?: string;
}

export function VersionBanner({
  docsVersion = DOCS_VERSION,
  contractVersion = CONTRACT_VERSION,
}: VersionBannerProps = {}) {
  if (!docsVersion) return null;
  if (!isVersionStale(docsVersion, contractVersion)) return null;

  return (
    <div className="bg-brand-900/30 border-l-4 border-brand-400 px-4 py-3 mb-6">
      <p className="text-sm text-brand-200">
        📋 <strong>API Version:</strong> These docs apply to <code className="bg-black/30 px-2 py-1 rounded text-xs">{docsVersion}</code>.
        The deployed API is now on <code className="bg-black/30 px-2 py-1 rounded text-xs">{contractVersion}</code>, which
        may have different contract interfaces and error codes.
      </p>
    </div>
  );
}
