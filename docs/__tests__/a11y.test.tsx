import React from "react";
import { describe, it, expect, vi } from "vitest";
import { render } from "@testing-library/react";
import { MDXProvider } from "@mdx-js/react";
import axe from "axe-core";

import HomePage from "@/app/page";
// @ts-ignore -- .mdx modules are compiled by @mdx-js/rollup in vitest.config.ts
import GettingStarted from "@/app/docs/getting-started/page.mdx";
// @ts-ignore
import ApiOverview from "@/app/docs/api/overview/page.mdx";
import { useMDXComponents } from "@/mdx-components";

vi.mock("next/navigation", () => ({
  usePathname: () => "/",
  useRouter: () => ({ push: vi.fn(), replace: vi.fn(), prefetch: vi.fn() }),
  useSearchParams: () => new URLSearchParams(),
}));

/**
 * Rules disabled (everything else in axe's default set runs):
 * - `region`: pages are rendered as fragments without the layout, so the
 *   landmark/skip-link structure supplied by app/layout.tsx is absent.
 * - `color-contrast`: jsdom has no layout/CSS engine, so axe cannot compute
 *   colors reliably and the rule yields false results.
 */
const AXE_OPTIONS: axe.RunOptions = {
  rules: {
    region: { enabled: false },
    "color-contrast": { enabled: false },
  },
};

async function expectNoViolations(container: HTMLElement) {
  const results = await axe.run(container, AXE_OPTIONS);
  const message = results.violations
    .map(
      (v) =>
        `${v.id} (${v.impact}): ${v.help}\n` +
        v.nodes.map((n) => `  - ${n.target.join(" ")}\n    ${n.html}`).join("\n"),
    )
    .join("\n\n");
  expect(results.violations, message).toEqual([]);
}

function renderMdx(Page: React.ComponentType) {
  return render(
    <MDXProvider components={useMDXComponents({})}>
      <main>
        <Page />
      </main>
    </MDXProvider>,
  );
}

describe("accessibility (axe-core)", () => {
  it("home page has no violations", async () => {
    const { container } = render(<HomePage />);
    await expectNoViolations(container);
  });

  it("getting-started page has no violations", async () => {
    const { container } = renderMdx(GettingStarted);
    await expectNoViolations(container);
  });

  it("api/overview page has no violations", async () => {
    const { container } = renderMdx(ApiOverview);
    await expectNoViolations(container);
  });
});
