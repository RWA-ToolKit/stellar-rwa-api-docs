"use client";

import { useState, useRef, useEffect } from "react";
import Link from "next/link";
import { search, type SearchResult } from "@/lib/search";

/** Client-side documentation search with keyboard shortcuts. */
export function Search() {
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<SearchResult[]>([]);
  const [isOpen, setIsOpen] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (query.trim()) {
      setResults(search(query));
      setIsOpen(true);
    } else {
      setResults([]);
      setIsOpen(false);
    }
  }, [query]);

  useEffect(() => {
    function handleKeyDown(e: KeyboardEvent) {
      if ((e.metaKey || e.ctrlKey) && e.key === "k") {
        e.preventDefault();
        inputRef.current?.focus();
      }
      if (e.key === "Escape") {
        setIsOpen(false);
      }
    }

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, []);

  return (
    <div className="mb-6">
      <div className="relative">
        <input
          ref={inputRef}
          type="text"
          placeholder="Search docs... (⌘K)"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          onFocus={() => query && setIsOpen(true)}
          className="w-full rounded-lg border border-base-300/30 bg-base-900/40 px-3 py-2 text-sm text-base-100 placeholder-base-300/50 transition-colors focus:border-brand-500/50 focus:outline-none focus:ring-1 focus:ring-brand-500/20"
        />
        {isOpen && results.length > 0 && (
          <div className="absolute top-full z-50 mt-2 w-full overflow-hidden rounded-lg border border-base-300/20 bg-base-900 shadow-lg">
            <ul className="max-h-96 overflow-y-auto py-1">
              {results.map((result) => (
                <li key={result.href}>
                  <Link
                    href={result.href}
                    onClick={() => {
                      setQuery("");
                      setIsOpen(false);
                    }}
                    className="block px-3 py-2 text-sm transition-colors hover:bg-base-800"
                  >
                    <div className="font-medium text-base-100">{result.title}</div>
                    <div className="text-xs text-base-300/60">{result.section}</div>
                    {result.excerpt && (
                      <div className="mt-1 line-clamp-1 text-xs text-base-200/50">
                        {result.excerpt}
                      </div>
                    )}
                  </Link>
                </li>
              ))}
            </ul>
          </div>
        )}
      </div>
    </div>
  );
}
