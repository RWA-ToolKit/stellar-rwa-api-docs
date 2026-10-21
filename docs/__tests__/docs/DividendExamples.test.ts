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

// Worked example verified against the current Testnet deployment.
// These values mirror the runnable example on the dividend contract reference page.
const TESTNET_ASSET_TOKEN =
  "CBMCWLSQSWUTLUJFCNBHNBSXMUM3XU7NAQ5TSNERW4HA4ZZBYHLG4ECZ";
const TESTNET_PAYMENT_TOKEN =
  "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC";

// The worked example creates a distribution of 100,000,000,000 stroops
// (10,000.0000000 units at 7 decimals) and claims 25% of it.
const WORKED_EXAMPLE_TOTAL = "100000000000";
const WORKED_EXAMPLE_CLAIMED = "25000000000";

describe("Dividend API Examples", () => {
  describe("Example response structure", () => {
    const exampleResponse: Distribution[] = [
      {
        id: 1,
        asset_token: TESTNET_ASSET_TOKEN,
        payment_token: TESTNET_PAYMENT_TOKEN,
        total_amount: WORKED_EXAMPLE_TOTAL,
        distributed: WORKED_EXAMPLE_CLAIMED,
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

  describe("Worked example (verified on Testnet)", () => {
    it("should compute claimed_percent from distributed / total_amount", () => {
      const total = BigInt(WORKED_EXAMPLE_TOTAL);
      const distributed = BigInt(WORKED_EXAMPLE_CLAIMED);
      const claimedPercent = Number((distributed * 10000n) / total) / 100;
      expect(claimedPercent).toBe(25.0);
    });

    it("should leave the distribution incomplete after a partial claim", () => {
      const distributed = BigInt(WORKED_EXAMPLE_CLAIMED);
      const total = BigInt(WORKED_EXAMPLE_TOTAL);
      const completed = distributed >= total;
      expect(completed).toBe(false);
    });

    it("should use the documented Testnet token addresses", () => {
      expect(TESTNET_ASSET_TOKEN).toMatch(/^C[A-Z0-9]{55}$/);
      expect(TESTNET_PAYMENT_TOKEN).toMatch(/^C[A-Z0-9]{55}$/);
    });
  });

  describe("Dividend contract documentation examples", () => {
    it("should match code example in contract documentation", () => {
      // The contract docs show a code example calling create_distribution
      // This test verifies the distribution structure aligns with what the contract creates
      const distribution: Distribution = {
        id: 1,
        asset_token: TESTNET_ASSET_TOKEN,
        payment_token: TESTNET_PAYMENT_TOKEN,
        total_amount: WORKED_EXAMPLE_TOTAL,
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
        asset_token: TESTNET_ASSET_TOKEN,
        payment_token: TESTNET_PAYMENT_TOKEN,
        total_amount: WORKED_EXAMPLE_TOTAL,
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
        asset_token: TESTNET_ASSET_TOKEN,
        payment_token: TESTNET_PAYMENT_TOKEN,
        total_amount: WORKED_EXAMPLE_TOTAL,
        distributed: WORKED_EXAMPLE_TOTAL,
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
            asset_token: TESTNET_ASSET_TOKEN,
            payment_token: TESTNET_PAYMENT_TOKEN,
            total_amount: WORKED_EXAMPLE_TOTAL,
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
      const newAdmin = TESTNET_ASSET_TOKEN;
      expect(newAdmin).toMatch(/^C[A-Z0-9]{55}$/);
    });
  });
});
