import searchIndexData from "./search-index.json";
import { FLAT_NAV } from "@/components/nav";

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
const PAGES = FLAT_NAV;

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
  return PAGES.flatMap((page) => {
    const document = DOCUMENTS.find((doc) => doc.href === page.href);
    if (!document) return [];

    return [{
      title: page.title,
      href: page.href,
      section: page.section,
      excerpt: document.description || "",
    }];
  });
}
