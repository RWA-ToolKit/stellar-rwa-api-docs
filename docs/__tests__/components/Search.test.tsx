import { render, screen, fireEvent } from "@testing-library/react";
import { Search } from "@/components/Search";

const mockPush = jest.fn();

jest.mock("next/navigation", () => ({
  useRouter: () => ({ push: mockPush }),
}));

jest.mock("@/lib/search", () => ({
  search: (query: string) => {
    if (query.toLowerCase().includes("install")) {
      return [
        { title: "Installation", section: "Getting Started", href: "/docs/install", excerpt: "How to install" },
        { title: "Install Config", section: "Getting Started", href: "/docs/install-config", excerpt: "Configure install" },
      ];
    }
    if (query.toLowerCase().includes("api")) {
      return [{ title: "API Reference", section: "API", href: "/docs/api", excerpt: "API docs" }];
    }
    return [];
  },
}));

describe("Search", () => {
  beforeEach(() => {
    mockPush.mockClear();
  });

  it("renders the search input", () => {
    render(<Search />);
    expect(screen.getByRole("combobox", { name: /search documentation/i })).toBeInTheDocument();
  });

  it("opens results when typing a matching query", () => {
    render(<Search />);
    const input = screen.getByRole("combobox");
    fireEvent.change(input, { target: { value: "install" } });
    expect(screen.getByRole("listbox")).toBeInTheDocument();
    expect(screen.getByText("Installation")).toBeInTheDocument();
    expect(screen.getByText("Install Config")).toBeInTheDocument();
  });

  it("shows an empty state when there are no results", () => {
    render(<Search />);
    const input = screen.getByRole("combobox");
    fireEvent.change(input, { target: { value: "zzzz" } });
    expect(screen.queryByRole("listbox")).not.toBeInTheDocument();
    expect(screen.getByText(/no results for/i)).toBeInTheDocument();
  });

  it("closes results on Escape", () => {
    render(<Search />);
    const input = screen.getByRole("combobox");
    fireEvent.change(input, { target: { value: "install" } });
    expect(screen.getByRole("listbox")).toBeInTheDocument();
    fireEvent.keyDown(input, { key: "Escape" });
    expect(screen.queryByRole("listbox")).not.toBeInTheDocument();
  });

  it("moves the active result down with ArrowDown", () => {
    render(<Search />);
    const input = screen.getByRole("combobox");
    fireEvent.change(input, { target: { value: "install" } });

    const options = screen.getAllByRole("option");
    expect(options[0]).toHaveAttribute("aria-selected", "false");

    fireEvent.keyDown(input, { key: "ArrowDown" });
    expect(screen.getAllByRole("option")[0]).toHaveAttribute("aria-selected", "true");
    expect(input).toHaveAttribute("aria-activedescendant", "search-result-0");

    fireEvent.keyDown(input, { key: "ArrowDown" });
    expect(screen.getAllByRole("option")[1]).toHaveAttribute("aria-selected", "true");
    expect(input).toHaveAttribute("aria-activedescendant", "search-result-1");
  });

  it("wraps around when ArrowDown passes the last result", () => {
    render(<Search />);
    const input = screen.getByRole("combobox");
    fireEvent.change(input, { target: { value: "install" } });

    fireEvent.keyDown(input, { key: "ArrowDown" });
    fireEvent.keyDown(input, { key: "ArrowDown" });
    fireEvent.keyDown(input, { key: "ArrowDown" });
    expect(screen.getAllByRole("option")[0]).toHaveAttribute("aria-selected", "true");
  });

  it("moves the active result up with ArrowUp and wraps to the last", () => {
    render(<Search />);
    const input = screen.getByRole("combobox");
    fireEvent.change(input, { target: { value: "install" } });

    fireEvent.keyDown(input, { key: "ArrowUp" });
    expect(screen.getAllByRole("option")[1]).toHaveAttribute("aria-selected", "true");

    fireEvent.keyDown(input, { key: "ArrowUp" });
    expect(screen.getAllByRole("option")[0]).toHaveAttribute("aria-selected", "true");
  });

  it("navigates to the highlighted result on Enter", () => {
    render(<Search />);
    const input = screen.getByRole("combobox");
    fireEvent.change(input, { target: { value: "install" } });

    fireEvent.keyDown(input, { key: "ArrowDown" });
    fireEvent.keyDown(input, { key: "ArrowDown" });
    fireEvent.keyDown(input, { key: "Enter" });

    expect(mockPush).toHaveBeenCalledWith("/docs/install-config");
  });

  it("does not navigate on Enter when no result is highlighted", () => {
    render(<Search />);
    const input = screen.getByRole("combobox");
    fireEvent.change(input, { target: { value: "install" } });

    fireEvent.keyDown(input, { key: "Enter" });
    expect(mockPush).not.toHaveBeenCalled();
  });
});
