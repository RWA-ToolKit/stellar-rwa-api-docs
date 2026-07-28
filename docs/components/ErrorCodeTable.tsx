interface ErrorCode {
  code: number;
  name: string;
  description: string;
}

interface ErrorCodeTableProps {
  contract: string;
  codes: ErrorCode[];
}

export function ErrorCodeTable({ contract, codes }: ErrorCodeTableProps) {
  return (
    <div className="my-6">
      <CalloutBox variant="warning" title={`${contract} Error Codes`}>
        <p className="text-sm mb-4">
          Error codes are <strong>contract-specific</strong>. The same numeric code means different things across contracts.
          Always refer to the error table for the specific contract you're calling.
        </p>
      </CalloutBox>
      <div className="overflow-x-auto">
        <table className="w-full text-sm border-collapse">
          <thead>
            <tr className="border-b border-white/10">
              <th className="text-left px-4 py-2 font-semibold text-base-100">Code</th>
              <th className="text-left px-4 py-2 font-semibold text-base-100">Error Name</th>
              <th className="text-left px-4 py-2 font-semibold text-base-100">Description</th>
            </tr>
          </thead>
          <tbody>
            {codes.map((error) => (
              <tr key={error.code} className="border-b border-white/5">
                <td className="px-4 py-3 text-brand-300 font-mono">{error.code}</td>
                <td className="px-4 py-3 font-mono text-base-200">{error.name}</td>
                <td className="px-4 py-3 text-base-300">{error.description}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}

interface CalloutBoxProps {
  variant?: "warning" | "compliance" | "default";
  title?: string;
  children: React.ReactNode;
}

function CalloutBox({ variant = "default", title, children }: CalloutBoxProps) {
  const variantClasses = {
    warning: "bg-amber-900/20 border-l-4 border-amber-500",
    compliance: "bg-brand-900/20 border-l-4 border-brand-400",
    default: "bg-base-900/20 border-l-4 border-base-400",
  };

  return (
    <div className={`px-4 py-3 rounded ${variantClasses[variant]}`}>
      {title && <h4 className="font-semibold mb-2 text-base-100">{title}</h4>}
      {children}
    </div>
  );
}
