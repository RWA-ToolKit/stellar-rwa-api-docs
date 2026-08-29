import Link from "next/link";

const CARDS = [
  {
    title: "Getting Started",
    body: "What the toolkit is, how RWA tokenization works, and how to run everything locally.",
    href: "/docs/getting-started",
  },
  {
    title: "Contract Reference",
    body: "Full function reference for the asset-token, compliance, registry and dividend contracts.",
    href: "/docs/contracts/asset-token",
  },
  {
    title: "API Reference",
    body: "REST endpoints for indexed assets, holders, compliance summaries and dividends.",
    href: "/docs/api/overview",
  },
  {
    title: "Compliance Guide",
    body: "The core of an RWA platform — allowlists, transfer gating, jurisdictions, KYC expiry.",
    href: "/docs/compliance-guide",
  },
  {
    title: "Web App Guide",
    body: "Connect Freighter, tokenize an asset, manage compliance, and distribute dividends.",
    href: "/docs/web-app",
  },
  {
    title: "Integration",
    body: "Query the API, use the TypeScript examples, and call the contracts directly.",
    href: "/docs/integration",
  },
];

export default function DocsHome() {
  return (
    <main id="main-content" className="mx-auto max-w-4xl px-4 py-16 sm:px-6 sm:py-24">
      <span className="chip border border-brand-500/25 bg-brand-500/10 text-brand-300">
        <span className="h-1.5 w-1.5 rounded-full bg-brand-400" />
        Stellar RWA Toolkit
      </span>
      <h1 className="mt-6 text-4xl font-bold tracking-tight text-base-50 sm:text-5xl">
        Tokenize real-world assets on Stellar
      </h1>
      <p className="mt-5 max-w-2xl text-lg text-base-200/75">
        A complete toolkit for bringing real estate, invoices and commodities on-chain
        as <strong className="text-base-100">compliance-gated tokens</strong>. These docs
        cover the Soroban contracts, the indexing REST API, and the web app.
      </p>
      <div className="mt-8 flex flex-wrap gap-3">
        <Link href="/docs/getting-started" className="rounded-xl bg-brand-500 px-5 py-3 text-sm font-semibold text-base-950 hover:bg-brand-400">
          Get started
        </Link>
        <Link href="/docs/compliance-guide" className="rounded-xl border border-white/10 bg-white/5 px-5 py-3 text-sm font-semibold text-base-100 hover:bg-white/10">
          Read the compliance guide
        </Link>
      </div>

      <div className="mt-14 grid grid-cols-1 gap-4 sm:grid-cols-2">
        {CARDS.map((c) => (
          <Link
            key={c.href}
            href={c.href}
            className="group rounded-2xl border border-white/5 bg-base-900/60 p-6 transition-colors hover:border-brand-500/40 hover:bg-base-850/70"
          >
            <h2 className="text-base font-semibold text-base-50 group-hover:text-brand-300">
              {c.title}
            </h2>
            <p className="mt-2 text-sm leading-relaxed text-base-200/70">{c.body}</p>
          </Link>
        ))}
      </div>
    </main>
  );
}
