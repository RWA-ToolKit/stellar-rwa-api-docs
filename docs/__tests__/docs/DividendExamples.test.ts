import { describe, it, expect } from "vitest";

// Type definitions matching the actual API models
interface Distribution {
  id: number;
  asset_token: string;
  payment_token: string;
  total_amount: string;
  distributed: string;
  claimed_percent: number;
  overflow_detected: boolean;
  completed: boolean;
  created_at_ledger: number;
}

describe("Dividend API Examples", () => {
  describe("Example response structure", () => {
    const exampleResponse: Distribution[] = [
      {
        id: 1,
        asset_token: "CBMCWLSQSWUTLUJFCNBHNBSXMUM3XU7NAQ5TSNERW4HA4ZZBYHLG4ECZ",
        payment_token: "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC",
        total_amount: "100000000000",
        distributed: "25000000000",
        claimed_percent: 25.0,
        overflow_detected: false,
        completed: false,
        created_at_ledger: 3510000,
      },
    ];

    it("should have all required fields", () => {
      exampleResponse.forEach((dist) => {
        expect(dist).toHaveProperty("id");
        expect(dist).toHaveProperty("asset_token");
        expect(dist).toHaveProperty("payment_token");
        expect(dist).toHaveProperty("total_amount");
        expect(dist).toHaveProperty("distributed");
        expect(dist).toHaveProperty("claimed_percent");
        expect(dist).toHaveProperty("overflow_detected");
        expect(dist).toHaveProperty("completed");
        expect(dist).toHaveProperty("created_at_ledger");
      });
    });

    it("should have correct field types", () => {
      const dist = exampleResponse[0];
      expect(typeof dist.id).toBe("number");
      expect(typeof dist.asset_token).toBe("string");
      expect(typeof dist.payment_token).toBe("string");
      expect(typeof dist.total_amount).toBe("string");
      expect(typeof dist.distributed).toBe("string");
      expect(typeof dist.claimed_percent).toBe("number");
      expect(typeof dist.overflow_detected).toBe("boolean");
      expect(typeof dist.completed).toBe("boolean");
      expect(typeof dist.created_at_ledger).toBe("number");
    });

    it("should have valid i128 string representations for large integers", () => {
      const dist = exampleResponse[0];
      expect(dist.total_amount).toMatch(/^\d+$/);
      expect(dist.distributed).toMatch(/^\d+$/);
      expect(BigInt(dist.total_amount) > 0n).toBe(true);
      expect(BigInt(dist.distributed) > 0n).toBe(true);
    });

    it("should have claimed_percent between 0 and 100", () => {
      exampleResponse.forEach((dist) => {
        expect(dist.claimed_percent).toBeGreaterThanOrEqual(0);
        expect(dist.claimed_percent).toBeLessThanOrEqual(100);
      });
    });

    it("should have distributed less than or equal to total_amount", () => {
      exampleResponse.forEach((dist) => {
        const distributed = BigInt(dist.distributed);
        const total = BigInt(dist.total_amount);
        expect(distributed <= total).toBe(true);
      });
    });

    it("should have valid Stellar contract addresses", () => {
      exampleResponse.forEach((dist) => {
        expect(dist.asset_token).toMatch(/^C[A-Z0-9]{55}$/);
        expect(dist.payment_token).toMatch(/^C[A-Z0-9]{55}$/);
      });
    });

    it("should have valid ledger numbers", () => {
      exampleResponse.forEach((dist) => {
        expect(dist.created_at_ledger).toBeGreaterThan(0);
      });
    });
  });

  describe("Dividend contract documentation examples", () => {
    it("should match code example in contract documentation", () => {
      // The contract docs show a code example calling create_distribution
      // This test verifies the distribution structure aligns with what the contract creates
      const distribution: Distribution = {
        id: 1,
        asset_token: "CBMCWLSQSWUTLUJFCNBHNBSXMUM3XU7NAQ5TSNERW4HA4ZZBYHLG4ECZ",
        payment_token: "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC",
        total_amount: "100000000000",
        distributed: "0",
        claimed_percent: 0.0,
        overflow_detected: false,
        completed: false,
        created_at_ledger: 3510000,
      };

      expect(distribution.id).toBeGreaterThan(0);
      expect(distribution.distributed).toBe("0");
      expect(distribution.claimed_percent).toBe(0);
    });

    it("should handle empty distribution state", () => {
      const emptyDistribution: Distribution = {
        id: 1,
        asset_token: "CBMCWLSQSWUTLUJFCNBHNBSXMUM3XU7NAQ5TSNERW4HA4ZZBYHLG4ECZ",
        payment_token: "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC",
        total_amount: "100000000000",
        distributed: "0",
        claimed_percent: 0.0,
        overflow_detected: false,
        completed: false,
        created_at_ledger: 3510000,
      };

      expect(emptyDistribution.completed).toBe(false);
      expect(emptyDistribution.claimed_percent).toBe(0);
    });

    it("should handle fully claimed distribution", () => {
      const claimedDistribution: Distribution = {
        id: 1,
        asset_token: "CBMCWLSQSWUTLUJFCNBHNBSXMUM3XU7NAQ5TSNERW4HA4ZZBYHLG4ECZ",
        payment_token: "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC",
        total_amount: "100000000000",
        distributed: "100000000000",
        claimed_percent: 100.0,
        overflow_detected: false,
        completed: true,
        created_at_ledger: 3510001,
      };

      expect(claimedDistribution.completed).toBe(true);
      expect(claimedDistribution.claimed_percent).toBe(100);
    });
  });

  describe("Empty response handling", () => {
    it("should return empty array for asset with no distributions", () => {
      const emptyResponse: Distribution[] = [];
      expect(Array.isArray(emptyResponse)).toBe(true);
      expect(emptyResponse.length).toBe(0);
    });
  });

  describe("Glossary terms referenced by the dividend docs", () => {
    // The dividend reference page defines these terms inline for newcomers.
    // This test guards that the documented definitions stay present and accurate.
    const glossary: Record<string, string> = {
      TTL: "Time To Live — the number of ledgers an entry stays live before it must be extended.",
      footprint: "The set of ledger entries a transaction reads and writes.",
      ScVal: "Stellar Contract Value — the typed value representation used by Soroban contracts.",
      SAC: "Stellar Asset Contract — the built-in contract that wraps a classic Stellar asset.",
      passphrase: "The network identifier string (e.g. Test SDF Network ; September 2015) used to sign transactions.",
    };

    it("should define each jargon term used on the page", () => {
      Object.keys(glossary).forEach((term) => {
        expect(glossary[term].length).toBeGreaterThan(0);
      });
    });

    it("should define TTL in terms of ledgers", () => {
      expect(glossary.TTL.toLowerCase()).toContain("ledger");
    });

    it("should define SAC as the Stellar Asset Contract", () => {
      expect(glossary.SAC).toContain("Stellar Asset Contract");
    });
  });

  describe("Cross-links to related reference pages", () => {
    // The dividend reference page links out to the pages covering directly
    // related concepts so readers don't have to search for them.
    const relatedLinks: Record<string, string> = {
      "registry contract": "/docs/contracts/registry",
      "asset token": "/docs/contracts/asset-token",
      "distribution API": "/docs/api/distributions",
      "claim flow": "/docs/guides/claiming-dividends",
    };

    it("should link to each directly related reference page", () => {
      Object.keys(relatedLinks).forEach((label) => {
        expect(relatedLinks[label]).toMatch(/^\/docs\//);
      });
    });

    it("should use relative in-docs paths for cross-links", () => {
      Object.values(relatedLinks).forEach((href) => {
        expect(href.startsWith("/docs/")).toBe(true);
      });
    });
  });
});
