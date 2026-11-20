import type { ReactNode } from "react";
import { CalloutBox } from "@/components/CalloutBox";

interface BreakingChangeCalloutProps {
  /** Version in which the breaking change ships, e.g. "0.2.0". */
  version?: string;
  children: ReactNode;
}

/**
 * Standard callout for documenting a breaking change. Wraps `CalloutBox` with
 * a fixed "danger" styling and a consistent "Breaking change" label so every
 * breaking-change notice in the docs looks and reads the same way.
 *
 * Usage:
 * ```mdx
 * <BreakingChangeCallout version="0.2.0">
 *   `GET /assets/:id` now returns `tvl_cents` as a string instead of a
 *   number to avoid precision loss. Update any client that parses it as a
 *   numeric type.
 * </BreakingChangeCallout>
 * ```
 */
export function BreakingChangeCallout({ version, children }: BreakingChangeCalloutProps) {
  const title = version ? `Breaking change (${version})` : "Breaking change";
  return (
    <CalloutBox variant="danger" title={title}>
      {children}
    </CalloutBox>
  );
}

export default BreakingChangeCallout;
