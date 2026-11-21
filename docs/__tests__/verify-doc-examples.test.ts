import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, describe, expect, it } from "vitest";
import { diff, extractExamples, shape, walk } from "../../scripts/verify-doc-examples.mjs";

const fence = "```";

describe("shape", () => {
  it("reduces primitives to their JSON type names", () => {
    expect(shape("GABC")).toBe("string");
    expect(shape(42)).toBe("number");
    expect(shape(true)).toBe("boolean");
    expect(shape(null)).toBe("null");
  });

  it("keeps object keys sorted and recurses into values", () => {
    const result = shape({ zeta: 1, alpha: { nested: "x" } });
    expect(result).toEqual({ alpha: { nested: "string" }, zeta: "number" });
    expect(Object.keys(result)).toEqual(["alpha", "zeta"]);
  });

  it("uses only the first array element and keeps empty arrays empty", () => {
    expect(shape([{ id: 1 }, { other: "ignored" }])).toEqual([{ id: "number" }]);
    expect(shape([])).toEqual([]);
  });
});

describe("diff", () => {
  it("reports no drift when shapes match even though values differ", () => {
    const documented = shape({ balance: "100", holders: [{ address: "GA", since: 1 }] });
    const live = shape({ balance: "999", holders: [{ address: "GB", since: 2 }] });
    expect(diff(documented, live)).toEqual([]);
  });

  it("reports keys documented but missing from the live response", () => {
    expect(diff(shape({ id: 1, name: "a" }), shape({ id: 2 }))).toEqual([
      "$.name: documented but missing from live response",
    ]);
  });

  it("reports live keys that are not documented", () => {
    expect(diff(shape({ id: 1 }), shape({ id: 2, extra: true }))).toEqual([
      "$.extra: in live response but not documented",
    ]);
  });

  it("reports leaf type mismatches with the full path", () => {
    expect(diff(shape({ data: { amount: "10" } }), shape({ data: { amount: 10 } }))).toEqual([
      "$.data.amount: documented string, live number",
    ]);
    expect(diff(shape({ memo: null }), shape({ memo: "hi" }))).toEqual([
      "$.memo: documented null, live string",
    ]);
  });

  it("reports an object documented where the live response has a primitive", () => {
    expect(diff(shape({ data: { id: 1 } }), shape({ data: "x" }))).toEqual([
      '$.data: documented {"id":"number"}, live "string"',
    ]);
  });

  it("compares the first element of arrays and skips empty arrays", () => {
    expect(diff(shape({ items: [{ id: 1 }] }), shape({ items: [{ id: "1" }] }))).toEqual([
      "$.items[0].id: documented number, live string",
    ]);
    expect(diff(shape({ items: [] }), shape({ items: [{ id: 1 }] }))).toEqual([]);
    expect(diff(shape({ items: [{ id: 1 }] }), shape({ items: [] }))).toEqual([]);
  });
});

describe("extractExamples", () => {
  it("pairs each curl request path with the next json fence", () => {
    const mdx = [
      `${fence}bash`,
      'curl -s "https://api.example.com/v1/assets/ABC?limit=2"',
      fence,
      "",
      "Response:",
      "",
      `${fence}json`,
      '{ "id": "ABC" }',
      fence,
      "",
      `${fence}sh`,
      "curl https://api.example.com/v1/health",
      fence,
      `${fence}json`,
      '{ "ok": true }',
      fence,
    ].join("\n");

    expect(extractExamples(mdx)).toEqual([
      { path: "/v1/assets/ABC?limit=2", body: '{ "id": "ABC" }\n' },
      { path: "/v1/health", body: '{ "ok": true }\n' },
    ]);
  });

  it("ignores curl fences in other languages and pages without examples", () => {
    const mdx = [
      `${fence}ts`,
      'fetch("https://api.example.com/v1/assets")',
      fence,
      `${fence}json`,
      "{}",
      fence,
    ].join("\n");
    expect(extractExamples(mdx)).toEqual([]);
    expect(extractExamples("# No examples here")).toEqual([]);
  });

  it("returns non-JSON bodies verbatim so the caller can skip them", () => {
    const mdx = [`${fence}bash`, "curl https://api.example.com/v1/x", fence, `${fence}json`, "{ ... }", fence].join(
      "\n",
    );
    const [example] = extractExamples(mdx);
    expect(example.path).toBe("/v1/x");
    expect(() => JSON.parse(example.body)).toThrow();
  });
});

describe("walk", () => {
  let dir: string | undefined;
  afterEach(() => {
    if (dir) rmSync(dir, { recursive: true, force: true });
    dir = undefined;
  });

  it("collects .mdx files recursively and ignores other files", () => {
    dir = mkdtempSync(join(tmpdir(), "verify-doc-examples-"));
    mkdirSync(join(dir, "nested", "deeper"), { recursive: true });
    writeFileSync(join(dir, "page.mdx"), "");
    writeFileSync(join(dir, "notes.md"), "");
    writeFileSync(join(dir, "nested", "deeper", "child.mdx"), "");
    writeFileSync(join(dir, "nested", "layout.tsx"), "");

    expect(walk(dir).sort()).toEqual(
      [join(dir, "nested", "deeper", "child.mdx"), join(dir, "page.mdx")].sort(),
    );
  });
});
