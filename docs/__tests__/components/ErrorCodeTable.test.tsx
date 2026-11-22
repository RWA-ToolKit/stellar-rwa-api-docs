import { describe, it, expect } from "vitest";
import { render, screen, within } from "@testing-library/react";
import ErrorCodeTable, { ErrorCodeTable as NamedErrorCodeTable } from "../../components/ErrorCodeTable";

const SAMPLE_CODES = [
  {
    code: 1,
    name: "NotInitialized",
    cause: "The contract has not been initialized.",
    remediation: "Call initialize before invoking other methods.",
  },
  {
    code: 2,
    name: "Unauthorized",
    cause: "Caller is not the admin.",
    remediation: "Sign the transaction with the admin account.",
  },
  {
    code: 7,
    name: "InsufficientBalance",
    cause: "Balance is lower than the requested amount.",
    remediation: "Top up the balance and retry.",
  },
];

describe("ErrorCodeTable", () => {
  it("exports the same component as default and named export", () => {
    expect(ErrorCodeTable).toBe(NamedErrorCodeTable);
  });

  it("renders a table with a screen-reader caption naming the contract", () => {
    render(<ErrorCodeTable contract="Token" codes={SAMPLE_CODES} />);

    const table = screen.getByRole("table", { name: "Token contract error codes" });
    expect(table).toBeInTheDocument();
    expect(table.querySelector("caption")).toHaveClass("sr-only");
  });

  it("renders the four column headers with scope=col", () => {
    render(<ErrorCodeTable contract="Token" codes={SAMPLE_CODES} />);

    const headers = screen.getAllByRole("columnheader");
    expect(headers.map((h) => h.textContent)).toEqual(["Code", "Error Name", "Cause", "Remediation"]);
    headers.forEach((h) => expect(h).toHaveAttribute("scope", "col"));
  });

  it("renders one body row per error code plus the header row", () => {
    render(<ErrorCodeTable contract="Token" codes={SAMPLE_CODES} />);

    const rows = screen.getAllByRole("row");
    expect(rows).toHaveLength(SAMPLE_CODES.length + 1);
  });

  it("renders code, name, cause and remediation in each row", () => {
    render(<ErrorCodeTable contract="Token" codes={SAMPLE_CODES} />);

    const rows = screen.getAllByRole("row").slice(1);
    SAMPLE_CODES.forEach((error, i) => {
      const cells = within(rows[i]).getAllByRole("cell");
      expect(cells.map((c) => c.textContent)).toEqual([
        String(error.code),
        error.name,
        error.cause,
        error.remediation,
      ]);
    });
  });

  it("renders codes and names in monospace", () => {
    render(<ErrorCodeTable contract="Token" codes={SAMPLE_CODES} />);

    expect(screen.getByText("1")).toHaveClass("font-mono");
    expect(screen.getByText("NotInitialized")).toHaveClass("font-mono");
    expect(screen.getByText("Caller is not the admin.")).not.toHaveClass("font-mono");
  });

  it("preserves the order of the codes passed in", () => {
    render(<ErrorCodeTable contract="Token" codes={[...SAMPLE_CODES].reverse()} />);

    const rows = screen.getAllByRole("row").slice(1);
    const names = rows.map((r) => within(r).getAllByRole("cell")[1].textContent);
    expect(names).toEqual(["InsufficientBalance", "Unauthorized", "NotInitialized"]);
  });

  it("renders a focusable scrollable region labelled with the contract", () => {
    render(<ErrorCodeTable contract="Escrow" codes={SAMPLE_CODES} />);

    const region = screen.getByRole("region", { name: "Escrow error codes table" });
    expect(region).toHaveAttribute("tabindex", "0");
    expect(region).toHaveClass("overflow-x-auto");
    expect(within(region).getByRole("table")).toBeInTheDocument();
  });

  it("renders the contract-specific warning callout with the contract title", () => {
    render(<ErrorCodeTable contract="Escrow" codes={SAMPLE_CODES} />);

    expect(screen.getByRole("heading", { level: 3, name: "Escrow Error Codes" })).toBeInTheDocument();
    expect(screen.getByText("contract-specific")).toBeInTheDocument();
    expect(screen.getByText(/The same numeric code means different things across contracts/)).toBeInTheDocument();
  });

  it("renders only the header row when codes is empty", () => {
    render(<ErrorCodeTable contract="Token" codes={[]} />);

    expect(screen.getByRole("table")).toBeInTheDocument();
    expect(screen.getAllByRole("columnheader")).toHaveLength(4);
    expect(screen.getAllByRole("row")).toHaveLength(1);
    expect(screen.queryAllByRole("cell")).toHaveLength(0);
  });
});
