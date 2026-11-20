import type { ReactNode } from "react";

const SITE_URL = "https://docs.handsoff.dev";
const API_DOCS_URL = `${SITE_URL}/docs/api`;

const structuredData = {
  "@context": "https://schema.org",
  "@type": "TechArticle",
  headline: "HandsOff API Reference",
  description:
    "Reference documentation for the HandsOff API, including endpoints, request and response schemas, and authentication details.",
  url: API_DOCS_URL,
  mainEntityOfPage: {
    "@type": "WebPage",
    "@id": API_DOCS_URL,
  },
  inLanguage: "en",
  isPartOf: {
    "@type": "WebSite",
    name: "HandsOff Docs",
    url: SITE_URL,
  },
  publisher: {
    "@type": "Organization",
    name: "HandsOff",
    url: SITE_URL,
  },
  proficiencyLevel: "Expert",
};

export default function ApiDocsLayout({ children }: { children: ReactNode }) {
  return (
    <>
      <script
        type="application/ld+json"
        // JSON-LD structured data for API reference pages (issue #755)
        dangerouslySetInnerHTML={{ __html: JSON.stringify(structuredData) }}
      />
      {children}
    </>
  );
}
