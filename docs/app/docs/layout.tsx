import { Sidebar } from "@/components/Sidebar";
import { PrevNext } from "@/components/PrevNext";
import { VersionBanner } from "@/components/VersionBanner";

/** Two-column documentation shell: sticky sidebar + prose article. */
export default function DocsLayout({ children }: { children: React.ReactNode }) {
  return (
    <div className="mx-auto flex max-w-screen-2xl gap-8 px-4 sm:px-6">
      <aside className="hidden w-64 shrink-0 lg:block">
        <div className="sticky top-16 max-h-[calc(100vh-4rem)] overflow-y-auto py-8 pr-2">
          <Sidebar />
        </div>
      </aside>

      <main className="min-w-0 flex-1 py-10">
        <div className="mx-auto w-full max-w-3xl">
          <VersionBanner />
        </div>
        <article className="prose mx-auto w-full max-w-3xl">{children}</article>
        <div className="mx-auto w-full max-w-3xl">
          <PrevNext />
        </div>
      </main>
    </div>
  );
}
