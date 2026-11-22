import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

async function loadVersion() {
  vi.resetModules();
  return import("@/lib/version");
}

describe("lib/version constants", () => {
  beforeEach(() => {
    vi.unstubAllEnvs();
  });

  afterEach(() => {
    vi.unstubAllEnvs();
    vi.resetModules();
  });

  describe("happy path (env set)", () => {
    it("uses NEXT_PUBLIC_API_VERSION for DOCS_VERSION", async () => {
      vi.stubEnv("NEXT_PUBLIC_API_VERSION", "v2.3.4");
      vi.stubEnv("NEXT_PUBLIC_CONTRACT_VERSION", "v2.4.0");
      const { DOCS_VERSION } = await loadVersion();
      expect(DOCS_VERSION).toBe("v2.3.4");
    });

    it("uses NEXT_PUBLIC_CONTRACT_VERSION for CONTRACT_VERSION", async () => {
      vi.stubEnv("NEXT_PUBLIC_API_VERSION", "v2.3.4");
      vi.stubEnv("NEXT_PUBLIC_CONTRACT_VERSION", "v2.4.0");
      const { CONTRACT_VERSION } = await loadVersion();
      expect(CONTRACT_VERSION).toBe("v2.4.0");
    });
  });

  describe("fallback path (env unset)", () => {
    it("falls back to v1.0.0 for DOCS_VERSION", async () => {
      vi.stubEnv("NEXT_PUBLIC_API_VERSION", "");
      vi.stubEnv("NEXT_PUBLIC_CONTRACT_VERSION", "");
      const { DOCS_VERSION } = await loadVersion();
      expect(DOCS_VERSION).toBe("v1.0.0");
    });

    it("falls back CONTRACT_VERSION to DOCS_VERSION", async () => {
      vi.stubEnv("NEXT_PUBLIC_API_VERSION", "");
      vi.stubEnv("NEXT_PUBLIC_CONTRACT_VERSION", "");
      const { DOCS_VERSION, CONTRACT_VERSION, isVersionStale } = await loadVersion();
      expect(CONTRACT_VERSION).toBe(DOCS_VERSION);
      expect(isVersionStale(DOCS_VERSION, CONTRACT_VERSION)).toBe(false);
    });

    it("CONTRACT_VERSION follows a custom DOCS_VERSION when only the API version is set", async () => {
      vi.stubEnv("NEXT_PUBLIC_API_VERSION", "v3.1.0");
      vi.stubEnv("NEXT_PUBLIC_CONTRACT_VERSION", "");
      const { CONTRACT_VERSION } = await loadVersion();
      expect(CONTRACT_VERSION).toBe("v3.1.0");
    });
  });
});

describe("isVersionStale", () => {
  it("is stale when docs are behind on major, minor or patch", async () => {
    const { isVersionStale } = await loadVersion();
    expect(isVersionStale("v1.0.0", "v2.0.0")).toBe(true);
    expect(isVersionStale("v1.1.0", "v1.2.0")).toBe(true);
    expect(isVersionStale("v1.1.1", "v1.1.2")).toBe(true);
  });

  it("is not stale when versions are equal", async () => {
    const { isVersionStale } = await loadVersion();
    expect(isVersionStale("v1.2.3", "v1.2.3")).toBe(false);
  });

  it("is not stale when docs are ahead on major, minor or patch", async () => {
    const { isVersionStale } = await loadVersion();
    expect(isVersionStale("v2.0.0", "v1.9.9")).toBe(false);
    expect(isVersionStale("v1.3.0", "v1.2.9")).toBe(false);
    expect(isVersionStale("v1.2.4", "v1.2.3")).toBe(false);
  });

  it("higher major wins over lower minor/patch", async () => {
    const { isVersionStale } = await loadVersion();
    expect(isVersionStale("v1.9.9", "v2.0.0")).toBe(true);
    expect(isVersionStale("v2.0.0", "v1.9.9")).toBe(false);
  });

  it("handles the v prefix (any case) and surrounding whitespace", async () => {
    const { isVersionStale } = await loadVersion();
    expect(isVersionStale("1.0.0", "v1.0.1")).toBe(true);
    expect(isVersionStale("V1.0.1", "1.0.0")).toBe(false);
    expect(isVersionStale(" v1.0.0 ", "v1.0.0")).toBe(false);
  });

  it("treats missing segments as 0", async () => {
    const { isVersionStale } = await loadVersion();
    expect(isVersionStale("v1", "v1.0.0")).toBe(false);
    expect(isVersionStale("v1.0", "v1.0.1")).toBe(true);
    expect(isVersionStale("v1.2", "v1.1.9")).toBe(false);
  });

  it("treats non-numeric segments as 0", async () => {
    const { isVersionStale } = await loadVersion();
    expect(isVersionStale("abc", "v0.0.0")).toBe(false);
    expect(isVersionStale("abc", "v0.0.1")).toBe(true);
    expect(isVersionStale("v1.x.0", "v1.0.0")).toBe(false);
    expect(isVersionStale("v1.x.0", "v1.1.0")).toBe(true);
  });
});

describe("resolveDeployedVersion (build-time fallback from fetch-api-version.mjs)", () => {
  it("returns the fetched version when it is valid", async () => {
    const { resolveDeployedVersion } = await import("@/lib/apiVersion");
    expect(resolveDeployedVersion("v1.0.0", "v2.1.0")).toBe("v2.1.0");
    expect(resolveDeployedVersion("v1.0.0", "2.1.0")).toBe("2.1.0");
    expect(resolveDeployedVersion("v1.0.0", " v2.1.0-rc.1 ")).toBe("v2.1.0-rc.1");
  });

  it("falls back when the value is missing or empty", async () => {
    const { resolveDeployedVersion } = await import("@/lib/apiVersion");
    expect(resolveDeployedVersion("v1.0.0")).toBe("v1.0.0");
    expect(resolveDeployedVersion("v1.0.0", "")).toBe("v1.0.0");
    expect(resolveDeployedVersion("v1.0.0", "   ")).toBe("v1.0.0");
  });

  it("falls back when the value is not a version string", async () => {
    const { resolveDeployedVersion } = await import("@/lib/apiVersion");
    expect(resolveDeployedVersion("v1.0.0", "undefined")).toBe("v1.0.0");
    expect(resolveDeployedVersion("v1.0.0", "<html>error</html>")).toBe("v1.0.0");
    expect(resolveDeployedVersion("v1.0.0", "1.2")).toBe("v1.0.0");
  });
});
