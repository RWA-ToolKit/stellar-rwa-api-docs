import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import NotFound from "./not-found";

describe("NotFound", () => {
  it("renders a 404 indicator", () => {
    render(<NotFound />);
    expect(screen.getByText("404")).toBeInTheDocument();
  });

  it("tells the reader the page doesn't exist", () => {
    render(<NotFound />);
    expect(screen.getByRole("heading", { name: "Page not found" })).toBeInTheDocument();
    expect(
      screen.getByText(/that documentation page doesn't exist/i),
    ).toBeInTheDocument();
  });

  it("links back to the home page", () => {
    render(<NotFound />);
    const homeLink = screen.getByRole("link", { name: "Home" });
    expect(homeLink).toHaveAttribute("href", "/");
  });

  it("links to the getting-started guide as a recovery path", () => {
    render(<NotFound />);
    const gettingStartedLink = screen.getByRole("link", { name: "Getting Started" });
    expect(gettingStartedLink).toHaveAttribute("href", "/docs/getting-started");
  });

  it("exports a default function component with no required props", () => {
    expect(typeof NotFound).toBe("function");
    expect(NotFound.length).toBe(0);
  });
});
