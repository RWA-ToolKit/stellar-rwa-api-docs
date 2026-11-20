"use client";

import Link from "next/link";
import { useMemo, useState } from "react";
import { FLAT_NAV } from "./nav";

/**
 * Lightweight client-side filter over the flattened nav list. No index, no
 * network request — just a substring match against page titles, good enough
 * for the handful of pages this site has. Renders a text input plus a
 * dropdown of matches; picking one or pressing Enter on the top match
 * navigates there.
 */
export function DocSearch() {
  const [query, setQuery] = useState("");

  const matches = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return [];
    return FLAT_NAV.filter((item) => item.title.toLowerCase().includes(q)).slice(0, 8);
  }, [query]);

  return (
    <div className="relative mb-4 px-3">
      <label htmlFor="doc-search" className="sr-only">
        Search documentation
      </label>
      <input
        id="doc-search"
        type="search"
        value={query}
        onChange={(e) => setQuery(e.target.value)}
        placeholder="Search docs…"
        autoComplete="off"
        className="w-full rounded-lg border border-white/10 bg-white/5 px-3 py-1.5 text-sm text-base-100 placeholder:text-base-300/50 focus:border-brand-400/50 focus:outline-none"
      />
      {matches.length > 0 && (
        <ul className="absolute left-3 right-3 top-full z-10 mt-1 overflow-hidden rounded-lg border border-white/10 bg-base-900 shadow-xl">
          {matches.map((item) => (
            <li key={item.href}>
              <Link
                href={item.href}
                onClick={() => setQuery("")}
                className="block px-3 py-2 text-sm text-base-200/85 hover:bg-white/5 hover:text-base-100"
              >
                {item.title}
              </Link>
            </li>
          ))}
        </ul>
      )}
      {query.trim() && matches.length === 0 && (
        <p className="absolute left-3 right-3 top-full z-10 mt-1 rounded-lg border border-white/10 bg-base-900 px-3 py-2 text-sm text-base-300/70 shadow-xl">
          No pages match &quot;{query}&quot;.
        </p>
      )}
    </div>
  );
}

export default DocSearch;
