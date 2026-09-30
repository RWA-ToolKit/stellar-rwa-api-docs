#!/usr/bin/env node
/**
 * Build-time search index generator for the documentation.
 *
 * Scans all `.mdx` files under `docs/app/docs/`, extracts page metadata,
 * cleans body text, segments pages into headed sections with anchor links,
 * and outputs a static search index to `docs/lib/search-index.json`.
 *
 * Runs at build time (`npm run prebuild`) and can be run manually via
 * `npm run build:search`.
 */

import { readFileSync, writeFileSync, readdirSync, statSync } from "node:fs";
import { join, dirname, relative } from "node:path";
import { fileURLToPath } from "node:url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

const DOCS_APP_ROOT = join(__dirname, "..", "app", "docs");
const NAV_FILE = join(__dirname, "..", "components", "nav-data.json");
const OUTPUT_FILE = join(__dirname, "..", "lib", "search-index.json");

const NAV = JSON.parse(readFileSync(NAV_FILE, "utf-8"));
const SECTION_MAP = new Map(
  NAV.flatMap(({ title, items }) => items.map(({ href }) => [href, title])),
);

function getSectionForRoute(route) {
  if (SECTION_MAP.has(route)) return SECTION_MAP.get(route);
  if (route.startsWith("/docs/api/")) return "API Reference";
  if (route.startsWith("/docs/contracts/")) return "Contract Reference";
  if (route.startsWith("/docs/")) return "Documentation";
  return "General";
}

function findMdxFiles(dir) {
  const results = [];
  const entries = readdirSync(dir);
  for (const entry of entries) {
    const fullPath = join(dir, entry);
    const stat = statSync(fullPath);
    if (stat.isDirectory()) {
      results.push(...findMdxFiles(fullPath));
    } else if (entry.endsWith(".mdx")) {
      results.push(fullPath);
    }
  }
  return results;
}

function slugify(text) {
  return text
    .toLowerCase()
    .replace(/[^\w\s-]/g, "")
    .trim()
    .replace(/\s+/g, "-");
}

function cleanMarkdown(text) {
  return (
    text
      // Remove code fence delimiters but keep code content inside
      .replace(/^```[a-zA-Z0-9_-]*\s*$/gm, " ")
      .replace(/^```\s*$/gm, " ")
      // Replace inline code backticks with surrounding spaces
      .replace(/`([^`]+)`/g, " $1 ")
      // Remove JSX / HTML tags
      .replace(/<[^>]+>/g, " ")
      // Remove markdown links [text](url) -> text
      .replace(/\[([^\]]+)\]\([^)]+\)/g, " $1 ")
      // Remove markdown headers
      .replace(/^#{1,6}\s+/gm, "")
      // Remove bold/italics only when preceded/followed by whitespace or boundary (preserving snake_case identifiers like total_supply)
      .replace(/(?:^|\s)\*\*([^*]+)\*\*(?=\s|[.,;:!?]|$)/g, " $1 ")
      .replace(/(?:^|\s)\*([^*]+)\*(?=\s|[.,;:!?]|$)/g, " $1 ")
      .replace(/(?:^|\s)__([^_]+)__(?=\s|[.,;:!?]|$)/g, " $1 ")
      .replace(/(?:^|\s)_([^_]+)_(?=\s|[.,;:!?]|$)/g, " $1 ")
      // Remove table formatting
      .replace(/\|/g, " ")
      .replace(/[-:]{3,}/g, " ")
      // Collapse whitespace
      .replace(/\s+/g, " ")
      .trim()
  );
}

function extractMetadata(content, filePath) {
  const metaMatch = content.match(/export\s+const\s+metadata\s*=\s*({[\s\S]*?});/);
  let title = "";
  let description = "";
  let canonical = "";

  if (metaMatch) {
    try {
      const metaBlock = metaMatch[1];
      const titleMatch = metaBlock.match(/title:\s*["']([^"']+)["']/);
      const descMatch = metaBlock.match(/description:\s*["']([^"']+)["']/);
      const canonMatch = metaBlock.match(/canonical:\s*["']([^"']+)["']/);

      if (titleMatch) title = titleMatch[1];
      if (descMatch) description = descMatch[1];
      if (canonMatch) canonical = canonMatch[1];
    } catch {
      // Fallback
    }
  }

  if (!canonical) {
    const rel = relative(join(__dirname, "..", "app"), filePath)
      .replace(/\/page\.mdx$/, "")
      .replace(/\\/g, "/");
    canonical = "/" + rel;
  }

  if (!title) {
    const h1Match = content.match(/^#\s+(.+)$/m);
    title = h1Match ? h1Match[1].trim() : canonical.split("/").pop() || "Documentation";
  }

  return { title, description, canonical };
}

function processMdxFile(filePath) {
  const rawContent = readFileSync(filePath, "utf-8");
  const { title, description, canonical } = extractMetadata(rawContent, filePath);
  const section = getSectionForRoute(canonical);

  // Strip metadata export and imports
  let body = rawContent
    .replace(/export\s+const\s+metadata\s*=\s*{[\s\S]*?};/, "")
    .replace(/^import\s+.*$/gm, "")
    .replace(/^export\s+.*$/gm, "");

  // Split into sections by H2 (##) or H3 (###)
  const lines = body.split("\n");
  const documents = [];

  let currentHeading = "";
  let currentSlug = "";
  let currentLines = [];

  function flushSection() {
    const rawSectionText = currentLines.join("\n").trim();
    if (!rawSectionText && !currentHeading) return;

    const cleanedText = cleanMarkdown(rawSectionText);
    if (!cleanedText && !currentHeading) return;

    if (currentHeading) {
      documents.push({
        id: `${canonical}#${currentSlug}`,
        title: `${title} › ${currentHeading}`,
        pageTitle: title,
        href: `${canonical}#${currentSlug}`,
        section,
        description: currentHeading,
        content: cleanedText,
      });
    } else {
      // Intro section of page
      documents.push({
        id: canonical,
        title,
        pageTitle: title,
        href: canonical,
        section,
        description: description || title,
        content: cleanedText,
      });
    }
    currentLines = [];
  }

  for (const line of lines) {
    const headingMatch = line.match(/^(#{2,3})\s+(.+)$/);
    if (headingMatch) {
      flushSection();
      currentHeading = headingMatch[2].replace(/[#*`]/g, "").trim();
      currentSlug = slugify(currentHeading);
    } else {
      currentLines.push(line);
    }
  }
  flushSection();

  if (!documents.some((document) => document.href === canonical)) {
    documents.unshift({
      id: canonical,
      title,
      pageTitle: title,
      href: canonical,
      section,
      description: description || title,
      content: "",
    });
  }

  // If no sections were extracted, index the entire file content
  if (documents.length === 0) {
    documents.push({
      id: canonical,
      title,
      pageTitle: title,
      href: canonical,
      section,
      description: description || title,
      content: cleanMarkdown(body),
    });
  }

  return documents;
}

function main() {
  const files = findMdxFiles(DOCS_APP_ROOT);
  const allDocuments = [];

  for (const file of files) {
    const docs = processMdxFile(file);
    allDocuments.push(...docs);
  }

  writeFileSync(OUTPUT_FILE, JSON.stringify(allDocuments, null, 2), "utf-8");
  console.log(`Generated search index with ${allDocuments.length} searchable sections from ${files.length} pages.`);
}

main();
