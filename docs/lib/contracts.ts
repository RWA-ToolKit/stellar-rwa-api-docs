/**
 * Contract method signatures and metadata for validation.
 * Used to check that documentation stays in sync with actual contract ABIs.
 */

export interface ContractMethod {
  name: string;
  returns: string;
  params: Array<{ name: string; type: string }>;
  auth?: string;
  errors?: string[];
}

export interface ContractSpec {
  name: string;
  address: string;
  network: string;
  methods: ContractMethod[];
}

export const ASSET_TOKEN_CONTRACT: ContractSpec = {
  name: "Asset Token",
  address: "CBMCWLSQSWUTLUJFCNBHNBSXMUM3XU7NAQ5TSNERW4HA4ZZBYHLG4ECZ",
  network: "testnet",
  methods: [
    {
      name: "initialize",
      returns: "void",
      params: [
        { name: "admin", type: "Address" },
        { name: "name", type: "String" },
        { name: "symbol", type: "String" },
        { name: "asset_type", type: "String" },
        { name: "total_supply", type: "i128" },
        { name: "decimals", type: "u32" },
        { name: "compliance_contract", type: "Address" },
        { name: "asset_description", type: "String" },
        { name: "valuation", type: "i128" },
      ],
      auth: "admin",
      errors: ["AlreadyInitialized (1)", "InvalidAmount (5)", "RecipientNotCompliant (8)"],
    },
    {
      name: "transfer",
      returns: "void",
      params: [
        { name: "from", type: "Address" },
        { name: "to", type: "Address" },
        { name: "amount", type: "i128" },
      ],
      auth: "from",
      errors: [
        "InvalidAmount (5)",
        "Paused (6)",
        "SenderNotCompliant (7)",
        "RecipientNotCompliant (8)",
        "InsufficientBalance (4)",
      ],
    },
    {
      name: "mint",
      returns: "void",
      params: [
        { name: "admin", type: "Address" },
        { name: "to", type: "Address" },
        { name: "amount", type: "i128" },
      ],
      auth: "admin",
      errors: [
        "Unauthorized (3)",
        "InvalidAmount (5)",
        "Paused (6)",
        "RecipientNotCompliant (8)",
        "Overflow (9)",
      ],
    },
    {
      name: "burn",
      returns: "void",
      params: [
        { name: "from", type: "Address" },
        { name: "amount", type: "i128" },
      ],
      auth: "from",
      errors: ["InvalidAmount (5)", "InsufficientBalance (4)"],
    },
    {
      name: "balance",
      returns: "i128",
      params: [{ name: "id", type: "Address" }],
    },
    {
      name: "total_supply",
      returns: "i128",
      params: [],
    },
    {
      name: "pause",
      returns: "void",
      params: [{ name: "admin", type: "Address" }],
      auth: "admin",
      errors: ["Unauthorized (3)"],
    },
    {
      name: "unpause",
      returns: "void",
      params: [{ name: "admin", type: "Address" }],
      auth: "admin",
      errors: ["Unauthorized (3)"],
    },
    {
      name: "update_valuation",
      returns: "void",
      params: [
        { name: "admin", type: "Address" },
        { name: "new_valuation", type: "i128" },
      ],
      auth: "admin",
      errors: ["Unauthorized (3)", "InvalidAmount (5)"],
    },
    {
      name: "set_compliance",
      returns: "void",
      params: [
        { name: "admin", type: "Address" },
        { name: "compliance", type: "Address" },
      ],
      auth: "admin",
    },
    {
      name: "get_metadata",
      returns: "AssetMetadata",
      params: [],
    },
  ],
};

export const COMPLIANCE_CONTRACT: ContractSpec = {
  name: "Compliance",
  address: "CBUERYDM7DXTZLLKDBRJKUBPFJ7M4OSUN4T7XKUARU345RLXNAIQD2IU",
  network: "testnet",
  methods: [
    {
      name: "initialize",
      returns: "void",
      params: [{ name: "admin", type: "Address" }],
      auth: "admin",
    },
    {
      name: "add_to_allowlist",
      returns: "void",
      params: [
        { name: "admin", type: "Address" },
        { name: "account", type: "Address" },
        { name: "kyc_data", type: "KYCData" },
      ],
      auth: "admin",
    },
    {
      name: "remove_from_allowlist",
      returns: "void",
      params: [
        { name: "admin", type: "Address" },
        { name: "account", type: "Address" },
      ],
      auth: "admin",
    },
    {
      name: "is_allowed",
      returns: "bool",
      params: [{ name: "account", type: "Address" }],
    },
    {
      name: "set_jurisdiction_restrictions",
      returns: "void",
      params: [
        { name: "admin", type: "Address" },
        { name: "blocked_jurisdictions", type: "Vec<String>" },
      ],
      auth: "admin",
    },
    {
      name: "is_jurisdiction_blocked",
      returns: "bool",
      params: [{ name: "jurisdiction", type: "String" }],
    },
  ],
};

export const REGISTRY_CONTRACT: ContractSpec = {
  name: "Registry",
  address: "CBX5SMLTXX6JP4HA5GQIO2V6QM7WCUGL2GZ6D4U773HMRI6RXISKPUR3",
  network: "testnet",
  methods: [
    {
      name: "initialize",
      returns: "void",
      params: [{ name: "admin", type: "Address" }],
      auth: "admin",
    },
    {
      name: "register_asset",
      returns: "i128",
      params: [
        { name: "admin", type: "Address" },
        { name: "token_contract", type: "Address" },
        { name: "compliance_contract", type: "Address" },
      ],
      auth: "admin",
    },
    {
      name: "get_all_assets",
      returns: "Vec<i128>",
      params: [],
    },
    {
      name: "get_asset",
      returns: "AssetDetails",
      params: [{ name: "id", type: "i128" }],
    },
  ],
};

export const DIVIDEND_CONTRACT: ContractSpec = {
  name: "Dividend",
  address: "CAR4XY3CEBQWFOL27JEWFW34KXSIZA7RFKDQMEIV7ZU723RWY37I2SYX",
  network: "testnet",
  methods: [
    {
      name: "initialize",
      returns: "void",
      params: [{ name: "admin", type: "Address" }],
      auth: "admin",
    },
    {
      name: "create_distribution",
      returns: "i128",
      params: [
        { name: "admin", type: "Address" },
        { name: "asset_token", type: "Address" },
        { name: "payment_token", type: "Address" },
        { name: "amount", type: "i128" },
      ],
      auth: "admin",
    },
    {
      name: "claim_distribution",
      returns: "i128",
      params: [
        { name: "holder", type: "Address" },
        { name: "distribution_id", type: "i128" },
      ],
      auth: "holder",
    },
    {
      name: "get_distribution",
      returns: "DistributionDetails",
      params: [{ name: "id", type: "i128" }],
    },
    {
      name: "get_distributions_for_asset",
      returns: "Vec<i128>",
      params: [{ name: "asset_token", type: "Address" }],
    },
  ],
};

export const ALL_CONTRACTS = [
  ASSET_TOKEN_CONTRACT,
  COMPLIANCE_CONTRACT,
  REGISTRY_CONTRACT,
  DIVIDEND_CONTRACT,
];

/**
 * Validate that documented methods match the contract spec.
 * This function can be used in tests to ensure documentation doesn't drift.
 */
export function validateContractDocumentation(
  contract: ContractSpec,
  documentedMethods: Array<{ name: string; params: number }>,
): { valid: boolean; mismatches: string[] } {
  const mismatches: string[] = [];

  for (const doc of documentedMethods) {
    const spec = contract.methods.find((m) => m.name === doc.name);
    if (!spec) {
      mismatches.push(`Documented method "${doc.name}" not found in contract spec`);
      continue;
    }
    if (spec.params.length !== doc.params) {
      mismatches.push(
        `Method "${doc.name}": documented ${doc.params} params, but spec has ${spec.params.length}`,
      );
    }
  }

  // Check for methods in spec but not documented
  const documentedNames = new Set(documentedMethods.map((m) => m.name));
  for (const specMethod of contract.methods) {
    if (!documentedNames.has(specMethod.name)) {
      mismatches.push(`Contract method "${specMethod.name}" is not documented`);
    }
  }

  return {
    valid: mismatches.length === 0,
    mismatches,
  };
}
