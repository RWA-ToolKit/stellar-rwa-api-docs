import { expect, it } from "vitest";
import robots from "../app/robots";
import sitemap from "../app/sitemap";
import sections from "../components/nav-data.json";
import { SITE_URL } from "../lib/site";
import redirects from "../redirects.json";

it("allows crawlers across the site and advertises the canonical sitemap", () => {
  expect(robots()).toEqual({
    rules: { userAgent: "*", allow: "/" },
    sitemap: `${SITE_URL}/sitemap.xml`,
  });
});

it("publishes the homepage and navigation URLs and excludes redirect sources", () => {
  const urls = sitemap().map(entry => entry.url);
  const paths = ["/", ...sections.flatMap(section => section.items.map(item => item.href))];
  expect(urls.sort()).toEqual(paths.map(path => SITE_URL + path).sort());
  for (const { source } of redirects.redirects) {
    expect(urls).not.toContain(SITE_URL + source);
  }
});
