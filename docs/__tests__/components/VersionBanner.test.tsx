import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { VersionBanner } from "../../components/VersionBanner";

describe("VersionBanner", () => {
  it("renders the stale-version notice when docs trail the deployed contract version", () => {
    render(<VersionBanner docsVersion="v1.0.0" contractVersion="v1.2.0" />);

    expect(screen.getByText(/api version/i)).toBeInTheDocument();
    expect(screen.getByText("v1.0.0")).toBeInTheDocument();
    expect(screen.getByText("v1.2.0")).toBeInTheDocument();
  });

  it("is absent when the docs version matches the deployed contract version", () => {
    const { container } = render(<VersionBanner docsVersion="v1.2.0" contractVersion="v1.2.0" />);
    expect(container).toBeEmptyDOMElement();
  });

  it("is absent when the docs version is ahead of the deployed contract version", () => {
    const { container } = render(<VersionBanner docsVersion="v2.0.0" contractVersion="v1.2.0" />);
    expect(container).toBeEmptyDOMElement();
  });

  it("is absent when there is no docs version", () => {
    const { container } = render(<VersionBanner docsVersion="" contractVersion="v1.2.0" />);
    expect(container).toBeEmptyDOMElement();
  });
});
