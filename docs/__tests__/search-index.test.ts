import { afterAll, beforeAll, describe, expect, it, vi } from "vitest";
import { execFileSync } from "node:child_process";
import {
  copyFileSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  symlinkSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import type { SearchDocument } from "../lib/search";

const docsRoot = join(__dirname, "..");
const committedIndexPath = join(docsRoot, "lib", "search-index.json");

/**
 * Runs scripts/build-search-index.mjs inside a throwaway sandbox so the
 * committed lib/search-index.json is never touched. The script resolves its
 * inputs and output relative to its own location (`../app/docs`,
 * `../components/nav-data.json`, `../lib/search-index.json`), so a copy of the
 * script next to symlinked `app/` and `components/` directories writes the
 * fresh index into the sandbox's `lib/` instead.
 */
function buildFreshIndex(sandbox: string): SearchDocument[] {
  mkdirSync(join(sandbox, "scripts"));
  mkdirSync(join(sandbox, "lib"));
  copyFileSync(
    join(docsRoot, "scripts", "build-search-index.mjs"),
    join(sandbox, "scripts", "build-search-index.mjs"),
  );
  const linkType = process.platform === "win32" ? "junction" : "dir";
  symlinkSync(join(docsRoot, "app"), join(sandbox, "app"), linkType);
  symlinkSync(join(docsRoot, "components"), join(sandbox, "components"), linkType);

  execFileSync(process.execPath, [join(sandbox, "scripts", "build-search-index.mjs")], {
    cwd: sandbox,
    stdio: "pipe",
  });

  return JSON.parse(readFileSync(join(sandbox, "lib", "search-index.json"), "utf-8"));
}

describe("search index freshness", () => {
  let sandbox: string;
  let freshIndex: SearchDocument[];
  let committedRaw: string;

  beforeAll(() => {
    committedRaw = readFileSync(committedIndexPath, "utf-8");
    sandbox = mkdtempSync(join(tmpdir(), "search-index-"));
    freshIndex = buildFreshIndex(sandbox);
  });

  afterAll(() => {
    if (sandbox) rmSync(sandbox, { recursive: true, force: true });
  });

  it("generates a non-empty index with the expected document shape", () => {
    expect(Array.isArray(freshIndex)).toBe(true);
    expect(freshIndex.length).toBeGreaterThan(0);

    for (const doc of freshIndex) {
      expect(typeof doc.id).toBe("string");
      expect(typeof doc.title).toBe("string");
      expect(typeof doc.pageTitle).toBe("string");
      expect(doc.href.startsWith("/")).toBe(true);
      expect(typeof doc.section).toBe("string");
      expect(typeof doc.description).toBe("string");
      expect(typeof doc.content).toBe("string");
    }
  });

  it("does not modify the committed search-index.json", () => {
    expect(readFileSync(committedIndexPath, "utf-8")).toBe(committedRaw);
  });

  it("matches the committed search-index.json (run `node scripts/build-search-index.mjs` to regenerate)", () => {
    expect(JSON.parse(committedRaw)).toEqual(freshIndex);
  });

  describe("lib/search.ts against the freshly built index", () => {
    let searchModule: typeof import("../lib/search");

    beforeAll(async () => {
      vi.resetModules();
      vi.doMock("../lib/search-index.json", () => ({ default: freshIndex }));
      searchModule = await import("../lib/search");
    });

    afterAll(() => {
      vi.doUnmock("../lib/search-index.json");
      vi.resetModules();
    });

    it("returns results for a known query without throwing", () => {
      let results: ReturnType<typeof searchModule.search> = [];
      expect(() => {
        results = searchModule.search("assets");
      }).not.toThrow();

      expect(results.length).toBeGreaterThan(0);
      expect(results.length).toBeLessThanOrEqual(8);
      const freshHrefs = new Set(freshIndex.map((doc) => doc.href));
      for (const result of results) {
        expect(freshHrefs).toContain(result.href);
        expect(typeof result.title).toBe("string");
        expect(typeof result.section).toBe("string");
        expect(typeof result.excerpt).toBe("string");
      }
    });

    it("handles multi-term, empty and unmatched queries", () => {
      expect(() => searchModule.search("list assets")).not.toThrow();
      expect(searchModule.search("list assets").length).toBeGreaterThan(0);
      expect(searchModule.search("")).toEqual([]);
      expect(searchModule.search("   ")).toEqual([]);
      expect(searchModule.search("zzqxjv-no-such-term-qqzzx")).toEqual([]);
    });

    it("resolves every navigation page via getAllPages()", () => {
      let pages: ReturnType<typeof searchModule.getAllPages> = [];
      expect(() => {
        pages = searchModule.getAllPages();
      }).not.toThrow();

      expect(pages.length).toBeGreaterThan(0);
      const freshHrefs = new Set(freshIndex.map((doc) => doc.href));
      for (const page of pages) {
        expect(freshHrefs).toContain(page.href);
      }
    });
  });
});
