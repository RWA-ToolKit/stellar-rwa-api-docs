import type { MDXComponents } from "mdx/types";
import Link from "next/link";
import { CalloutBox } from "@/components/CalloutBox";
import { CodeBlock } from "@/components/CodeBlock";
import { ApiEndpoint } from "@/components/ApiEndpoint";
import { ErrorCodeTable } from "@/components/ErrorCodeTable";
import { createHeading } from "@/components/HeadingAnchor";
import { LastReviewed } from "@/components/DocHeader";

const h2 = createHeading(2);
const h3 = createHeading(3);
const h4 = createHeading(4);
const h5 = createHeading(5);
const h6 = createHeading(6);

/**
 * Global MDX component map. Custom components are available to every `.mdx`
 * page without per-file imports, internal links use the Next.js router, and
 * h2–h6 headings get a stable slug id plus a keyboard-accessible "#" anchor.
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
    pre: ({ children, ...props }) => (
      <pre
        tabIndex={0}
        role="region"
        aria-label="Code sample"
        className="focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-brand-400"
        {...props}
      >
        {children}
      </pre>
    ),
    table: ({ children, ...props }) => (
      <div
        tabIndex={0}
        role="region"
        aria-label="Data table"
        className="overflow-x-auto focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-brand-400 rounded-lg"
      >
        <table {...props}>{children}</table>
      </div>
    ),
    h2,
    h3,
    h4,
    h5,
    h6,
    CalloutBox,
    CodeBlock,
    ApiEndpoint,
    ErrorCodeTable,
    LastReviewed,
    ...components,
  };
}
