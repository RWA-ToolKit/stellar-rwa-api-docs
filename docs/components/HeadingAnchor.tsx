import { Children, isValidElement, type HTMLAttributes, type ReactNode } from "react";

/** Flattens a React node tree (text, inline code, links, …) to plain text. */
function textOf(node: ReactNode): string {
  if (typeof node === "string" || typeof node === "number") return String(node);
  if (Array.isArray(node)) return node.map(textOf).join("");
  if (isValidElement<{ children?: ReactNode }>(node)) return textOf(node.props.children);
  return Children.toArray(node).map(textOf).join("");
}

/**
 * Derives a stable, URL-safe id from heading text, e.g.
 * "Reading contracts directly" -> "reading-contracts-directly",
 * "`is_allowed`" -> "is-allowed". The id only changes if the heading text does.
 */
export function slugify(text: string): string {
  return text
    .toLowerCase()
    .trim()
    .replace(/[^a-z0-9\s_-]/g, "")
    .replace(/[\s_]+/g, "-")
    .replace(/-+/g, "-")
    .replace(/^-|-$/g, "");
}

type Level = 1 | 2 | 3 | 4 | 5 | 6;
type HeadingProps = HTMLAttributes<HTMLHeadingElement>;

/**
 * Builds an MDX heading component that gets a slug id and a "#" anchor link.
 * The link is revealed on hover and on keyboard focus (see `.heading-anchor`).
 */
export function createHeading(level: Level) {
  const Tag = `h${level}` as const;
  function Heading({ id, children, ...props }: HeadingProps) {
    const slug = id ?? slugify(textOf(children));
    return (
      <Tag id={slug} {...props}>
        {children}
        {slug && (
          <a href={`#${slug}`} className="heading-anchor" aria-label={`Link to this section: ${textOf(children)}`}>
            #
          </a>
        )}
      </Tag>
    );
  }
  Heading.displayName = `Heading${level}`;
  return Heading;
}
