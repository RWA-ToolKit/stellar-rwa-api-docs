"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import { useEffect, useRef, useState } from "react";
import { Sidebar } from "./Sidebar";
import { DEFAULT_LOCALE, formatDate } from "@/lib/formatDate";

const CONTRACTS_REPO = "https://github.com/your-org/stellar-rwa-contracts";
const WEB_REPO = "https://github.com/your-org/stellar-rwa-web";
const DOCS_REPO = "https://github.com/your-org/stellar-rwa-web";

/**
 * Build the GitHub edit URL for the current docs page. Docs routes map to
 * source files under `docs/app`, so `/docs/getting-started` becomes
 * `docs/app/docs/getting-started/page.tsx`.
 */
function editUrlForPath(pathname: string | null): string {
  const clean = (pathname ?? "/docs").replace(/\/+$/, "") || "/docs";
  return `${DOCS_REPO}/edit/main/docs/app${clean}/page.tsx`;
}

/** Top navigation bar with a mobile-only slide-in sidebar drawer. */
export function DocHeader() {
  const [open, setOpen] = useState(false);
  const pathname = usePathname();
  const editUrl = editUrlForPath(pathname);
  const toggleRef = useRef<HTMLButtonElement>(null);
  const closeRef = useRef<HTMLButtonElement>(null);
  const wasOpen = useRef(false);

  // Keyboard support for the drawer: focus moves into it on open, Escape
  // closes it, and focus returns to the menu toggle on close.
  useEffect(() => {
    if (!open) {
      if (wasOpen.current) toggleRef.current?.focus();
      wasOpen.current = false;
      return;
    }
    wasOpen.current = true;
    closeRef.current?.focus();
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") setOpen(false);
    };
    document.addEventListener("keydown", onKeyDown);
    return () => document.removeEventListener("keydown", onKeyDown);
  }, [open]);

  return (
    <>
      <header className="sticky top-0 z-40 border-b border-white/5 bg-base-950/80 backdrop-blur-xl">
        <div className="mx-auto flex h-16 max-w-screen-2xl items-center justify-between gap-4 px-4 sm:px-6">
          <div className="flex items-center gap-3">
            <button
              ref={toggleRef}
              type="button"
              className="rounded-lg p-2 text-base-200 hover:bg-white/5 focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-brand-400 lg:hidden"
              onClick={() => setOpen(true)}
              aria-label="Open navigation menu"
              aria-expanded={open}
              aria-controls="mobile-nav-drawer"
            >
              <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" aria-hidden="true">
                <path d="M3 12h18M3 6h18M3 18h18" strokeLinecap="round" />
              </svg>
            </button>
            <Link href="/" className="flex items-center gap-2.5 focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-brand-400 rounded">
              <span className="flex h-8 w-8 items-center justify-center rounded-lg bg-gradient-to-br from-brand-400 to-brand-600 text-base-950">
                <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.2" aria-hidden="true">
                  <path d="m12 2 8 4.5v9L12 20l-8-4.5v-9L12 2Z" strokeLinejoin="round" />
                  <path d="M12 8v8M8 10v4M16 10v4" strokeLinecap="round" />
                </svg>
              </span>
              <span className="text-sm font-bold tracking-tight text-base-50">
                Stellar<span className="text-brand-400">RWA</span>
                <span className="ml-1.5 font-normal text-base-300">Docs</span>
              </span>
            </Link>
          </div>

          <nav aria-label="Site navigation" className="flex items-center gap-1 text-sm">
            <Link href="/docs/getting-started" className="hidden rounded-lg px-3 py-2 text-base-200/70 hover:text-base-100 focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-brand-400 sm:block">
              Docs
            </Link>
            <a href={CONTRACTS_REPO} target="_blank" rel="noopener noreferrer" className="hidden rounded-lg px-3 py-2 text-base-200/70 hover:text-base-100 focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-brand-400 sm:block">
              Contracts <span aria-label="(opens in new tab)">↗</span>
            </a>
            <a href={WEB_REPO} target="_blank" rel="noopener noreferrer" className="rounded-lg px-3 py-2 text-base-200/70 hover:text-base-100 focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-brand-400">
              Web App <span aria-label="(opens in new tab)">↗</span>
            </a>
            <a
              href={editUrl}
              target="_blank"
              rel="noopener noreferrer"
              aria-label="Edit this page on GitHub"
              className="rounded-lg px-3 py-2 text-base-200/70 hover:text-base-100 focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-brand-400"
            >
              <span className="hidden sm:inline">Edit this page</span>
              <span className="sm:hidden">Edit</span> <span aria-label="(opens in new tab)">↗</span>
            </a>
          </nav>
        </div>
      </header>

      {/* Mobile drawer */}
      {open && (
        <div className="fixed inset-0 z-50 lg:hidden">
          {/* Backdrop */}
          <div
            className="absolute inset-0 bg-black/60"
            onClick={() => setOpen(false)}
            aria-hidden="true"
          />
          <div
            id="mobile-nav-drawer"
            role="dialog"
            aria-modal="true"
            aria-label="Navigation menu"
            className="absolute left-0 top-0 h-full w-72 overflow-y-auto border-r border-white/10 bg-base-900 p-5"
          >
            <div className="mb-5 flex items-center justify-between">
              <span className="text-sm font-semibold text-base-100">Navigation</span>
              <button
                ref={closeRef}
                type="button"
                onClick={() => setOpen(false)}
                aria-label="Close navigation menu"
                className="rounded-lg p-1.5 hover:bg-white/5 focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-brand-400"
              >
                <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" aria-hidden="true">
                  <path d="M18 6 6 18M6 6l12 12" strokeLinecap="round" />
                </svg>
              </button>
            </div>
            <Sidebar onNavigate={() => setOpen(false)} />
          </div>
        </div>
      )}
    </>
  );
}

/**
 * "Last reviewed" line shown under a page title. The date is maintained in
 * each page's `metadata.lastReviewed` export (ISO `YYYY-MM-DD`).
 */
export function LastReviewed({
  date,
  locale = DEFAULT_LOCALE,
}: {
  date?: string;
  /** BCP 47 locale for the visible date; defaults to `DEFAULT_LOCALE` ("en-US"). */
  locale?: string;
}) {
  if (!date) return null;
  const label = formatDate(date, locale);
  return (
    <p className="not-prose -mt-2 mb-6 text-sm text-base-300">
      Last reviewed: <time dateTime={date}>{label}</time>
    </p>
  );
}

export default DocHeader;
