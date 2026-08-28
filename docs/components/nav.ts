/** Documentation navigation tree, shared by the sidebar and page metadata. */

export interface NavItem {
  title: string;
  href: string;
}

export interface NavSection {
  title: string;
  items: NavItem[];
}

export const NAV: NavSection[] = [
  {
    title: "Introduction",
    items: [
      { title: "Getting Started", href: "/docs/getting-started" },
      { title: "Architecture Overview", href: "/docs/architecture" },
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
    ],
  },
  {
    title: "Guides",
    items: [
      { title: "Compliance Guide", href: "/docs/compliance-guide" },
      { title: "Time & Ledgers", href: "/docs/time-and-ledgers" },
      { title: "Web App Guide", href: "/docs/web-app" },
      { title: "Integration", href: "/docs/integration" },
    ],
  },
];

/** Flattened, ordered list of all pages — used for prev/next navigation. */
export const FLAT_NAV: NavItem[] = NAV.flatMap((s) => s.items);
