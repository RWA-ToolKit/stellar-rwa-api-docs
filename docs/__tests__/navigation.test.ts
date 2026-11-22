import { describe, expect, it } from "vitest";
import { existsSync, readFileSync, readdirSync } from "node:fs";
import { join, relative, sep } from "node:path";
import { FLAT_NAV, NAV } from "../components/nav";
import { getAllPages, search } from "../lib/search";
import sitemap from "../app/sitemap";

const appRoot = join(__dirname, "..", "app");
const docsRoot = join(appRoot, "docs");
const navDataPath = join(__dirname, "..", "components", "nav-data.json");

/** Maps a nav href (e.g. "/docs/foo#bar") to its page.mdx path (app/docs/foo/page.mdx). */
function pageFileForHref(href: string): string {
  const pathname = href.split("#")[0].split("?")[0].replace(/\/+$/, "");
  return join(appRoot, ...pathname.split("/").filter(Boolean), "page.mdx");
}

/** Pilot translation locales (/docs/<locale>/<page>); not part of the English nav/search/sitemap. */
const LOCALE_DIRS = ["es"];

function findMdxRoutes(directory: string): string[] {
  return readdirSync(directory, { withFileTypes: true }).flatMap((entry) => {
    const entryPath = join(directory, entry.name);
    if (entry.isDirectory()) {
      if (directory === docsRoot && LOCALE_DIRS.includes(entry.name)) return [];
      return findMdxRoutes(entryPath);
    }
    if (entry.name !== "page.mdx") return [];

    const route = relative(docsRoot, entryPath)
      .split(sep)
      .join("/")
      .replace(/\/page\.mdx$/, "");
    return [`/docs/${route}`];
  });
}

describe("documentation route registration", () => {
  it("includes every MDX page in navigation, search pages, and the sitemap", () => {
    const routes = findMdxRoutes(docsRoot);
    const navRoutes = new Set(NAV.flatMap((section) => section.items.map((item) => item.href)));
    const searchRoutes = new Set(getAllPages().map((page) => page.href));
    const sitemapRoutes = new Set(sitemap().map((entry) => new URL(entry.url).pathname));

    for (const route of routes) {
      expect(navRoutes).toContain(route);
      expect(searchRoutes).toContain(route);
      expect(sitemapRoutes).toContain(route);
    }

    const pages = getAllPages();
    expect(pages.map((page) => page.href)).toEqual(FLAT_NAV.map((page) => page.href));
    expect(pages.every((page) => page.excerpt.length > 0)).toBe(true);

    const formerlyMissingPages = [
      { query: "Architecture Overview", href: "/docs/architecture" },
      { query: "Rate Limits & Caching", href: "/docs/api/rate-limits" },
      { query: "Time & Ledgers", href: "/docs/time-and-ledgers" },
      { query: "Events", href: "/docs/api/events" },
    ];

    for (const { query, href } of formerlyMissingPages) {
      expect(search(query).some((result) => result.href.split("#")[0] === href)).toBe(true);
    }
  });

  it("has an existing page.mdx file for every nav-data.json href", () => {
    const navData: { title: string; items: { title: string; href: string }[] }[] = JSON.parse(
      readFileSync(navDataPath, "utf8"),
    );
    const hrefs = navData.flatMap((section) => section.items.map((item) => item.href));
    expect(hrefs.length).toBeGreaterThan(0);

    const missing = hrefs.filter((href) => !existsSync(pageFileForHref(href)));
    expect(missing, `nav-data.json hrefs with no page.mdx: ${missing.join(", ")}`).toEqual([]);

    // nav.ts must expose exactly the hrefs defined in nav-data.json.
    expect(FLAT_NAV.map((item) => item.href)).toEqual(hrefs);
  });
});
