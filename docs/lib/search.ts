import searchIndexData from "./search-index.json";

export interface SearchResult {
  title: string;
  href: string;
  section: string;
  excerpt: string;
}

export interface SearchDocument {
  id: string;
  title: string;
  pageTitle: string;
  href: string;
  section: string;
  description: string;
  content: string;
}
const PAGES = [
  { title: "Getting Started", href: "/docs/getting-started", section: "Introduction" },
  { title: "Security Considerations", href: "/docs/security-considerations", section: "Guides" },
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

const DOCUMENTS: SearchDocument[] = searchIndexData as SearchDocument[];

/**
 * Extracts a contextual snippet around matching query terms in content,
 * trimmed cleanly at word boundaries with ellipsis.
 */
function extractSnippet(content: string, queryTerms: string[], fullQuery: string): string {
  if (!content) return "";

  const contentLower = content.toLowerCase();

  // Try finding exact full query first
  let matchIndex = contentLower.indexOf(fullQuery.toLowerCase());

  // Otherwise find the first matching term
  if (matchIndex === -1) {
    for (const term of queryTerms) {
      const idx = contentLower.indexOf(term);
      if (idx !== -1) {
        matchIndex = idx;
        break;
      }
    }
  }

  if (matchIndex === -1) {
    const snippet = content.slice(0, 140).trim();
    return snippet.length < content.length ? `${snippet}...` : snippet;
  }

  // Extract window around matchIndex
  const start = Math.max(0, matchIndex - 50);
  const end = Math.min(content.length, matchIndex + fullQuery.length + 90);

  let snippet = content.slice(start, end).trim();

  // Trim to nearest word boundaries
  if (start > 0) {
    const firstSpace = snippet.indexOf(" ");
    if (firstSpace !== -1 && firstSpace < 20) {
      snippet = snippet.slice(firstSpace + 1);
    }
    snippet = "..." + snippet;
  }

  if (end < content.length) {
    const lastSpace = snippet.lastIndexOf(" ");
    if (lastSpace !== -1 && lastSpace > snippet.length - 20) {
      snippet = snippet.slice(0, lastSpace);
    }
    snippet = snippet + "...";
  }

  return snippet;
}

/**
 * Client-side documentation search matching titles, descriptions, and body content.
 */
export function search(query: string): SearchResult[] {
  if (!query || !query.trim()) return [];

  const rawQuery = query.trim();
  const normalizedQuery = rawQuery.toLowerCase();
  const terms = normalizedQuery.split(/\s+/).filter(Boolean);

  if (terms.length === 0) return [];

  const scoredResults: { doc: SearchDocument; score: number }[] = [];

  for (const doc of DOCUMENTS) {
    const titleLower = doc.title.toLowerCase();
    const pageTitleLower = doc.pageTitle.toLowerCase();
    const contentLower = doc.content.toLowerCase();
    const descLower = (doc.description || "").toLowerCase();

    let score = 0;
    let termsMatched = 0;

    // Check full query match
    if (titleLower === normalizedQuery) {
      score += 200;
    } else if (titleLower.includes(normalizedQuery)) {
      score += 100;
    }

    if (descLower.includes(normalizedQuery)) {
      score += 40;
    }

    if (contentLower.includes(normalizedQuery)) {
      score += 50;
    }

    // Check individual terms
    for (const term of terms) {
      let termMatched = false;

      if (titleLower.includes(term)) {
        score += 30;
        termMatched = true;
      }
      if (pageTitleLower.includes(term)) {
        score += 20;
        termMatched = true;
      }
      if (descLower.includes(term)) {
        score += 15;
        termMatched = true;
      }

      // Check occurrences in content
      if (contentLower.includes(term)) {
        termMatched = true;
        let count = 0;
        let pos = 0;
        while ((pos = contentLower.indexOf(term, pos)) !== -1 && count < 5) {
          count++;
          pos += term.length;
        }
        score += count * 6;
      }

      if (termMatched) {
        termsMatched++;
      }
    }

    if (terms.length > 1) {
      if (termsMatched === terms.length) {
        score += 40;
      } else if (termsMatched === 0) {
        continue;
      } else {
        score = score * (termsMatched / terms.length);
      }
    } else if (termsMatched === 0) {
      continue;
    }

    if (score > 0) {
      scoredResults.push({ doc, score });
    }
  }

  scoredResults.sort((a, b) => b.score - a.score);

  // Group/diversify results: at most 2 results from the same page
  const pageCounts = new Map<string, number>();
  const finalResults: SearchResult[] = [];

  for (const { doc } of scoredResults) {
    const count = pageCounts.get(doc.pageTitle) || 0;
    if (count >= 2) continue;
    pageCounts.set(doc.pageTitle, count + 1);

    const excerpt = extractSnippet(doc.content, terms, rawQuery) || doc.description || "";

    finalResults.push({
      title: doc.title,
      href: doc.href,
      section: doc.section,
      excerpt,
    });

    if (finalResults.length >= 8) break;
  }

  return finalResults;
}

/**
 * Returns all unique documentation pages.
 */
export function getAllPages(): SearchResult[] {
  const seenHrefs = new Set<string>();
  const pages: SearchResult[] = [];

  for (const doc of DOCUMENTS) {
    const baseHref = doc.href.split("#")[0];
    if (!seenHrefs.has(baseHref)) {
      seenHrefs.add(baseHref);
      pages.push({
        title: doc.pageTitle,
        href: baseHref,
        section: doc.section,
        excerpt: doc.description || "",
      });
    }
  }

  return pages;
}
