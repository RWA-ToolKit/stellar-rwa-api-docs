import { NAV } from "@/components/nav";

export interface SearchResult {
  title: string;
  href: string;
  section: string;
  excerpt: string;
}

const PAGES = [
  { title: "Getting Started", href: "/docs/getting-started", section: "Introduction" },
  { title: "Asset Token", href: "/docs/contracts/asset-token", section: "Contract Reference" },
  { title: "Compliance", href: "/docs/contracts/compliance", section: "Contract Reference" },
  { title: "Registry", href: "/docs/contracts/registry", section: "Contract Reference" },
  { title: "Dividend", href: "/docs/contracts/dividend", section: "Contract Reference" },
  { title: "Overview", href: "/docs/api/overview", section: "API Reference" },
  { title: "Assets", href: "/docs/api/assets", section: "API Reference" },
  { title: "Holders", href: "/docs/api/holders", section: "API Reference" },
  { title: "Compliance", href: "/docs/api/compliance", section: "API Reference" },
  { title: "Dividends", href: "/docs/api/dividends", section: "API Reference" },
  { title: "Compliance Guide", href: "/docs/compliance-guide", section: "Guides" },
  { title: "Web App Guide", href: "/docs/web-app", section: "Guides" },
  { title: "Integration", href: "/docs/integration", section: "Guides" },
];

const KEYWORDS: Record<string, SearchResult[]> = {
  asset: [
    { title: "Assets", href: "/docs/api/assets", section: "API Reference", excerpt: "List all tokenized assets with valuation, supply and holder counts." },
    { title: "Asset Token", href: "/docs/contracts/asset-token", section: "Contract Reference", excerpt: "Compliant RWA token contract for transferring assets." },
  ],
  compliance: [
    { title: "Compliance", href: "/docs/api/compliance", section: "API Reference", excerpt: "Non-PII compliance summary for a tokenized asset." },
    { title: "Compliance", href: "/docs/contracts/compliance", section: "Contract Reference", excerpt: "Gate contract for allowlist-based compliance." },
    { title: "Compliance Guide", href: "/docs/compliance-guide", section: "Guides", excerpt: "How to implement transfer gating and KYC." },
  ],
  api: [
    { title: "Overview", href: "/docs/api/overview", section: "API Reference", excerpt: "REST service for indexed tokenized asset activity." },
    { title: "Assets", href: "/docs/api/assets", section: "API Reference", excerpt: "List all tokenized assets." },
    { title: "Holders", href: "/docs/api/holders", section: "API Reference", excerpt: "Holder list with balances for an asset." },
    { title: "Dividends", href: "/docs/api/dividends", section: "API Reference", excerpt: "Distribution history for an asset." },
  ],
  contract: [
    { title: "Asset Token", href: "/docs/contracts/asset-token", section: "Contract Reference", excerpt: "Compliant RWA token contract." },
    { title: "Compliance", href: "/docs/contracts/compliance", section: "Contract Reference", excerpt: "Allowlist gate contract." },
    { title: "Registry", href: "/docs/contracts/registry", section: "Contract Reference", excerpt: "Asset registry contract." },
    { title: "Dividend", href: "/docs/contracts/dividend", section: "Contract Reference", excerpt: "Distribution contract." },
  ],
  transfer: [
    { title: "Asset Token", href: "/docs/contracts/asset-token", section: "Contract Reference", excerpt: "Learn how transfer gating works." },
    { title: "Integration", href: "/docs/integration", section: "Guides", excerpt: "How to submit transactions." },
  ],
  integration: [
    { title: "Integration", href: "/docs/integration", section: "Guides", excerpt: "Query the API and submit transactions." },
    { title: "Getting Started", href: "/docs/getting-started", section: "Introduction", excerpt: "Quickstart guide." },
  ],
  rest: [
    { title: "Overview", href: "/docs/api/overview", section: "API Reference", excerpt: "Read-only REST service." },
  ],
  holder: [
    { title: "Holders", href: "/docs/api/holders", section: "API Reference", excerpt: "Holder list with balances." },
  ],
  dividend: [
    { title: "Dividends", href: "/docs/api/dividends", section: "API Reference", excerpt: "Distribution history." },
    { title: "Dividend", href: "/docs/contracts/dividend", section: "Contract Reference", excerpt: "Distribution contract." },
  ],
  registry: [
    { title: "Registry", href: "/docs/contracts/registry", section: "Contract Reference", excerpt: "Asset registry contract." },
  ],
  kyc: [
    { title: "Compliance Guide", href: "/docs/compliance-guide", section: "Guides", excerpt: "KYC and allowlist procedures." },
  ],
  soroban: [
    { title: "Getting Started", href: "/docs/getting-started", section: "Introduction", excerpt: "Soroban smart contracts for RWA." },
  ],
  stellar: [
    { title: "Getting Started", href: "/docs/getting-started", section: "Introduction", excerpt: "Stellar RWA Toolkit overview." },
  ],
};

export function search(query: string): SearchResult[] {
  if (!query.trim()) return [];

  const normalized = query.toLowerCase().trim();
  const results: Map<string, SearchResult> = new Map();

  // Keyword-based search
  for (const [keyword, matches] of Object.entries(KEYWORDS)) {
    if (keyword.includes(normalized) || normalized.includes(keyword)) {
      matches.forEach((result) => {
        results.set(result.href, result);
      });
    }
  }

  // Title-based search
  PAGES.forEach((page) => {
    if (page.title.toLowerCase().includes(normalized)) {
      if (!results.has(page.href)) {
        results.set(page.href, {
          ...page,
          excerpt: "",
        });
      }
    }
  });

  // Return results, limit to 8
  return Array.from(results.values()).slice(0, 8);
}

export function getAllPages(): SearchResult[] {
  return PAGES.map((page) => ({
    ...page,
    excerpt: "",
  }));
}
