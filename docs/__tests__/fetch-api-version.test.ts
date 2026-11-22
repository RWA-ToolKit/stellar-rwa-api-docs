import { afterEach, describe, expect, it, vi } from "vitest";
import { fetchApiVersion, main } from "../scripts/fetch-api-version.mjs";

const okJson = (body: unknown) => ({ ok: true, json: async () => body });

afterEach(() => {
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

describe("fetchApiVersion", () => {
  it("returns the release field on success and requests /version", async () => {
    const f = vi.fn().mockResolvedValue(okJson({ release: "1.2.3" }));
    expect(await fetchApiVersion("https://api.example.com", f)).toBe("1.2.3");
    expect(f.mock.calls[0][0]).toBe("https://api.example.com/version");
  });

  it("strips a trailing slash from the base URL", async () => {
    const f = vi.fn().mockResolvedValue(okJson({ release: "v0.4.0" }));
    await fetchApiVersion("https://api.example.com/", f);
    expect(f.mock.calls[0][0]).toBe("https://api.example.com/version");
  });

  it("uses global fetch by default", async () => {
    const f = vi.fn().mockResolvedValue(okJson({ release: "2.0.0" }));
    vi.stubGlobal("fetch", f);
    expect(await fetchApiVersion("https://x.test")).toBe("2.0.0");
  });

  it("returns empty string without fetching when base URL is empty or undefined", async () => {
    const f = vi.fn();
    expect(await fetchApiVersion("", f)).toBe("");
    expect(await fetchApiVersion(undefined, f)).toBe("");
    expect(f).not.toHaveBeenCalled();
  });

  it("falls back to empty string when the network is unreachable", async () => {
    const f = vi.fn().mockRejectedValue(new TypeError("fetch failed"));
    await expect(fetchApiVersion("http://127.0.0.1:1", f)).resolves.toBe("");
  });

  it("falls back when the body is not valid JSON", async () => {
    const f = vi.fn().mockResolvedValue({
      ok: true,
      json: async () => {
        throw new SyntaxError("Unexpected token");
      },
    });
    expect(await fetchApiVersion("https://x.test", f)).toBe("");
  });

  it("falls back when release is missing, non-string, or the body is null", async () => {
    for (const body of [{}, { release: 123 }, { release: null }, null]) {
      const f = vi.fn().mockResolvedValue(okJson(body));
      expect(await fetchApiVersion("https://x.test", f)).toBe("");
    }
  });

  it("falls back on a non-OK response whose body has no release", async () => {
    const f = vi.fn().mockResolvedValue({ ok: false, status: 503, json: async () => ({ error: "down" }) });
    expect(await fetchApiVersion("https://x.test", f)).toBe("");
  });
});

describe("main", () => {
  const original = process.env.API_BASE_URL;
  afterEach(() => {
    if (original === undefined) delete process.env.API_BASE_URL;
    else process.env.API_BASE_URL = original;
  });

  it("writes the release to stdout using API_BASE_URL", async () => {
    process.env.API_BASE_URL = "https://api.example.com/";
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(okJson({ release: "3.1.4" })));
    const write = vi.spyOn(process.stdout, "write").mockImplementation(() => true);
    await main();
    expect(write).toHaveBeenCalledWith("3.1.4");
  });

  it("writes nothing and does not throw when fetch rejects", async () => {
    process.env.API_BASE_URL = "https://api.example.com";
    vi.stubGlobal("fetch", vi.fn().mockRejectedValue(new Error("boom")));
    const write = vi.spyOn(process.stdout, "write").mockImplementation(() => true);
    await expect(main()).resolves.toBeUndefined();
    expect(write).toHaveBeenCalledWith("");
  });
});
