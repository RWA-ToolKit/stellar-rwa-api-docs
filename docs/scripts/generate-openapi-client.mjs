import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const rootDir = path.resolve(__dirname, '..');
const openapiPath = path.join(rootDir, 'public/openapi.json');
const outputPath = path.join(rootDir, 'lib/api-client.ts');

const spec = JSON.parse(fs.readFileSync(openapiPath, 'utf8'));
const schemas = spec.components?.schemas ?? {};
const names = ['Asset', 'Holder', 'ComplianceSummary', 'JurisdictionCount', 'Distribution', 'Stats', 'Error'];

const toTsType = (schema, seen = new Set()) => {
  if (!schema) return 'unknown';
  if (schema.$ref) {
    const ref = schema.$ref.split('/').pop();
    if (ref && !seen.has(ref)) return ref;
    return 'unknown';
  }
  if (schema.enum) {
    return schema.enum.map((value) => JSON.stringify(String(value))).join(' | ');
  }
  if (schema.type === 'array') {
    return `${toTsType(schema.items, seen)}[]`;
  }
  if (schema.type === 'object' || schema.properties) {
    return 'Record<string, unknown>';
  }
  if (schema.type === 'boolean') return 'boolean';
  if (schema.type === 'integer' || schema.type === 'number') return 'number';
  if (schema.type === 'string') return 'string';
  return 'unknown';
};

const lines = [
  'export type AssetSortField = "valuation" | "holders" | "created_at";',
  'export type SortDirection = "asc" | "desc";',
  '',
];

for (const name of names) {
  const schema = schemas[name];
  if (!schema || !schema.properties) continue;

  const required = new Set(schema.required ?? []);
  lines.push(`export type ${name} = {`);
  for (const [key, value] of Object.entries(schema.properties)) {
    const requiredSuffix = required.has(key) ? '' : '?';
    const type = toTsType(value);
    lines.push(`  ${key}${requiredSuffix}: ${type};`);
  }
  lines.push('};');
  lines.push('');
}

fs.writeFileSync(outputPath, `${lines.join('\n')}\n`, 'utf8');
console.log(`Generated ${path.relative(rootDir, outputPath)}`);
