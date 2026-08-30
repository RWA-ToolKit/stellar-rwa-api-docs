import {
  ASSET_TOKEN_CONTRACT,
  COMPLIANCE_CONTRACT,
  REGISTRY_CONTRACT,
  DIVIDEND_CONTRACT,
  validateContractDocumentation,
} from "@/lib/contracts";

/**
 * Tests to ensure contract documentation stays in sync with actual contract ABIs.
 * When contract methods change, update both the contract spec and these tests.
 */

describe("Asset Token Contract Documentation", () => {
  it("should have all expected methods", () => {
    const documentedMethods = [
      { name: "initialize", params: 9 },
      { name: "transfer", params: 3 },
      { name: "mint", params: 3 },
      { name: "burn", params: 2 },
      { name: "balance", params: 1 },
      { name: "total_supply", params: 0 },
      { name: "pause", params: 1 },
      { name: "unpause", params: 1 },
      { name: "update_valuation", params: 2 },
      { name: "set_compliance", params: 2 },
      { name: "get_metadata", params: 0 },
    ];

    const result = validateContractDocumentation(ASSET_TOKEN_CONTRACT, documentedMethods);
    expect(result.valid).toBe(true);
    if (!result.valid) {
      console.error("Asset Token mismatches:", result.mismatches);
    }
  });

  it("should match contract at expected address", () => {
    expect(ASSET_TOKEN_CONTRACT.address).toBe(
      "CBMCWLSQSWUTLUJFCNBHNBSXMUM3XU7NAQ5TSNERW4HA4ZZBYHLG4ECZ",
    );
  });

  it("should have correct method signatures", () => {
    const transferMethod = ASSET_TOKEN_CONTRACT.methods.find((m) => m.name === "transfer");
    expect(transferMethod).toBeDefined();
    expect(transferMethod?.params).toHaveLength(3);
    expect(transferMethod?.auth).toBe("from");
  });
});

describe("Compliance Contract Documentation", () => {
  it("should have all expected methods", () => {
    const documentedMethods = [
      { name: "initialize", params: 1 },
      { name: "add_to_allowlist", params: 3 },
      { name: "remove_from_allowlist", params: 2 },
      { name: "is_allowed", params: 1 },
      { name: "set_jurisdiction_restrictions", params: 2 },
      { name: "is_jurisdiction_blocked", params: 1 },
    ];

    const result = validateContractDocumentation(COMPLIANCE_CONTRACT, documentedMethods);
    expect(result.valid).toBe(true);
    if (!result.valid) {
      console.error("Compliance mismatches:", result.mismatches);
    }
  });

  it("should match contract at expected address", () => {
    expect(COMPLIANCE_CONTRACT.address).toBe(
      "CBUERYDM7DXTZLLKDBRJKUBPFJ7M4OSUN4T7XKUARU345RLXNAIQD2IU",
    );
  });
});

describe("Registry Contract Documentation", () => {
  it("should have all expected methods", () => {
    const documentedMethods = [
      { name: "initialize", params: 1 },
      { name: "register_asset", params: 3 },
      { name: "get_all_assets", params: 0 },
      { name: "get_asset", params: 1 },
    ];

    const result = validateContractDocumentation(REGISTRY_CONTRACT, documentedMethods);
    expect(result.valid).toBe(true);
    if (!result.valid) {
      console.error("Registry mismatches:", result.mismatches);
    }
  });

  it("should match contract at expected address", () => {
    expect(REGISTRY_CONTRACT.address).toBe(
      "CBX5SMLTXX6JP4HA5GQIO2V6QM7WCUGL2GZ6D4U773HMRI6RXISKPUR3",
    );
  });
});

describe("Dividend Contract Documentation", () => {
  it("should have all expected methods", () => {
    const documentedMethods = [
      { name: "initialize", params: 1 },
      { name: "create_distribution", params: 4 },
      { name: "claim_distribution", params: 2 },
      { name: "get_distribution", params: 1 },
      { name: "get_distributions_for_asset", params: 1 },
    ];

    const result = validateContractDocumentation(DIVIDEND_CONTRACT, documentedMethods);
    expect(result.valid).toBe(true);
    if (!result.valid) {
      console.error("Dividend mismatches:", result.mismatches);
    }
  });

  it("should match contract at expected address", () => {
    expect(DIVIDEND_CONTRACT.address).toBe(
      "CAR4XY3CEBQWFOL27JEWFW34KXSIZA7RFKDQMEIV7ZU723RWY37I2SYX",
    );
  });
});

describe("Contract Specifications", () => {
  it("should have valid network configuration", () => {
    const contracts = [ASSET_TOKEN_CONTRACT, COMPLIANCE_CONTRACT, REGISTRY_CONTRACT, DIVIDEND_CONTRACT];
    contracts.forEach((contract) => {
      expect(contract.network).toBe("testnet");
      expect(contract.address).toBeTruthy();
      expect(contract.methods.length).toBeGreaterThan(0);
    });
  });

  it("should document critical methods", () => {
    // Asset token should have transfer gating methods
    expect(ASSET_TOKEN_CONTRACT.methods.find((m) => m.name === "transfer")).toBeDefined();
    expect(ASSET_TOKEN_CONTRACT.methods.find((m) => m.auth === "from")).toBeDefined();

    // Compliance should have allowlist check
    expect(COMPLIANCE_CONTRACT.methods.find((m) => m.name === "is_allowed")).toBeDefined();

    // Registry should have asset enumeration
    expect(REGISTRY_CONTRACT.methods.find((m) => m.name === "get_all_assets")).toBeDefined();

    // Dividend should support distributions
    expect(DIVIDEND_CONTRACT.methods.find((m) => m.name === "create_distribution")).toBeDefined();
  });
});
