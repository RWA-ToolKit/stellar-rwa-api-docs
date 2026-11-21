import { DocHeader } from '@/components/DocHeader';
import type { Metadata } from 'next';

export const metadata: Metadata = {
  title: 'API Reference',
  description: 'Complete API reference for the platform.',
};

const jsonLd = {
  '@context': 'https://schema.org',
  '@type': 'TechArticle',
  headline: 'API Reference',
  description: 'Complete API reference for the platform.',
  url: 'https://example.com/docs/api',
  inLanguage: 'en',
  publisher: {
    '@type': 'Organization',
    name: 'Platform',
  },
};

export default function ApiReferencePage() {
  return (
    <>
      <script
        type="application/ld+json"
        dangerouslySetInnerHTML={{ __html: JSON.stringify(jsonLd) }}
      />
      <DocHeader
        title="API Reference"
        description="Complete API reference for the platform."
      />
    </>
  );
}
