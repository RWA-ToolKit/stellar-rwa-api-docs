import { describe, it, expect } from "vitest";
import { metadata } from "../app/layout";
import { SITE_URL } from "../lib/site";

/**
 * Guards against OpenGraph/Twitter tags silently regressing to relative
 * URLs. Next.js only resolves relative `openGraph.url` / image paths to
 * absolute URLs when `metadataBase` is set — without it, crawlers and
 * social previews receive broken relative links.
 */
describe("Root metadata", () => {
  it("sets metadataBase to an absolute URL matching SITE_URL", () => {
    expect(metadata.metadataBase).toBeInstanceOf(URL);
    expect(metadata.metadataBase?.toString()).toBe(new URL(SITE_URL).toString());
  });

  it("resolves openGraph.url to an absolute URL", () => {
    const ogUrl = metadata.openGraph?.url;
    expect(ogUrl).toBeDefined();
    expect(() => new URL(String(ogUrl))).not.toThrow();
    expect(String(ogUrl).startsWith("http")).toBe(true);
  });

  it("defines openGraph title, description and type", () => {
    expect(metadata.openGraph?.title).toBeDefined();
    expect(metadata.openGraph?.description).toBeTruthy();
    expect(metadata.openGraph?.type).toBe("website");
  });

  it("defines a twitter summary_large_image card with title and description", () => {
    expect(metadata.twitter?.card).toBe("summary_large_image");
    expect(metadata.twitter?.title).toBeDefined();
    expect(metadata.twitter?.description).toBeTruthy();
  });
});
