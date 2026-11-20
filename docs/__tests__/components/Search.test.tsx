import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";

const push = vi.fn();
vi.mock("next/navigation", () => ({
  useRouter: () => ({ push }),
}));

import { Search } from "../../components/Search";

describe("Search", () => {
  it("shows an empty state when a query has zero results", () => {
    render(<Search />);
    const input = screen.getByPlaceholderText(/search docs/i);

    fireEvent.change(input, { target: { value: "zzzznotarealresult" } });

    expect(screen.getByText(/no results for/i)).toBeInTheDocument();
  });

  it("does not show an empty state before typing", () => {
    render(<Search />);
    expect(screen.queryByText(/no results for/i)).not.toBeInTheDocument();
  });

  it("shows matching results for a valid query", () => {
    render(<Search />);
    const input = screen.getByPlaceholderText(/search docs/i);

    fireEvent.change(input, { target: { value: "compliance" } });

    expect(screen.getAllByText(/compliance/i).length).toBeGreaterThan(0);
    expect(screen.queryByText(/no results for/i)).not.toBeInTheDocument();
  });

  it("moves the active selection with ArrowDown/ArrowUp", () => {
    render(<Search />);
    const input = screen.getByPlaceholderText(/search docs/i);

    fireEvent.change(input, { target: { value: "api" } });
    fireEvent.keyDown(input, { key: "ArrowDown" });

    const options = screen.getAllByRole("option");
    expect(options[0]).toHaveAttribute("aria-selected", "true");

    fireEvent.keyDown(input, { key: "ArrowUp" });
    expect(options[options.length - 1]).toHaveAttribute("aria-selected", "true");
  });

  it("navigates to the active result on Enter", () => {
    render(<Search />);
    const input = screen.getByPlaceholderText(/search docs/i);

    fireEvent.change(input, { target: { value: "api" } });
    fireEvent.keyDown(input, { key: "ArrowDown" });
    fireEvent.keyDown(input, { key: "Enter" });

    expect(push).toHaveBeenCalled();
  });

  it("closes the results on Escape", () => {
    render(<Search />);
    const input = screen.getByPlaceholderText(/search docs/i);

    fireEvent.change(input, { target: { value: "api" } });
    expect(screen.getAllByRole("option").length).toBeGreaterThan(0);

    fireEvent.keyDown(input, { key: "Escape" });

    expect(screen.queryAllByRole("option").length).toBe(0);
  });
});
