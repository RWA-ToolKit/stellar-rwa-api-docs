import { describe, it, expect } from "vitest";
import { render, screen, within } from "@testing-library/react";
import { ApiEndpoint } from "../../components/ApiEndpoint";

const METHODS = [
  { method: "GET", classes: ["bg-brand-500/15", "text-brand-300", "border-brand-500/30"] },
  { method: "POST", classes: ["bg-sky-500/15", "text-sky-300", "border-sky-500/30"] },
  { method: "PUT", classes: ["bg-gold-500/15", "text-gold-300", "border-gold-500/30"] },
  { method: "DELETE", classes: ["bg-red-500/15", "text-red-300", "border-red-500/30"] },
] as const;

describe("ApiEndpoint", () => {
  describe.each(METHODS)("$method method", ({ method, classes }) => {
    it("renders the method badge with its color classes", () => {
      render(<ApiEndpoint method={method} path="/v1/assets" />);

      const badge = screen.getByText(method);
      expect(badge.tagName).toBe("SPAN");
      expect(badge).toHaveClass("font-mono", "font-bold", ...classes);
    });

    it("renders the path in a code element", () => {
      render(<ApiEndpoint method={method} path="/v1/assets/{id}" />);

      const path = screen.getByText("/v1/assets/{id}");
      expect(path.tagName).toBe("CODE");
      expect(path).toHaveClass("font-mono");
    });

    it("exposes an accessible region label", () => {
      render(<ApiEndpoint method={method} path="/v1/assets" />);

      expect(screen.getByRole("region", { name: `API endpoint: ${method} /v1/assets` })).toBeInTheDocument();
    });
  });

  it("defaults to the GET method", () => {
    render(<ApiEndpoint path="/v1/health" />);

    expect(screen.getByText("GET")).toHaveClass("text-brand-300");
    expect(screen.getByRole("region", { name: "API endpoint: GET /v1/health" })).toBeInTheDocument();
  });

  it("does not leak other methods' colors onto the badge", () => {
    render(<ApiEndpoint method="POST" path="/v1/assets" />);

    const badge = screen.getByText("POST");
    expect(badge).not.toHaveClass("text-brand-300");
    expect(badge).not.toHaveClass("text-red-300");
  });

  it("renders the description when provided", () => {
    render(<ApiEndpoint path="/v1/assets" description="List all assets." />);

    const desc = screen.getByText("List all assets.");
    expect(desc.tagName).toBe("P");
  });

  it("omits the description paragraph when not provided", () => {
    const { container } = render(<ApiEndpoint path="/v1/assets" />);

    expect(container.querySelector("p")).toBeNull();
  });

  it("renders no Request/Response panels without children", () => {
    render(<ApiEndpoint path="/v1/assets" />);

    expect(screen.queryByText("Request")).not.toBeInTheDocument();
    expect(screen.queryByText("Response")).not.toBeInTheDocument();
  });

  it("labels the first child as Request and the second as Response", () => {
    render(
      <ApiEndpoint method="POST" path="/v1/assets">
        <pre data-testid="req">req body</pre>
        <pre data-testid="res">res body</pre>
      </ApiEndpoint>,
    );

    const region = screen.getByRole("region");
    const request = within(region).getByText("Request").parentElement as HTMLElement;
    const response = within(region).getByText("Response").parentElement as HTMLElement;

    expect(within(request).getByTestId("req")).toBeInTheDocument();
    expect(within(response).getByTestId("res")).toBeInTheDocument();
  });

  it("ignores non-element children such as plain text", () => {
    render(
      <ApiEndpoint path="/v1/assets">
        just text
        <pre>only panel</pre>
      </ApiEndpoint>,
    );

    expect(screen.getByText("Request")).toBeInTheDocument();
    expect(screen.queryByText("Response")).not.toBeInTheDocument();
    expect(screen.queryByText("just text")).not.toBeInTheDocument();
  });

  it("renders an empty label for panels beyond the second", () => {
    const { container } = render(
      <ApiEndpoint path="/v1/assets">
        <pre>a</pre>
        <pre>b</pre>
        <pre>c</pre>
      </ApiEndpoint>,
    );

    const labels = container.querySelectorAll(".uppercase");
    expect(labels).toHaveLength(3);
    expect(labels[2].textContent).toBe("");
  });
});
