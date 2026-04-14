/**
 * Post-processes `waypoints/app/bindings.g.ts` after tauri-specta codegen.
 *
 * tauri-specta inlines every type referenced by commands. Many of those types
 * are already exported by `@waypoints/domain`, `@waypoints/editor`, and
 * `@waypoints/cartograph`. This script replaces the inline definitions with
 * `import type` statements so the frontend shares one set of type identities
 * across all crates and the command bindings.
 *
 * Without this, TypeScript would see e.g. `bindings.g.ts::Territory` and
 * `@waypoints/cartograph::Territory` as structurally-compatible but nominally
 * distinct types, which breaks type narrowing, assignment, and generics.
 *
 * IMPORTANT: Run treefmt on bindings.g.ts *before* this script so the output
 * is in a predictable, semicolon-terminated format.
 */

import { readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";

const root = resolve(import.meta.dirname, "..");
const domainRoot = resolve(root, "crates/domain");
const editorRoot = resolve(root, "crates/editor");
const cartographRoot = resolve(root, "crates/cartograph");
const bindingsPath = resolve(root, "waypoints/app/bindings.g.ts");

// ── Helpers ──────────────────────────────────────────────────────────────────

function extractTypeNames(path: string): Set<string> {
  const src = readFileSync(path, "utf-8");
  const names = new Set<string>();
  for (const [, name] of src.matchAll(/^export type (\w+)/gm)) {
    names.add(name);
  }
  return names;
}

function genTypeImport(specifier: string, names: string[]): string {
  if (names.length === 0) return "";
  const sorted = [...names].sort();
  return `import type { ${sorted.join(", ")} } from "${specifier}";`;
}

function findJsdocStart(lines: string[], typeLine: number): number {
  if (typeLine === 0) return typeLine;
  let scan = typeLine - 1;
  while (scan >= 0 && lines[scan].trim() === "") scan--;
  if (scan >= 0 && lines[scan].trimEnd().endsWith("*/")) {
    while (scan >= 0 && !lines[scan].trimStart().startsWith("/**")) scan--;
    if (scan >= 0) return scan;
  }
  return typeLine;
}

/**
 * Split a bindings file into two regions:
 *   1. Everything BEFORE the first `export type` (commands, helpers, imports)
 *   2. The type declarations themselves
 *
 * This lets us surgically replace only the type section while preserving
 * command bindings, the tauri-specta runtime helpers, and the import block
 * at the bottom (TAURI_INVOKE, TAURI_CHANNEL, etc.).
 */
function splitBindings(source: string): {
  preamble: string;
  typeSection: string;
  postamble: string;
} {
  const lines = source.split("\n");

  // Find the marker comments that tauri-specta uses to delimit sections
  const typeStartMarker = "/** user-defined types **/";
  const typeEndIdx = lines.length;

  let typeStartIdx = lines.findIndex((l) => l.includes(typeStartMarker));
  if (typeStartIdx === -1) {
    // Fallback: first `export type` line
    typeStartIdx = lines.findIndex((l) => /^export type \w+/.test(l));
  } else {
    typeStartIdx++; // skip the marker line itself
  }

  if (typeStartIdx === -1) {
    return { preamble: source, typeSection: "", postamble: "" };
  }

  // Find where the type section ends — look for the tauri-specta globals marker
  const globalsMarker = "/** tauri-specta globals **/";
  let globalsIdx = lines.findIndex((l) => l.includes(globalsMarker));
  if (globalsIdx === -1) globalsIdx = lines.length;

  return {
    preamble: lines.slice(0, typeStartIdx).join("\n"),
    typeSection: lines.slice(typeStartIdx, globalsIdx).join("\n"),
    postamble: lines.slice(globalsIdx).join("\n"),
  };
}

function extractTypeDeclarations(source: string): Map<string, string> {
  const types = new Map<string, string>();
  const lines = source.split("\n");

  const declStarts: { name: string; line: number }[] = [];
  for (let i = 0; i < lines.length; i++) {
    const match = lines[i].match(/^export type (\w+)/);
    if (match) declStarts.push({ name: match[1], line: i });
  }

  for (let idx = 0; idx < declStarts.length; idx++) {
    const { name, line: typeLine } = declStarts[idx];
    const blockStart = findJsdocStart(lines, typeLine);

    let blockEnd: number;
    if (idx + 1 < declStarts.length) {
      blockEnd = findJsdocStart(lines, declStarts[idx + 1].line);
    } else {
      blockEnd = lines.length;
    }

    while (blockEnd > blockStart && lines[blockEnd - 1].trim() === "") blockEnd--;

    types.set(name, lines.slice(blockStart, blockEnd).join("\n"));
  }

  return types;
}

// ── Collect crate type names ─────────────────────────────────────────────────

const domainTypes = extractTypeNames(resolve(domainRoot, "typescript/types.g.ts"));
const editorTypes = extractTypeNames(resolve(editorRoot, "typescript/types.g.ts"));
const cartographTypes = extractTypeNames(resolve(cartographRoot, "typescript/types.g.ts"));

// ── Parse bindings ───────────────────────────────────────────────────────────

const file = readFileSync(bindingsPath, "utf-8");
const { preamble, typeSection, postamble } = splitBindings(file);

const allDecls = extractTypeDeclarations(typeSection);

const fromDomain: string[] = [];
const fromEditor: string[] = [];
const fromCartograph: string[] = [];
const localDefs: string[] = [];

for (const [name, decl] of allDecls) {
  if (domainTypes.has(name)) {
    fromDomain.push(name);
  } else if (editorTypes.has(name)) {
    fromEditor.push(name);
  } else if (cartographTypes.has(name)) {
    fromCartograph.push(name);
  } else {
    localDefs.push(decl);
  }
}

// ── Assemble ─────────────────────────────────────────────────────────────────

const imports = [
  genTypeImport("@waypoints/domain", fromDomain),
  genTypeImport("@waypoints/editor", fromEditor),
  genTypeImport("@waypoints/cartograph", fromCartograph),
]
  .filter(Boolean)
  .join("\n");

const patchedTypeSection = ["", imports, "", localDefs.join("\n\n"), ""].join("\n");

const patched = [preamble, patchedTypeSection, postamble].join("\n");

writeFileSync(bindingsPath, patched);

console.log(
  [
    `patched bindings.g.ts —`,
    `imported ${fromDomain.length} from domain,`,
    `${fromEditor.length} from editor,`,
    `${fromCartograph.length} from cartograph,`,
    `kept ${localDefs.length} local`,
  ].join(" "),
);
