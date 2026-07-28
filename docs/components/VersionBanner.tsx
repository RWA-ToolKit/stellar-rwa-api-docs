import { DOCS_VERSION } from "@/lib/version";

export function VersionBanner() {
  if (!DOCS_VERSION) return null;

  return (
    <div className="bg-brand-900/30 border-l-4 border-brand-400 px-4 py-3 mb-6">
      <p className="text-sm text-brand-200">
        📋 <strong>API Version:</strong> These docs apply to <code className="bg-black/30 px-2 py-1 rounded text-xs">{DOCS_VERSION}</code> and later.
        Older deployments may have different contract interfaces and error codes.
      </p>
    </div>
  );
}
