/** Canonical site URL, overridable via env for preview/production deploys. */
export const SITE_URL =
  process.env.NEXT_PUBLIC_SITE_URL ?? "https://stellar-rwa-docs.vercel.app";

/** Stellar RWA API base URL, overridable via env. Defaults to local development. */
export const API_BASE_URL =
  process.env.NEXT_PUBLIC_API_BASE_URL ?? "http://localhost:8080";
