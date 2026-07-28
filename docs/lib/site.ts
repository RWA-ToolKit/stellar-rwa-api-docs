/** Canonical site URL, overridable via env for preview/production deploys. */
const siteUrl =
  process.env.NEXT_PUBLIC_SITE_URL ?? "https://stellar-rwa-docs.vercel.app";

if (process.env.NODE_ENV === "production" && !process.env.NEXT_PUBLIC_SITE_URL) {
  console.warn(
    "⚠️  NEXT_PUBLIC_SITE_URL not set; using default. " +
    "Set NEXT_PUBLIC_SITE_URL env var for production deploys."
  );
}

export const SITE_URL = siteUrl;
