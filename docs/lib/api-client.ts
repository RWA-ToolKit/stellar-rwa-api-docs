export type AssetSortField = "valuation" | "holders" | "created_at";
export type SortDirection = "asc" | "desc";

export type Asset = {
  id: number;
  token_contract: string;
  issuer: string;
  name: string;
  symbol: string;
  asset_type: "real_estate" | "invoice" | "commodity";
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
};

export type Holder = {
  address: string;
  balance: string;
  share_percent: number;
};

export type ComplianceSummary = {
  total_records: number;
  approved: number;
  suspended: number;
  rejected: number;
  pending: number;
  with_expiry: number;
  jurisdictions: JurisdictionCount[];
};

export type JurisdictionCount = {
  jurisdiction: string;
  count: number;
};

export type Distribution = {
  id: number;
  asset_token: string;
  payment_token: string;
  total_amount: string;
  distributed: string;
  claimed_percent: number;
  overflow_detected: boolean;
  completed: boolean;
  created_at_ledger: number;
};

export type Stats = {
  total_assets: number;
  active_assets: number;
  tvl_cents: string;
  tvl_usd: number;
  total_holders: number;
  total_distributions: number;
  last_indexed_ledger: number;
  last_updated: string;
};

export type Error = {
  error: string;
  message: string;
};

