import navData from "./nav-data.json";

/** Documentation navigation tree, shared by the sidebar and page metadata. */

export interface NavItem {
  title: string;
  href: string;
}

export interface NavSection {
  title: string;
  items: NavItem[];
}

export const NAV: NavSection[] = navData;

/** Flattened, ordered list of all pages — used for prev/next navigation. */
export const FLAT_NAV: NavItem[] = NAV.flatMap((s) => s.items);
