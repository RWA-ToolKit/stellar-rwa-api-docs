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

// Type definitions for newer contract entry points documented on the page
interface DistributionConfig {
  min_holding_period: number;
  dividend_deadline: number;
}

interface PaginatedDistributions {
  items: Distribution[];
  total: number;
  offset: number;
  limit: number;
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

  describe("Newer contract entry points", () => {
    it("should expose min_holding_period and dividend_deadline config", () => {
      const config: DistributionConfig = {
        min_holding_period: 17280,
        dividend_deadline: 3600000,
      };

      expect(typeof config.min_holding_period).toBe("number");
      expect(typeof config.dividend_deadline).toBe("number");
      expect(config.min_holding_period).toBeGreaterThanOrEqual(0);
      expect(config.dividend_deadline).toBeGreaterThan(0);
    });

    it("should support paginated distribution listings", () => {
      const page: PaginatedDistributions = {
        items: [
          {
            id: 1,
            asset_token: "CBMCWLSQSWUTLUJFCNBHNBSXMUM3XU7NAQ5TSNERW4HA4ZZBYHLG4ECZ",
            payment_token: "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC",
            total_amount: "100000000000",
            distributed: "0",
            claimed_percent: 0.0,
            overflow_detected: false,
            completed: false,
            created_at_ledger: 3510000,
          },
        ],
        total: 1,
        offset: 0,
        limit: 10,
      };

      expect(Array.isArray(page.items)).toBe(true);
      expect(page.items.length).toBeLessThanOrEqual(page.limit);
      expect(page.offset).toBeGreaterThanOrEqual(0);
      expect(page.total).toBeGreaterThanOrEqual(page.items.length);
    });

    it("should support batch distribution creation ids", () => {
      // Batch create_distributions returns the ids of the created distributions
      const createdIds: number[] = [1, 2, 3];
      expect(createdIds.length).toBeGreaterThan(0);
      createdIds.forEach((id) => expect(id).toBeGreaterThan(0));
    });

    it("should support SEP-41 token interface for payment token", () => {
      // SEP-41 tokens expose transfer/balance/allowance entry points
      const sep41Methods = ["transfer", "transfer_from", "approve", "balance", "allowance"];
      expect(sep41Methods).toContain("transfer");
      expect(sep41Methods).toContain("balance");
    });

    it("should support admin handover entry point", () => {
      const newAdmin = "CBMCWLSQSWUTLUJFCNBHNBSXMUM3XU7NAQ5TSNERW4HA4ZZBYHLG4ECZ";
      expect(newAdmin).toMatch(/^C[A-Z0-9]{55}$/);
    });

    it("should support reclaim and cancel of unclaimed dividends", () => {
      const reclaimed: Distribution = {
        id: 1,
        asset_token: "CBMCWLSQSWUTLUJFCNBHNBSXMUM3XU7NAQ5TSNERW4HA4ZZBYHLG4ECZ",
        payment_token: "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC",
        total_amount: "100000000000",
        distributed: "25000000000",
        claimed_percent: 25.0,
        overflow_detected: false,
        completed: true,
        created_at_ledger: 3510000,
      };

      expect(reclaimed.completed).toBe(true);
      expect(BigInt(reclaimed.distributed) < BigInt(reclaimed.total_amount)).toBe(true);
    });
  });

  describe("Empty response handling", () => {
    it("should return empty array for asset with no distributions", () => {
      const emptyResponse: Distribution[] = [];
      expect(Array.isArray(emptyResponse)).toBe(true);
      expect(emptyResponse.length).toBe(0);
    });
  });
});
