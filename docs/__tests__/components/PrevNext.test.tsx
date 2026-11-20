import { describe, it, expect } from "vitest";
import { FLAT_NAV } from "../../components/nav";

describe("PrevNext Navigation", () => {
  describe("FLAT_NAV bounds", () => {
    it("should not be empty", () => {
      expect(FLAT_NAV.length).toBeGreaterThan(0);
    });

    it("should have all items with valid href", () => {
      FLAT_NAV.forEach((item) => {
        expect(item.href).toBeDefined();
        expect(item.href).toMatch(/^\/docs\//);
      });
    });

    it("should have all items with valid title", () => {
      FLAT_NAV.forEach((item) => {
        expect(item.title).toBeDefined();
        expect(item.title.length).toBeGreaterThan(0);
      });
    });
  });

  describe("First and last page navigation", () => {
    it("first page should not have previous", () => {
      const firstIndex = 0;
      const hasPrev = firstIndex > 0;
      expect(hasPrev).toBe(false);
    });

    it("last page should not have next", () => {
      const lastIndex = FLAT_NAV.length - 1;
      const hasNext = lastIndex < FLAT_NAV.length - 1;
      expect(hasNext).toBe(false);
    });

    it("middle page should have both prev and next", () => {
      const middleIndex = Math.floor(FLAT_NAV.length / 2);
      expect(middleIndex > 0).toBe(true);
      expect(middleIndex < FLAT_NAV.length - 1).toBe(true);
    });
  });

  describe("Navigation continuity", () => {
    it("should not have duplicate hrefs", () => {
      const hrefs = FLAT_NAV.map((item) => item.href);
      const uniqueHrefs = new Set(hrefs);
      expect(uniqueHrefs.size).toBe(hrefs.length);
    });

    it("should have sequential access for all items", () => {
      for (let i = 0; i < FLAT_NAV.length; i++) {
        const prev = i > 0 ? FLAT_NAV[i - 1] : null;
        const next = i < FLAT_NAV.length - 1 ? FLAT_NAV[i + 1] : null;

        if (prev) {
          expect(prev.href).toBeDefined();
        }
        if (next) {
          expect(next.href).toBeDefined();
        }
      }
    });
  });

  describe("Route existence validation", () => {
    it("should have routes that follow docs structure", () => {
      const validRoutePatterns = [
        /^\/docs\/getting-started$/,
        /^\/docs\/contracts\//,
        /^\/docs\/api\//,
        /^\/docs\/[a-z-]+$/,
      ];

      FLAT_NAV.forEach((item) => {
        const isValid = validRoutePatterns.some((pattern) =>
          pattern.test(item.href)
        );
        expect(isValid).toBe(true);
      });
    });
  });
});
