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

/** Flattened, ordered list of all pages — used for prev/next navigation. */
export const FLAT_NAV: FlatNavItem[] = NAV.flatMap((section) =>
  section.items.map((item) => ({ ...item, section: section.title })),
);
