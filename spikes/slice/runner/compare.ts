// Compares the outcomes of a fixture's run with its expected.json (see fixtures/README.md).
//
//   node spikes/slice/runner/compare.ts <fixture> <folder with mutants.json and outcomes.json>
//
// Prints one line per expected mutant: its outcome, or that the slice didn't make it, and then
// the mutants the slice made that the fixture doesn't list.

import { readFileSync } from "node:fs";
import { join } from "node:path";

import * as mutantFile from "./mutant.ts";

type Expected = { key: string; outcome: string };

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

/** What tells two mutants apart in expected.json; body replacement has no original. */
function readKey(
  file: string,
  line: number,
  operator: string,
  original: string,
  replacement: string,
): string {
  const shown = operator === "body-replacement" ? "" : original;
  return `${file}:${line.toString()} ${operator} ${JSON.stringify(shown)} -> ${JSON.stringify(replacement)}`;
}

function readExpected(text: string): Expected[] {
  const data: unknown = JSON.parse(text);
  if (!isRecord(data) || !Array.isArray(data["mutants"])) return [];
  return data["mutants"].flatMap((mutant: unknown) => {
    if (!isRecord(mutant)) return [];
    const { file, line, operator, original = "", replacement, outcome } = mutant;
    const isValid =
      typeof file === "string" &&
      typeof line === "number" &&
      typeof operator === "string" &&
      typeof original === "string" &&
      typeof replacement === "string" &&
      typeof outcome === "string";
    return isValid ? [{ key: readKey(file, line, operator, original, replacement), outcome }] : [];
  });
}

function readOutcomes(text: string): Map<string, string> {
  const data: unknown = JSON.parse(text);
  if (!isRecord(data) || !Array.isArray(data["records"])) return new Map();
  return new Map(
    data["records"].flatMap((record: unknown) => {
      if (!isRecord(record)) return [];
      const { id, outcome } = record;
      return typeof id === "string" && typeof outcome === "string" ? [[id, outcome] as const] : [];
    }),
  );
}

function compare(fixture: string, folder: string): string[] {
  const mutants = mutantFile.parse(readFileSync(join(folder, "mutants.json"), "utf8"));
  if (typeof mutants === "string") return [`mutants.json: ${mutants}`];
  const outcomes = readOutcomes(readFileSync(join(folder, "outcomes.json"), "utf8"));
  const expected = readExpected(readFileSync(join(fixture, "expected.json"), "utf8"));
  const made = new Map(
    mutants.map((mutant) => [
      readKey(mutant.file, mutant.line, mutant.operator, mutant.original, mutant.replacement),
      outcomes.get(mutant.id) ?? "not run",
    ]),
  );
  const listed = expected.map(({ key, outcome }) => {
    const actual = made.get(key);
    if (actual === undefined) return `not made   ${key} (expected ${outcome})`;
    return `${actual === outcome ? "match     " : "MISMATCH  "} ${key}: ${actual}, expected ${outcome}`;
  });
  const expectedKeys = new Set(expected.map(({ key }) => key));
  const extra = made
    .entries()
    .filter(([key]) => !expectedKeys.has(key))
    .map(([key, outcome]) => `not listed ${key}: ${outcome}`)
    .toArray();
  return [...listed, ...extra];
}

if (import.meta.main) {
  const [fixture, folder] = process.argv.slice(2);
  if (fixture === undefined || folder === undefined) {
    console.error("usage: node spikes/slice/runner/compare.ts <fixture> <folder>");
    process.exitCode = 2;
  }
  if (fixture !== undefined && folder !== undefined) {
    for (const line of compare(fixture, folder)) console.log(line);
  }
}
