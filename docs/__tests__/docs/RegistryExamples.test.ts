import { describe, it, expect } from "vitest";

// Type definitions matching the actual API models
interface Asset {
  id: number;
  token_contract: string;
  issuer: string;
  name: string;
  symbol: string;
  asset_type: string;
  description: string;
  valuation_cents: string;
  valuation_usd: number;
  decimals: number;
  total_supply: string;
  holders: number;
  active: boolean;
  paused: boolean;
  compliance_contract: string;
  created_at_ledger: number;
}

describe("Registry/Assets API Examples", () => {
  describe("Asset response structure", () => {
    const exampleAsset: Asset = {
      id: 1,
      token_contract: "CBMCWLSQSWUTLUJFCNBHNBSXMUM3XU7NAQ5TSNERW4HA4ZZBYHLG4ECZ",
      issuer: "GAIQGTOBTTLLDJ4SWGGESM7UWJ2DI4K3ZNHUSHPDKJL2IE5FKY3BSRAA",
      name: "Manhattan Loft",
      symbol: "MLOFT",
      asset_type: "real_estate",
      description: "A tokenized loft in Manhattan",
      valuation_cents: "500000000",
      valuation_usd: 5000000.0,
      decimals: 2,
      total_supply: "1000000",
      holders: 1,
      active: true,
      paused: false,
      compliance_contract: "CBUERYDM7DXTZLLKDBRJKUBPFJ7M4OSUN4T7XKUARU345RLXNAIQD2IU",
      created_at_ledger: 3502885,
    };

    it("should have all required fields", () => {
      expect(exampleAsset).toHaveProperty("id");
      expect(exampleAsset).toHaveProperty("token_contract");
      expect(exampleAsset).toHaveProperty("issuer");
      expect(exampleAsset).toHaveProperty("name");
      expect(exampleAsset).toHaveProperty("symbol");
      expect(exampleAsset).toHaveProperty("asset_type");
      expect(exampleAsset).toHaveProperty("description");
      expect(exampleAsset).toHaveProperty("valuation_cents");
      expect(exampleAsset).toHaveProperty("valuation_usd");
      expect(exampleAsset).toHaveProperty("decimals");
      expect(exampleAsset).toHaveProperty("total_supply");
      expect(exampleAsset).toHaveProperty("holders");
      expect(exampleAsset).toHaveProperty("active");
      expect(exampleAsset).toHaveProperty("paused");
      expect(exampleAsset).toHaveProperty("compliance_contract");
      expect(exampleAsset).toHaveProperty("created_at_ledger");
    });

    it("should have correct field types", () => {
      expect(typeof exampleAsset.id).toBe("number");
      expect(typeof exampleAsset.token_contract).toBe("string");
      expect(typeof exampleAsset.issuer).toBe("string");
      expect(typeof exampleAsset.name).toBe("string");
      expect(typeof exampleAsset.symbol).toBe("string");
      expect(typeof exampleAsset.asset_type).toBe("string");
      expect(typeof exampleAsset.description).toBe("string");
      expect(typeof exampleAsset.valuation_cents).toBe("string");
      expect(typeof exampleAsset.valuation_usd).toBe("number");
      expect(typeof exampleAsset.decimals).toBe("number");
      expect(typeof exampleAsset.total_supply).toBe("string");
      expect(typeof exampleAsset.holders).toBe("number");
      expect(typeof exampleAsset.active).toBe("boolean");
      expect(typeof exampleAsset.paused).toBe("boolean");
      expect(typeof exampleAsset.compliance_contract).toBe("string");
      expect(typeof exampleAsset.created_at_ledger).toBe("number");
    });

    it("should have valid i128 string representations for monetary fields", () => {
      expect(exampleAsset.valuation_cents).toMatch(/^\d+$/);
      expect(exampleAsset.total_supply).toMatch(/^\d+$/);
      expect(BigInt(exampleAsset.valuation_cents) >= 0n).toBe(true);
      expect(BigInt(exampleAsset.total_supply) > 0n).toBe(true);
    });

    it("should have valuation_usd matching valuation_cents", () => {
      const expectedUsd =
        Number(BigInt(exampleAsset.valuation_cents)) / 100;
      expect(exampleAsset.valuation_usd).toBeCloseTo(expectedUsd, 2);
    });

    it("should have valid asset_type", () => {
      const validTypes = ["real_estate", "invoice", "commodity"];
      expect(validTypes).toContain(exampleAsset.asset_type);
    });

    it("should have valid Stellar contract addresses", () => {
      expect(exampleAsset.token_contract).toMatch(/^C[A-Z0-9]{55}$/);
      expect(exampleAsset.compliance_contract).toMatch(/^C[A-Z0-9]{55}$/);
      expect(exampleAsset.issuer).toMatch(/^G[A-Z0-9]{55}$/);
    });

    it("should have non-negative decimals", () => {
      expect(exampleAsset.decimals).toBeGreaterThanOrEqual(0);
      expect(exampleAsset.decimals).toBeLessThanOrEqual(18);
    });

    it("should have non-negative holders count", () => {
      expect(exampleAsset.holders).toBeGreaterThanOrEqual(0);
    });

    it("should have valid ledger number", () => {
      expect(exampleAsset.created_at_ledger).toBeGreaterThan(0);
    });
  });

  describe("Asset type variants", () => {
    it("should handle real_estate asset", () => {
      const realEstateAsset: Asset = {
        id: 1,
        token_contract: "CBMCWLSQSWUTLUJFCNBHNBSXMUM3XU7NAQ5TSNERW4HA4ZZBYHLG4ECZ",
        issuer: "GAIQGTOBTTLLDJ4SWGGESM7UWJ2DI4K3ZNHUSHPDKJL2IE5FKY3BSRAA",
        name: "Manhattan Loft",
        symbol: "MLOFT",
        asset_type: "real_estate",
        description: "A tokenized loft in Manhattan",
        valuation_cents: "500000000",
        valuation_usd: 5000000.0,
        decimals: 2,
        total_supply: "1000000",
        holders: 1,
        active: true,
        paused: false,
        compliance_contract: "CBUERYDM7DXTZLLKDBRJKUBPFJ7M4OSUN4T7XKUARU345RLXNAIQD2IU",
        created_at_ledger: 3502885,
      };

      expect(realEstateAsset.asset_type).toBe("real_estate");
    });

    it("should handle invoice asset", () => {
      const invoiceAsset: Asset = {
        id: 2,
        token_contract: "CBMCWLSQSWUTLUJFCNBHNBSXMUM3XU7NAQ5TSNERW4HA4ZZBYHLG4ECZ",
        issuer: "GAIQGTOBTTLLDJ4SWGGESM7UWJ2DI4K3ZNHUSHPDKJL2IE5FKY3BSRAA",
        name: "Invoice Pool",
        symbol: "INV",
        asset_type: "invoice",
        description: "Tokenized invoices",
        valuation_cents: "1000000000",
        valuation_usd: 10000000.0,
        decimals: 2,
        total_supply: "10000000",
        holders: 50,
        active: true,
        paused: false,
        compliance_contract: "CBUERYDM7DXTZLLKDBRJKUBPFJ7M4OSUN4T7XKUARU345RLXNAIQD2IU",
        created_at_ledger: 3502886,
      };

      expect(invoiceAsset.asset_type).toBe("invoice");
    });

    it("should handle commodity asset", () => {
      const commodityAsset: Asset = {
        id: 3,
        token_contract: "CBMCWLSQSWUTLUJFCNBHNBSXMUM3XU7NAQ5TSNERW4HA4ZZBYHLG4ECZ",
        issuer: "GAIQGTOBTTLLDJ4SWGGESM7UWJ2DI4K3ZNHUSHPDKJL2IE5FKY3BSRAA",
        name: "Gold Bars",
        symbol: "GLD",
        asset_type: "commodity",
        description: "Tokenized gold",
        valuation_cents: "2000000000",
        valuation_usd: 20000000.0,
        decimals: 6,
        total_supply: "1000000000000",
        holders: 100,
        active: true,
        paused: false,
        compliance_contract: "CBUERYDM7DXTZLLKDBRJKUBPFJ7M4OSUN4T7XKUARU345RLXNAIQD2IU",
        created_at_ledger: 3502887,
      };

      expect(commodityAsset.asset_type).toBe("commodity");
    });
  });

  describe("Asset state variations", () => {
    it("should handle active and unpaused asset", () => {
      const asset: Asset = {
        id: 1,
        token_contract: "CBMCWLSQSWUTLUJFCNBHNBSXMUM3XU7NAQ5TSNERW4HA4ZZBYHLG4ECZ",
        issuer: "GAIQGTOBTTLLDJ4SWGGESM7UWJ2DI4K3ZNHUSHPDKJL2IE5FKY3BSRAA",
        name: "Manhattan Loft",
        symbol: "MLOFT",
        asset_type: "real_estate",
        description: "A tokenized loft in Manhattan",
        valuation_cents: "500000000",
        valuation_usd: 5000000.0,
        decimals: 2,
        total_supply: "1000000",
        holders: 1,
        active: true,
        paused: false,
        compliance_contract: "CBUERYDM7DXTZLLKDBRJKUBPFJ7M4OSUN4T7XKUARU345RLXNAIQD2IU",
        created_at_ledger: 3502885,
      };

      expect(asset.active).toBe(true);
      expect(asset.paused).toBe(false);
    });

    it("should handle inactive asset", () => {
      const asset: Asset = {
        id: 1,
        token_contract: "CBMCWLSQSWUTLUJFCNBHNBSXMUM3XU7NAQ5TSNERW4HA4ZZBYHLG4ECZ",
        issuer: "GAIQGTOBTTLLDJ4SWGGESM7UWJ2DI4K3ZNHUSHPDKJL2IE5FKY3BSRAA",
        name: "Manhattan Loft",
        symbol: "MLOFT",
        asset_type: "real_estate",
        description: "A tokenized loft in Manhattan",
        valuation_cents: "500000000",
        valuation_usd: 5000000.0,
        decimals: 2,
        total_supply: "1000000",
        holders: 1,
        active: false,
        paused: false,
        compliance_contract: "CBUERYDM7DXTZLLKDBRJKUBPFJ7M4OSUN4T7XKUARU345RLXNAIQD2IU",
        created_at_ledger: 3502885,
      };

      expect(asset.active).toBe(false);
    });

    it("should handle paused asset", () => {
      const asset: Asset = {
        id: 1,
        token_contract: "CBMCWLSQSWUTLUJFCNBHNBSXMUM3XU7NAQ5TSNERW4HA4ZZBYHLG4ECZ",
        issuer: "GAIQGTOBTTLLDJ4SWGGESM7UWJ2DI4K3ZNHUSHPDKJL2IE5FKY3BSRAA",
        name: "Manhattan Loft",
        symbol: "MLOFT",
        asset_type: "real_estate",
        description: "A tokenized loft in Manhattan",
        valuation_cents: "500000000",
        valuation_usd: 5000000.0,
        decimals: 2,
        total_supply: "1000000",
        holders: 1,
        active: true,
        paused: true,
        compliance_contract: "CBUERYDM7DXTZLLKDBRJKUBPFJ7M4OSUN4T7XKUARU345RLXNAIQD2IU",
        created_at_ledger: 3502885,
      };

      expect(asset.paused).toBe(true);
    });
  });

  describe("List response validation", () => {
    it("should support array of assets", () => {
      const assets: Asset[] = [
        {
          id: 1,
          token_contract: "CBMCWLSQSWUTLUJFCNBHNBSXMUM3XU7NAQ5TSNERW4HA4ZZBYHLG4ECZ",
          issuer: "GAIQGTOBTTLLDJ4SWGGESM7UWJ2DI4K3ZNHUSHPDKJL2IE5FKY3BSRAA",
          name: "Manhattan Loft",
          symbol: "MLOFT",
          asset_type: "real_estate",
          description: "A tokenized loft in Manhattan",
          valuation_cents: "500000000",
          valuation_usd: 5000000.0,
          decimals: 2,
          total_supply: "1000000",
          holders: 1,
          active: true,
          paused: false,
          compliance_contract:
            "CBUERYDM7DXTZLLKDBRJKUBPFJ7M4OSUN4T7XKUARU345RLXNAIQD2IU",
          created_at_ledger: 3502885,
        },
      ];

      expect(Array.isArray(assets)).toBe(true);
      expect(assets.length).toBeGreaterThan(0);
    });

    it("should support empty asset list", () => {
      const assets: Asset[] = [];
      expect(Array.isArray(assets)).toBe(true);
      expect(assets.length).toBe(0);
    });
  });
});
