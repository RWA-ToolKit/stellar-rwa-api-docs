import type { Metadata } from "next";
import "./globals.css";
import { DocHeader } from "@/components/DocHeader";
import { SITE_URL } from "@/lib/site";

const description =
  "Documentation for the Stellar RWA platform: Soroban contracts, the indexing REST API, and the web app for tokenizing real-world assets with on-chain compliance.";

export const metadata: Metadata = {
  metadataBase: new URL(SITE_URL),
  title: {
    default: "Stellar RWA Docs",
    template: "%s · Stellar RWA Docs",
  },
  description,
  openGraph: {
    title: "Stellar RWA Docs",
    description,
    type: "website",
    url: SITE_URL,
  },
  twitter: { card: "summary", title: "Stellar RWA Docs", description },
};

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en" className="dark">
      <body className="min-h-screen">
        <a href="#main-content" className="sr-only focus:not-sr-only">Skip to main content</a>
        <DocHeader />
        {children}
      </body>
    </html>
  );
}
