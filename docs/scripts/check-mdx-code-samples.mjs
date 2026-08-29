#!/usr/bin/env node
/**
 * Extracts fenced code blocks from `.mdx` docs and verifies that TypeScript /
 * JavaScript samples at least parse and typecheck.
 *
 * This is a static check, not an execution check: it does not run any
 * sample's side effects (network calls, filesystem access, etc). It also only
 * covers `ts`/`tsx`/`js`/`jsx` fences — `bash`, `json`, `rust`, and other
 * languages are not typechecked here (Rust snippets are covered separately by
 * `api/tests/*_examples.rs`). A block that typechecks is not guaranteed to be
 * runnable in production; it only guarantees the sample is syntactically and
 * structurally valid TypeScript/JavaScript.
 *
 * Usage: node scripts/check-mdx-code-samples.mjs
 */
import { readFileSync, writeFileSync, mkdtempSync, rmSync, readdirSync, statSync } from "node:fs";
import { join, extname } from "node:path";
import { tmpdir } from "node:os";
import { execFileSync } from "node:child_process";

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

function main() {
  const mdxFiles = findMdxFiles(DOCS_ROOT);
  const samples = extractSamples(mdxFiles);

  if (samples.length === 0) {
    console.log("No TS/JS MDX code samples found.");
    return;
  }

  const tmpDir = mkdtempSync(join(tmpdir(), "mdx-samples-"));
  const tsconfig = {
    compilerOptions: {
      target: "ES2020",
      module: "ESNext",
      moduleResolution: "Bundler",
      jsx: "react-jsx",
      strict: false,
      noEmit: true,
      skipLibCheck: true,
      esModuleInterop: true,
      allowJs: true,
      isolatedModules: true,
    },
    include: ["*.ts", "*.tsx", "*.js", "*.jsx"],
  };
  writeFileSync(join(tmpDir, "tsconfig.json"), JSON.stringify(tsconfig, null, 2));

  const fileList = [];
  samples.forEach((sample, i) => {
    const ext = sample.lang === "tsx" || sample.lang === "jsx" ? sample.lang : sample.lang.startsWith("ts") ? "ts" : "js";
    const fileName = `sample-${i}.${ext}`;
    writeFileSync(join(tmpDir, fileName), sample.code);
    fileList.push({ fileName, source: `${sample.file}#block-${sample.index}` });
  });

  console.log(`Checking ${samples.length} TS/JS code sample(s) from ${mdxFiles.length} MDX file(s)...`);

  try {
    execFileSync("npx", ["--no-install", "tsc", "-p", tmpDir], {
      cwd: tmpDir,
      stdio: "inherit",
    });
    console.log("All MDX code samples parsed/typechecked successfully.");
  } catch (err) {
    console.error("\nOne or more MDX code samples failed to typecheck. Files checked:");
    fileList.forEach(({ fileName, source }) => console.error(`  ${fileName} <- ${source}`));
    process.exitCode = 1;
  } finally {
    rmSync(tmpDir, { recursive: true, force: true });
  }
}

main();
