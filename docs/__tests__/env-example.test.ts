import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { expect, it } from "vitest";

it("documents each environment variable read by docs/lib", () => {
  const lib = join(__dirname, "../lib");
  const example = readFileSync(join(__dirname, "../.env.example"), "utf8");
  for (const file of readdirSync(lib).filter(name => name.endsWith(".ts"))) {
    const source = readFileSync(join(lib, file), "utf8");
    for (const [, variable] of source.matchAll(/\bprocess\.env\.(\w+)/g)) {
      expect(example).toMatch(new RegExp(`^# .+\n(?:# )?${variable}=`, "m"));
    }
  }
});
