"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import { NAV } from "./nav";
import { Search } from "./Search";

/** Left-hand documentation navigation with active-page highlighting. */
export function Sidebar({ onNavigate }: { onNavigate?: () => void }) {
  const pathname = usePathname();

  return (
    <nav
      className="w-full max-w-full space-y-7 overflow-x-hidden text-sm"
      aria-label="Documentation"
    >
      <Search />
      {NAV.map((section) => (
        <div key={section.title}>
          <p className="mb-2 px-3 text-xs font-semibold uppercase tracking-wide text-base-300">
            {section.title}
          </p>
          <ul className="space-y-0.5">
            {section.items.map((item) => {
              const active = pathname === item.href;
              return (
                <li key={item.href}>
                  <Link
                    href={item.href}
                    onClick={onNavigate}
                    aria-current={active ? "page" : undefined}
                    className={`block truncate rounded-lg px-3 py-1.5 transition-colors focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-brand-400 ${
                      active
                        ? "bg-brand-500/10 font-medium text-brand-300"
                        : "text-base-200 hover:bg-white/5 hover:text-base-50"
                    }`}
                  >
                    {item.title}
                  </Link>
                </li>
              );
            })}
          </ul>
        </div>
      ))}
    </nav>
  );
}

export default Sidebar;
