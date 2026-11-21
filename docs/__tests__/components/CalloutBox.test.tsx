import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { CalloutBox } from "../../components/CalloutBox";

const VARIANTS = [
  { variant: "note", label: "Note", icon: "ℹ", border: "border-sky-500/25", bg: "bg-sky-500/[0.06]", text: "text-sky-300" },
  { variant: "tip", label: "Tip", icon: "✓", border: "border-brand-500/25", bg: "bg-brand-500/[0.06]", text: "text-brand-300" },
  { variant: "warning", label: "Warning", icon: "!", border: "border-gold-500/25", bg: "bg-gold-500/[0.06]", text: "text-gold-300" },
  { variant: "danger", label: "Important", icon: "✕", border: "border-red-500/25", bg: "bg-red-500/[0.06]", text: "text-red-300" },
  { variant: "compliance", label: "Compliance", icon: "⚖", border: "border-brand-500/30", bg: "bg-brand-500/[0.07]", text: "text-brand-300" },
] as const;

describe("CalloutBox", () => {
  describe.each(VARIANTS)("$variant variant", ({ variant, label, icon, border, bg, text }) => {
    it("renders the variant styling, icon and label", () => {
      render(<CalloutBox variant={variant}>Body text</CalloutBox>);

      const box = screen.getByRole("note", { name: label });
      expect(box).toHaveAttribute("data-variant", variant);
      expect(box).toHaveClass(border, bg);

      const iconEl = screen.getByText(icon);
      expect(iconEl).toHaveAttribute("aria-hidden", "true");
      expect(iconEl.parentElement).toHaveClass(text);

      expect(screen.getByText(label)).toBeInTheDocument();
      expect(screen.getByText("Body text")).toBeInTheDocument();
    });

    it("includes the title in the label and accessible name", () => {
      render(
        <CalloutBox variant={variant} title="Heads up">
          Body text
        </CalloutBox>,
      );

      expect(screen.getByRole("note", { name: `${label}: Heads up` })).toBeInTheDocument();
      expect(screen.getByText(": Heads up")).toBeInTheDocument();
    });
  });

  it("defaults to the note variant", () => {
    render(<CalloutBox>Default body</CalloutBox>);

    const box = screen.getByRole("note", { name: "Note" });
    expect(box).toHaveAttribute("data-variant", "note");
    expect(box).toHaveClass("border-sky-500/25", "bg-sky-500/[0.06]");
    expect(screen.getByText("ℹ")).toBeInTheDocument();
  });

  it("uses role=note rather than alert for every variant", () => {
    render(
      <>
        {VARIANTS.map(({ variant }) => (
          <CalloutBox key={variant} variant={variant}>
            {variant}
          </CalloutBox>
        ))}
      </>,
    );

    expect(screen.getAllByRole("note")).toHaveLength(VARIANTS.length);
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
  });
});
