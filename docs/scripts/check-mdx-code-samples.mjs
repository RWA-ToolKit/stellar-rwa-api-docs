#!/usr/bin/env node
/**
 * Extracts fenced code blocks from `.mdx` docs and verifies that they parse:
 * TypeScript / JavaScript samples are syntax-checked with the TypeScript
 * compiler, `json` samples with `JSON.parse`, `sh` / `bash` / `shell` samples
 * with `sh -n` / `bash -n`, `python` samples with `ast.parse`, and `go`
 * samples with `gofmt -e`.
 *
 * This is a static check, not an execution check: it does not run any
 * sample's side effects (network calls, filesystem access, etc). It also does
 * not cover `rust` or other languages (Rust snippets are covered separately by
 * `api/tests/*_examples.rs`).
 *
 * Scope note: samples are documentation *fragments*. They reference values
 * introduced in the surrounding prose (`admin`, `userAddress`, a `compliance`
 * client) and import packages the docs site does not itself depend on. Full
 * type resolution would therefore fail on correct, readable samples, and the
 * only way to satisfy it would be padding every block with boilerplate. So
 * this checks syntax — unbalanced braces, bad generics, stray tokens — which
 * is what the docstring has always promised and what actually catches broken
 * samples. Shell samples are syntax-checked only; variables such as `${API_KEY}`
 * are not resolved and commands are never run.
 *
 * Errors are reported as `file:line - [lang] message`, with the line in the
 * MDX file, sorted by file and line.
 *
 * Usage: node scripts/check-mdx-code-samples.mjs
 */
import { readFileSync, writeFileSync, mkdtempSync, rmSync, readdirSync, statSync } from "node:fs";
import { join, extname, relative } from "node:path";
import { tmpdir } from "node:os";
import { spawnSync } from "node:child_process";
import ts from "typescript";

const DOCS_ROOT = join(process.cwd(), "app");
const TS_LANGS = new Set(["ts", "typescript", "tsx"]);
const JS_LANGS = new Set(["js", "javascript", "jsx"]);
const JSON_LANGS = new Set(["json"]);
const SHELL_LANGS = new Set(["sh", "bash", "shell"]);
const PY_LANGS = new Set(["py", "python"]);
const GO_LANGS = new Set(["go", "golang"]);
const CHECKED_LANGS = new Set([
  ...TS_LANGS,
  ...JS_LANGS,
  ...JSON_LANGS,
  ...SHELL_LANGS,
  ...PY_LANGS,
  ...GO_LANGS,
]);

// Parses the file given as argv[1]; prints "<lineno>\t<msg>" on a syntax error.
const PY_SYNTAX_CHECK = `import ast, sys
try:
    ast.parse(open(sys.argv[1], encoding="utf-8").read(), sys.argv[1])
except SyntaxError as err:
    print(f"{err.lineno or 1}\\t{err.msg}")
    sys.exit(1)
`;

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
    while ((match = FENCE_RE.exec(content)) !== null) {
      const [, rawLang, code] = match;
      const lang = rawLang.toLowerCase();
      if (!CHECKED_LANGS.has(lang)) continue;
      // 1-indexed line of the opening fence in the MDX file.
      const fenceLine = content.slice(0, match.index).split("\n").length;
      samples.push({ file, lang, code, fenceLine });
    }
  }
  return samples;
}

function extensionFor(lang) {
  if (lang === "tsx" || lang === "jsx") return lang;
  if (JSON_LANGS.has(lang)) return "json";
  if (SHELL_LANGS.has(lang)) return "sh";
  return TS_LANGS.has(lang) ? "ts" : "js";
}

// Each checker returns null or { offset, message }, where offset is the line
// within the block (0 = the fence line itself).
function checkJson(code) {
  try {
    JSON.parse(code);
    return null;
  } catch (err) {
    let message = err.message;
    if (/,\s*[}\]]/.test(code)) message += " (trailing commas are not allowed in JSON)";
    if (/^\s*\/[/*]/m.test(code)) message += " (JSON does not support comments)";
    return { offset: 0, message };
  }
}

function checkShell(lang, path) {
  const shell = lang === "bash" ? "bash" : "sh";
  const result = spawnSync(shell, ["-n", path], { encoding: "utf8" });
  if (result.error) throw result.error;
  if (result.status === 0) return null;
  const match = result.stderr.match(/: line (\d+): (.*)/);
  return match
    ? { offset: Number(match[1]), message: match[2].trim() }
    : { offset: 0, message: result.stderr.trim() };
}

function checkPython(path) {
  const result = spawnSync("python3", ["-I", "-c", PY_SYNTAX_CHECK, path], { encoding: "utf8" });
  if (result.error) throw result.error;
  if (result.status === 0) return null;
  const [line, ...msg] = result.stdout.trim().split("\t");
  return { offset: Number(line) || 0, message: msg.join("\t") || result.stderr.trim() };
}

// shortcut: snippets with a package clause are parsed as-is; without one they
// are wrapped (top-level declarations) or placed in a func body (statements),
// so mixing imports with loose statements in one block will be reported as an
// error. Upgrade to a real Go parser if that pattern shows up in the docs.
function checkGo(code, path) {
  let source = code;
  let shift = 0;
  if (!/^\s*package\s/m.test(code)) {
    const hasDecls = /^(func|type|var|const|import)\b/m.test(code);
    source = hasDecls ? `package sample\n\n${code}` : `package sample\n\nfunc _() {\n${code}\n}\n`;
    shift = hasDecls ? 2 : 3;
  }
  writeFileSync(path, source);
  const result = spawnSync("gofmt", ["-e", path], { encoding: "utf8" });
  if (result.error) throw result.error;
  if (result.status === 0) return null;
  const match = result.stderr.match(/:(\d+):\d+: (.*)/);
  return match
    ? { offset: Math.max(Number(match[1]) - shift, 0), message: match[2].trim() }
    : { offset: 0, message: result.stderr.trim() };
}

function checkTypeScriptSamples(samples, tmpDir) {
  const entries = samples.map((sample, i) => {
    const path = join(tmpDir, `sample-${i}.${extensionFor(sample.lang)}`);
    writeFileSync(path, sample.code);
    return { path, sample };
  });

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

  const errors = [];
  for (const { path, sample } of entries) {
    const sourceFile = program.getSourceFile(path);
    for (const diagnostic of program.getSyntacticDiagnostics(sourceFile)) {
      const message = ts.flattenDiagnosticMessageText(diagnostic.messageText, " ");
      const { line } = sourceFile.getLineAndCharacterOfPosition(diagnostic.start ?? 0);
      errors.push({ sample, offset: line + 1, message });
    }
  }
  return errors;
}

function main() {
  const mdxFiles = findMdxFiles(DOCS_ROOT);
  const samples = extractSamples(mdxFiles);

  if (samples.length === 0) {
    console.log("No checked MDX code samples found.");
    return;
  }

  console.log(`Checking ${samples.length} code sample(s) from ${mdxFiles.length} MDX file(s)...`);

  const tmpDir = mkdtempSync(join(tmpdir(), "mdx-samples-"));
  try {
    const tsSamples = samples.filter((s) => TS_LANGS.has(s.lang) || JS_LANGS.has(s.lang));
    const errors = [];

    samples.forEach((sample, i) => {
      if (!sample.code.trim()) {
        errors.push({ sample, offset: 0, message: "code block is empty" });
      } else if (JSON_LANGS.has(sample.lang)) {
        const problem = checkJson(sample.code);
        if (problem) errors.push({ sample, ...problem });
      } else if (SHELL_LANGS.has(sample.lang)) {
        const path = join(tmpDir, `shell-${i}.sh`);
        writeFileSync(path, sample.code);
        const problem = checkShell(sample.lang, path);
        if (problem) errors.push({ sample, ...problem });
      } else if (PY_LANGS.has(sample.lang)) {
        const path = join(tmpDir, `python-${i}.py`);
        writeFileSync(path, sample.code);
        const problem = checkPython(path);
        if (problem) errors.push({ sample, ...problem });
      } else if (GO_LANGS.has(sample.lang)) {
        const path = join(tmpDir, `go-${i}.go`);
        const problem = checkGo(sample.code, path);
        if (problem) errors.push({ sample, ...problem });
      }
    });

    errors.push(...checkTypeScriptSamples(tsSamples.filter((s) => s.code.trim()), tmpDir));

    errors.sort(
      (a, b) =>
        a.sample.file.localeCompare(b.sample.file) ||
        a.sample.fenceLine + a.offset - (b.sample.fenceLine + b.offset),
    );

    for (const { sample, offset, message } of errors) {
      const line = sample.fenceLine + offset;
      console.error(`${relative(process.cwd(), sample.file)}:${line} - [${sample.lang}] ${message}`);
    }

    if (errors.length > 0) {
      console.error(`\n${errors.length} syntax error(s) across MDX code samples.`);
      process.exitCode = 1;
    } else {
      console.log("All MDX code samples parsed successfully.");
    }
  } finally {
    rmSync(tmpDir, { recursive: true, force: true });
  }
}

main();
