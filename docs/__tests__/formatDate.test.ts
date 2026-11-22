import { describe, expect, it } from "vitest";
import { DEFAULT_LOCALE, formatDate } from "@/lib/formatDate";

describe("formatDate", () => {
  it("defaults to en-US", () => {
    expect(DEFAULT_LOCALE).toBe("en-US");
    expect(formatDate("2026-11-21")).toBe("November 21, 2026");
  });

  it("formats with an explicit locale", () => {
    expect(formatDate("2026-11-21", "de-DE")).toBe("21. November 2026");
  });

  it("does not shift ISO date-only strings across a day boundary", () => {
    expect(formatDate("2026-01-01")).toBe("January 1, 2026");
  });
});
