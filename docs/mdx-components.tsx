import type { MDXComponents } from "mdx/types";
import Link from "next/link";
import { CalloutBox } from "@/components/CalloutBox";
import { CodeBlock } from "@/components/CodeBlock";
import { ApiEndpoint } from "@/components/ApiEndpoint";
import { ErrorCodeTable } from "@/components/ErrorCodeTable";

/**
 * Global MDX component map. Custom components (CalloutBox, ApiEndpoint,
 * CodeBlock) are made available to every `.mdx` page without per-file imports,
 * and internal links use the Next.js router.
 */
export function useMDXComponents(components: MDXComponents): MDXComponents {
  return {
    a: ({ href = "", children, ...props }) => {
      const isInternal = href.startsWith("/") || href.startsWith("#");
      if (isInternal) {
        return (
          <Link href={href} {...props}>
            {children}
          </Link>
        );
      }
      return (
        <a href={href} target="_blank" rel="noopener noreferrer" {...props}>
          {children}
        </a>
      );
    },
    CalloutBox,
    CodeBlock,
    ApiEndpoint,
    ErrorCodeTable,
    ...components,
  };
}
