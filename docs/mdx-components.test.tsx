import { describe, expect, it } from "vitest";
import { render, screen } from "@testing-library/react";
import { useMDXComponents } from "./mdx-components";

/**
 * Tests for the global MDX component map. Each override is resolved from
 * `useMDXComponents({})` and rendered in isolation to confirm it produces
 * the expected markup / behavior.
 */
describe("useMDXComponents", () => {
  const components = useMDXComponents({});

  it("includes the custom component overrides and passes through extras", () => {
    expect(components.CalloutBox).toBeDefined();
    expect(components.BreakingChangeCallout).toBeDefined();
    expect(components.CodeBlock).toBeDefined();
    expect(components.ApiEndpoint).toBeDefined();
    expect(components.a).toBeDefined();

    const Extra = () => null;
    const merged = useMDXComponents({ Extra } as never);
    expect(merged.Extra).toBe(Extra);
  });

  describe("a (link override)", () => {
    const Anchor = components.a as React.ComponentType<
      React.AnchorHTMLAttributes<HTMLAnchorElement>
    >;

    it("renders internal paths with next/link (no target/rel added)", () => {
      render(<Anchor href="/docs/getting-started">Getting started</Anchor>);
      const link = screen.getByRole("link", { name: "Getting started" });
      expect(link).toHaveAttribute("href", "/docs/getting-started");
      expect(link).not.toHaveAttribute("target");
      expect(link).not.toHaveAttribute("rel");
    });

    it("renders in-page hash links as internal", () => {
      render(<Anchor href="#section">Section</Anchor>);
      const link = screen.getByRole("link", { name: "Section" });
      expect(link).toHaveAttribute("href", "#section");
      expect(link).not.toHaveAttribute("target");
    });

    it("renders external links with target=_blank and safe rel", () => {
      render(<Anchor href="https://example.com">External</Anchor>);
      const link = screen.getByRole("link", { name: "External" });
      expect(link).toHaveAttribute("href", "https://example.com");
      expect(link).toHaveAttribute("target", "_blank");
      expect(link).toHaveAttribute("rel", "noopener noreferrer");
    });

    it("treats a missing href as internal (defaults to empty string)", () => {
      render(<Anchor>No href</Anchor>);
      const link = screen.getByRole("link", { name: "No href" });
      expect(link).not.toHaveAttribute("target");
    });
  });

  describe("CalloutBox override", () => {
    it("renders with its default note styling and label", () => {
      const Callout = components.CalloutBox as React.ComponentType<{
        children: React.ReactNode;
      }>;
      render(<Callout>Heads up.</Callout>);
      expect(screen.getByText("Note")).toBeInTheDocument();
      expect(screen.getByText("Heads up.")).toBeInTheDocument();
    });
  });

  describe("BreakingChangeCallout override", () => {
    it("renders the breaking-change label with its version", () => {
      const Breaking = components.BreakingChangeCallout as React.ComponentType<{
        version?: string;
        children: React.ReactNode;
      }>;
      render(<Breaking version="0.2.0">Field type changed.</Breaking>);
      expect(screen.getByText("Breaking change (0.2.0)")).toBeInTheDocument();
      expect(screen.getByText("Field type changed.")).toBeInTheDocument();
    });
  });

  describe("ApiEndpoint override", () => {
    it("renders the method and path", () => {
      const Endpoint = components.ApiEndpoint as React.ComponentType<{
        method?: "GET" | "POST" | "PUT" | "DELETE";
        path: string;
      }>;
      render(<Endpoint method="GET" path="/assets/:id" />);
      expect(screen.getByText("GET")).toBeInTheDocument();
      expect(screen.getByText("/assets/:id")).toBeInTheDocument();
    });
  });
});
