#!/usr/bin/env node
/**
 * Extracts fenced code blocks from `.mdx` docs and verifies that TypeScript /
 * JavaScript samples parse.
 *
 * This is a static check, not an execution check: it does not run any
 * sample's side effects (network calls, filesystem access, etc). It also only
 * covers `ts`/`tsx`/`js`/`jsx` fences — `bash`, `json`, `rust`, and other
 * languages are not checked here (Rust snippets are covered separately by
 * `api/tests/*_examples.rs`).
 *
 * Scope note: samples are documentation *fragments*. They reference values
 * introduced in the surrounding prose (`admin`, `userAddress`, a `compliance`
 * client) and import packages the docs site does not itself depend on. Full
 * type resolution would therefore fail on correct, readable samples, and the
 * only way to satisfy it would be padding every block with boilerplate. So
 * this checks syntax — unbalanced braces, bad generics, stray tokens — which
 * is what the docstring has always promised and what actually catches broken
 * samples.
 *
 * Usage: node scripts/check-mdx-code-samples.mjs
 */
import { readFileSync, writeFileSync, mkdtempSync, rmSync, readdirSync, statSync } from "node:fs";
import { join, extname } from "node:path";
import { tmpdir } from "node:os";
import ts from "typescript";

const DOCS_ROOT = join(process.cwd(), "app");
const TS_LANGS = new Set(["ts", "typescript", "tsx"]);
const JS_LANGS = new Set(["js", "javascript", "jsx"]);
const CHECKED_LANGS = new Set([...TS_LANGS, ...JS_LANGS]);

const FENCE_RE = /```([a-zA-Z0-9]*)\n([\s\S]*?)```/g;

function findMdxFiles(dir) {
  const out = [];
  for (const entry of readdirSync(dir)) {
    const full = join(dir, entry);
    const stat = statSync(full);
    if (stat.isDirectory()) {
      out.push(...findMdxFiles(full));
    } else if (extname(entry) === ".mdx") {
      out.push(full);
    }
  }
  return out;
}

function extractSamples(mdxFiles) {
  const samples = [];
  for (const file of mdxFiles) {
    const content = readFileSync(file, "utf8");
    let match;
    let index = 0;
    while ((match = FENCE_RE.exec(content)) !== null) {
      const [, rawLang, code] = match;
      const lang = rawLang.toLowerCase();
      if (!CHECKED_LANGS.has(lang)) continue;
      if (!code.trim()) continue;
      index += 1;
      samples.push({ file, lang, code, index });
    }
  }
  return samples;
}

function extensionFor(lang) {
  if (lang === "tsx" || lang === "jsx") return lang;
  return TS_LANGS.has(lang) ? "ts" : "js";
}

function main() {
  const mdxFiles = findMdxFiles(DOCS_ROOT);
  const samples = extractSamples(mdxFiles);

  if (samples.length === 0) {
    console.log("No TS/JS MDX code samples found.");
    return;
  }

  const tmpDir = mkdtempSync(join(tmpdir(), "mdx-samples-"));
  const entries = samples.map((sample, i) => {
    const fileName = `sample-${i}.${extensionFor(sample.lang)}`;
    const path = join(tmpDir, fileName);
    writeFileSync(path, sample.code);
    return { path, source: `${sample.file}#block-${sample.index}` };
  });

  console.log(`Checking ${samples.length} TS/JS code sample(s) from ${mdxFiles.length} MDX file(s)...`);

  try {
    const program = ts.createProgram({
      rootNames: entries.map((e) => e.path),
      options: {
        target: ts.ScriptTarget.ES2020,
        jsx: ts.JsxEmit.ReactJSX,
        allowJs: true,
        noEmit: true,
        // Samples are fragments: don't try to resolve their imports.
        noResolve: true,
      },
    });

    let failed = 0;
    for (const { path, source } of entries) {
      const sourceFile = program.getSourceFile(path);
      const diagnostics = program.getSyntacticDiagnostics(sourceFile);
      for (const diagnostic of diagnostics) {
        failed += 1;
        const message = ts.flattenDiagnosticMessageText(diagnostic.messageText, " ");
        const { line, character } = sourceFile.getLineAndCharacterOfPosition(diagnostic.start ?? 0);
        console.error(`${source}:${line + 1}:${character + 1} - ${message}`);
      }
    }

    if (failed > 0) {
      console.error(`\n${failed} syntax error(s) across MDX code samples.`);
      process.exitCode = 1;
    } else {
      console.log("All MDX code samples parsed successfully.");
    }
  } finally {
    rmSync(tmpDir, { recursive: true, force: true });
  }
}

main();
