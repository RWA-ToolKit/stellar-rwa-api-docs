import type { Metadata } from "next";
import { Sidebar } from "@/components/Sidebar";
import { PrevNext } from "@/components/PrevNext";
import { VersionBanner } from "@/components/VersionBanner";

/**
 * Per-page Open Graph metadata for documentation pages.
 *
 * Next.js resolves `metadata` from the nearest layout/page, so this layout
 * provides a meaningful default title/description for every docs page while
 * still allowing individual pages to override it via their own `metadata`
 * export. The `title.template` appends the site name so shared links preview
 * with a distinct, page-specific heading.
 */
export const metadata: Metadata = {
  title: {
    default: "Documentation",
    template: "%s | Documentation",
  },
  description:
    "Guides, references, and examples for building with the platform.",
  openGraph: {
    type: "article",
    siteName: "Documentation",
    title: {
      default: "Documentation",
      template: "%s | Documentation",
    },
    description:
      "Guides, references, and examples for building with the platform.",
  },
};

/** Two-column documentation shell: sticky sidebar + prose article. */
export default function DocsLayout({ children }: { children: React.ReactNode }) {
  return (
    <div className="mx-auto flex max-w-screen-2xl gap-8 px-4 sm:px-6">
      <aside className="hidden w-64 shrink-0 lg:block">
        <div className="sticky top-16 max-h-[calc(100vh-4rem)] overflow-y-auto py-8 pr-2">
          <Sidebar />
        </div>
      </aside>

      <main id="main-content" className="min-w-0 flex-1 overflow-x-hidden py-10">
        <div className="mx-auto w-full max-w-3xl">
          <VersionBanner />
        </div>
        <article className="prose mx-auto w-full max-w-3xl overflow-x-hidden">{children}</article>
        <div className="mx-auto w-full max-w-3xl">
          <PrevNext />
        </div>
      </main>
    </div>
  );
}
