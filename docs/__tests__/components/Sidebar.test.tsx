import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, within, fireEvent } from "@testing-library/react";
import { Sidebar } from "../../components/Sidebar";
import { DocHeader } from "../../components/DocHeader";
import { NAV, FLAT_NAV } from "../../components/nav";

const { usePathnameMock } = vi.hoisted(() => ({ usePathnameMock: vi.fn() }));

vi.mock("next/navigation", () => ({
  usePathname: usePathnameMock,
  useRouter: () => ({ push: vi.fn() }),
}));

describe("Sidebar keyboard & ARIA audit", () => {
  beforeEach(() => {
    usePathnameMock.mockReturnValue("/docs/getting-started");
  });

  it("renders a labelled navigation landmark", () => {
    render(<Sidebar />);
    expect(screen.getByRole("navigation", { name: "Documentation" })).toBeInTheDocument();
  });

  it("marks only the active link with aria-current=page", () => {
    render(<Sidebar />);
    const nav = screen.getByRole("navigation", { name: "Documentation" });
    const active = within(nav).getByRole("link", { name: "Getting Started" });
    expect(active).toHaveAttribute("aria-current", "page");

    const current = nav.querySelectorAll('[aria-current="page"]');
    expect(current).toHaveLength(1);
  });

  it("still matches the active link when the pathname has a trailing slash", () => {
    usePathnameMock.mockReturnValue("/docs/getting-started/");
    render(<Sidebar />);
    expect(screen.getByRole("link", { name: "Getting Started" })).toHaveAttribute("aria-current", "page");
  });

  it("exposes every nav entry as a keyboard-focusable link", () => {
    render(<Sidebar />);
    const nav = screen.getByRole("navigation", { name: "Documentation" });
    for (const item of FLAT_NAV) {
      const link = within(nav).getByRole("link", { name: item.title });
      expect(link.tagName).toBe("A");
      expect(link).toHaveAttribute("href", item.href);
      expect(link).not.toHaveAttribute("tabindex", "-1");
      link.focus();
      expect(link).toHaveFocus();
    }
  });

  it("does not use tree/menu roles that would require arrow-key handling", () => {
    const { container } = render(<Sidebar />);
    expect(container.querySelector('[role="tree"], [role="treeitem"], [role="menu"], [role="menuitem"]')).toBeNull();
  });

  it("labels each section list by an existing heading element", () => {
    render(<Sidebar />);
    const lists = screen
      .getAllByRole("list")
      .filter((list) => list.hasAttribute("aria-labelledby"));
    expect(lists).toHaveLength(NAV.length);
    lists.forEach((list, i) => {
      const label = document.getElementById(list.getAttribute("aria-labelledby")!);
      expect(label).not.toBeNull();
      expect(label).toHaveTextContent(NAV[i].title);
    });
  });

  it("generates unique section ids when rendered twice (desktop + mobile drawer)", () => {
    render(
      <>
        <Sidebar />
        <Sidebar />
      </>
    );
    const ids = Array.from(document.querySelectorAll("ul[aria-labelledby]")).map((el) =>
      el.getAttribute("aria-labelledby")
    );
    expect(new Set(ids).size).toBe(ids.length);
  });

  describe("mobile drawer toggle (DocHeader)", () => {
    it("is a native button whose aria-expanded toggles and whose aria-controls target exists", () => {
      render(<DocHeader />);
      const toggle = screen.getByRole("button", { name: "Open navigation menu" });

      // Native <button> guarantees Enter/Space activation without custom key handlers.
      expect(toggle.tagName).toBe("BUTTON");
      expect(toggle).toHaveAttribute("aria-expanded", "false");

      toggle.focus();
      fireEvent.click(toggle);
      expect(toggle).toHaveAttribute("aria-expanded", "true");

      const controlsId = toggle.getAttribute("aria-controls");
      expect(controlsId).toBeTruthy();
      const drawer = document.getElementById(controlsId!);
      expect(drawer).not.toBeNull();
      expect(within(drawer!).getByRole("navigation", { name: "Documentation" })).toBeInTheDocument();

      fireEvent.click(screen.getByRole("button", { name: "Close navigation menu" }));
      expect(toggle).toHaveAttribute("aria-expanded", "false");
    });

    it("closes the drawer when a sidebar link is activated", () => {
      render(<DocHeader />);
      fireEvent.click(screen.getByRole("button", { name: "Open navigation menu" }));
      const drawer = screen.getByRole("dialog", { name: "Navigation menu" });
      fireEvent.click(within(drawer).getByRole("link", { name: "Getting Started" }));
      expect(screen.queryByRole("dialog", { name: "Navigation menu" })).toBeNull();
    });
  });
});
