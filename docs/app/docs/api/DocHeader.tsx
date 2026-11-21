import { usePathname } from 'next/navigation';
import { useRouter } from 'next/router';
import { useEffect, useState } from 'react';
import { useTheme } from 'next-themes';
import { ChevronRight, Copy, Check } from 'lucide-react';

interface DocHeaderProps {
  title: string;
  description?: string;
  breadcrumbs?: { label: string; href?: string }[];
}

export function DocHeader({ title, description, breadcrumbs }: DocHeaderProps) {
  const pathname = usePathname();
  const router = useRouter();
  const { resolvedTheme } = useTheme();
  const [copied, setCopied] = useState(false);
  const [mounted, setMounted] = useState(false);

  useEffect(() => {
    setMounted(true);
  }, []);

  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(window.location.href);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch {
      // clipboard unavailable
    }
  };

  const siteUrl = process.env.NEXT_PUBLIC_SITE_URL ?? 'https://handsoff.dev';
  const pageUrl = `${siteUrl}${pathname ?? router.asPath ?? ''}`;

  const jsonLd = {
    '@context': 'https://schema.org',
    '@type': 'TechArticle',
    headline: title,
    description: description ?? title,
    url: pageUrl,
    mainEntityOfPage: {
      '@type': 'WebPage',
      '@id': pageUrl,
    },
    proficiencyLevel: 'Expert',
    inLanguage: 'en',
    publisher: {
      '@type': 'Organization',
      name: 'Handsoff',
      url: siteUrl,
    },
  };

  return (
    <header className="mb-8 border-b border-border pb-6">
      <script
        type="application/ld+json"
        dangerouslySetInnerHTML={{ __html: JSON.stringify(jsonLd) }}
      />
      {breadcrumbs && breadcrumbs.length > 0 && (
        <nav aria-label="Breadcrumb" className="mb-3 flex items-center gap-1 text-sm text-muted-foreground">
          {breadcrumbs.map((crumb, index) => (
            <span key={index} className="flex items-center gap-1">
              {index > 0 && <ChevronRight className="h-3 w-3" />}
              {crumb.href ? (
                <a href={crumb.href} className="hover:text-foreground">
                  {crumb.label}
                </a>
              ) : (
                <span>{crumb.label}</span>
              )}
            </span>
          ))}
        </nav>
      )}
      <div className="flex items-start justify-between gap-4">
        <div>
          <h1 className="text-3xl font-bold tracking-tight">{title}</h1>
          {description && (
            <p className="mt-2 text-muted-foreground">{description}</p>
          )}
        </div>
        {mounted && (
          <button
            type="button"
            onClick={handleCopy}
            aria-label="Copy page link"
            className="flex items-center gap-1 rounded-md border border-border px-2 py-1 text-sm text-muted-foreground hover:text-foreground"
          >
            {copied ? <Check className="h-4 w-4" /> : <Copy className="h-4 w-4" />}
            {copied ? 'Copied' : 'Copy link'}
          </button>
        )}
      </div>
    </header>
  );
}
