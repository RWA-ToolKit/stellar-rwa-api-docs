import { describe, it, expect, vi, afterEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { CodeBlock } from "../../components/CodeBlock";

describe("CodeBlock copy button", () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("is a focusable button with an accessible name and focus-visible style", () => {
    render(<CodeBlock title="example.ts">const x = 1;</CodeBlock>);
    const button = screen.getByRole("button", { name: /copy/i });
    expect(button.tagName).toBe("BUTTON");
    expect(button).toHaveAttribute("type", "button");
    expect(button.className).toMatch(/focus-visible:outline/);
    button.focus();
    expect(button).toHaveFocus();
  });

  it("copies the code and shows Copied", async () => {
    const writeText = vi.fn().mockResolvedValue(undefined);
    vi.stubGlobal("navigator", { clipboard: { writeText } });
    render(<CodeBlock title="example.ts">const x = 1;</CodeBlock>);
    fireEvent.click(screen.getByRole("button", { name: /copy/i }));
    expect(writeText).toHaveBeenCalledWith("const x = 1;");
    await waitFor(() => expect(screen.getAllByText("Copied").length).toBeGreaterThan(0));
  });

  it("shows Failed when the clipboard write rejects", async () => {
    const writeText = vi.fn().mockRejectedValue(new Error("denied"));
    vi.stubGlobal("navigator", { clipboard: { writeText } });
    render(<CodeBlock title="example.ts">const x = 1;</CodeBlock>);
    fireEvent.click(screen.getByRole("button", { name: /copy/i }));
    await waitFor(() => expect(screen.getByText("Failed")).toBeInTheDocument());
  });
});
