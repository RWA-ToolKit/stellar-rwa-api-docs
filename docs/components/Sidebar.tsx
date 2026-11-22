"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import { useId } from "react";
import { NAV } from "./nav";
import { DocSearch } from "./DocSearch";

/**
 * Left-hand documentation navigation with active-page highlighting.
 *
 * Keyboard model: sections are always expanded (no disclosure widgets), so the
 * tree is plain links in labelled lists — Tab/Shift+Tab move between links and
 * Enter follows them. No roving tabindex or ARIA tree/menu role is used on
 * purpose; native link semantics are the correct pattern for site navigation.
 * The mobile open/close toggle lives in DocHeader.
 */
export function Sidebar({ onNavigate }: { onNavigate?: () => void }) {
  const pathname = usePathname();
  // Sidebar renders twice (desktop aside + mobile drawer), so ids must be unique per instance.
  const idPrefix = useId();
  const current = pathname ? pathname.replace(/\/+$/, "") || "/" : null;

  return (
    <nav className="space-y-7 text-sm" aria-label="Documentation">
      <DocSearch />
      {NAV.map((section, sectionIndex) => {
        const headingId = `${idPrefix}-section-${sectionIndex}`;
        return (
          <div key={section.title}>
            <p
              id={headingId}
              className="mb-2 px-3 text-xs font-semibold uppercase tracking-wide text-base-300/70"
            >
              {section.title}
            </p>
            <ul className="space-y-0.5" aria-labelledby={headingId}>
              {section.items.map((item) => {
                const active = current === item.href;
                return (
                  <li key={item.href}>
                    <Link
                      href={item.href}
                      onClick={onNavigate}
                      aria-current={active ? "page" : undefined}
                      className={`block rounded-lg px-3 py-1.5 transition-colors focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-brand-400 ${
                        active
                          ? "bg-brand-500/10 font-medium text-brand-300"
                          : "text-base-200/70 hover:bg-white/5 hover:text-base-100"
                      }`}
                    >
                      {item.title}
                    </Link>
                  </li>
                );
              })}
            </ul>
          </div>
        );
      })}
    </nav>
  );
}

export default Sidebar;
