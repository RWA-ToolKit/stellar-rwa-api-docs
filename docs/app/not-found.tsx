import Link from "next/link";

export default function NotFound() {
  return (
    <main id="main-content" className="mx-auto flex min-h-[60vh] max-w-2xl flex-col items-center justify-center px-4 text-center">
      <p className="font-mono text-sm text-brand-400">404</p>
      <h1 className="mt-3 text-3xl font-bold tracking-tight text-base-50">Page not found</h1>
      <p className="mt-3 text-base-200/70">
        That documentation page doesn&apos;t exist. Try the getting-started guide or
        browse the sidebar.
      </p>
      <div className="mt-8 flex gap-3">
        <Link href="/" className="rounded-xl bg-brand-500 px-5 py-2.5 text-sm font-semibold text-base-950 hover:bg-brand-400">
          Home
        </Link>
        <Link href="/docs/getting-started" className="rounded-xl border border-white/10 bg-white/5 px-5 py-2.5 text-sm font-semibold text-base-100 hover:bg-white/10">
          Getting Started
        </Link>
      </div>
    </main>
  );
}
