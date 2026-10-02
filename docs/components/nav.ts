/** Documentation navigation tree, shared by the sidebar and page metadata. */

import navData from "./nav-data.json";

export interface NavItem {
  title: string;
  href: string;
}

export interface NavSection {
  title: string;
  items: NavItem[];
}

export interface FlatNavItem extends NavItem {
  section: string;
}

export const NAV: NavSection[] = navData;
export const NAV: NavSection[] = [
  {
    title: "Introduction",
    items: [
      { title: "Getting Started", href: "/docs/getting-started" },
      { title: "Architecture Overview", href: "/docs/architecture" },
    ],
  },
  {
    title: "Concepts",
    items: [
      { title: "Compliance Guide", href: "/docs/compliance-guide" },
      { title: "Time & Ledgers", href: "/docs/time-and-ledgers" },
      { title: "Glossary", href: "/docs/glossary" },
      { title: "Security Considerations", href: "/docs/security-considerations" },
    ],
  },
  {
    title: "Guides",
    items: [
      { title: "Integration", href: "/docs/integration" },
      { title: "Web App Guide", href: "/docs/web-app" },
      { title: "Troubleshooting", href: "/docs/troubleshooting" },
      { title: "FAQ", href: "/docs/faq" },
    ],
  },
  {
    title: "Contract Reference",
    items: [
      { title: "Asset Token", href: "/docs/contracts/asset-token" },
      { title: "Compliance", href: "/docs/contracts/compliance" },
      { title: "Registry", href: "/docs/contracts/registry" },
      { title: "Dividend", href: "/docs/contracts/dividend" },
    ],
  },
  {
    title: "API Reference",
    items: [
      { title: "Overview", href: "/docs/api/overview" },
      { title: "Assets", href: "/docs/api/assets" },
      { title: "Holders", href: "/docs/api/holders" },
      { title: "Compliance", href: "/docs/api/compliance" },
      { title: "Dividends", href: "/docs/api/dividends" },
      { title: "Events", href: "/docs/api/events" },
      { title: "Rate Limits & Caching", href: "/docs/api/rate-limits" },
      { title: "Changelog", href: "/docs/changelog" },
    ],
  },
];

/** Flattened, ordered list of all pages — used for prev/next navigation. */
export const FLAT_NAV: FlatNavItem[] = NAV.flatMap((section) =>
  section.items.map((item) => ({ ...item, section: section.title })),
);
