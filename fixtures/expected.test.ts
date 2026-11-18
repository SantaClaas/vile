// Checks that each fixture's expected.json still fits its sources (see fixtures/README.md).

import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, test } from "vitest";

const FIXTURES = import.meta.dirname;

/** The names in vile_core's Operator and Outcome, as the JSON writes them. */
const OPERATORS = new Set([
  "body-replacement",
  "relational",
  "logical",
  "arithmetic",
  "unary",
  "block-removal",
]);
const OUTCOMES = new Set([
  "killed",
  "survived",
  "timeout",
  "compile-error",
  "runtime-error",
  "equivalent",
  "no-coverage",
  "ignored",
]);

type ExpectedMutant = {
  file: string;
  line: number;
  operator: string;
  original?: string;
  replacement: string;
  outcome: string;
};

type Expected = { operators: string[]; mutants: ExpectedMutant[] };

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function isExpectedMutant(value: unknown): value is ExpectedMutant {
  if (!isRecord(value)) return false;
  return (
    typeof value["file"] === "string" &&
    typeof value["line"] === "number" &&
    typeof value["operator"] === "string" &&
    (value["original"] === undefined || typeof value["original"] === "string") &&
    typeof value["replacement"] === "string" &&
    typeof value["outcome"] === "string"
  );
}

function readExpected(fixture: string): Expected {
  const parsed: unknown = JSON.parse(
    readFileSync(join(FIXTURES, fixture, "expected.json"), "utf8"),
  );
  if (!isRecord(parsed)) throw new Error(`${fixture}/expected.json isn't an object`);
  const { operators, mutants } = parsed;
  if (!Array.isArray(operators) || !operators.every((operator) => typeof operator === "string")) {
    throw new Error(`${fixture}/expected.json has no list of operators`);
  }
  if (!Array.isArray(mutants) || !mutants.every((mutant) => isExpectedMutant(mutant))) {
    throw new Error(`${fixture}/expected.json has a mutant that's missing a field`);
  }
  return { operators, mutants };
}

/** The source from the start of a line, counted from 1, to its end. */
function readFromLine(fixture: string, file: string, line: number): string {
  const lines = readFileSync(join(FIXTURES, fixture, file), "utf8").split("\n");
  return lines.slice(line - 1).join("\n");
}

/** The mutants that name the text they replace, which is every operator but body replacement. */
function listWithOriginal(mutants: ExpectedMutant[]): (ExpectedMutant & { original: string })[] {
  return mutants.flatMap(({ original, ...mutant }) =>
    original === undefined ? [] : [{ ...mutant, original }],
  );
}

/** What tells two mutants apart in expected.json. */
function readKey({ file, line, operator, original, replacement }: ExpectedMutant): string {
  return `${file}:${String(line)}:${operator}:${original ?? ""}:${replacement}`;
}

const fixtures = readdirSync(FIXTURES, { withFileTypes: true })
  .filter((entry) => entry.isDirectory())
  .map((entry) => entry.name);

describe.each(fixtures)("%s", (fixture) => {
  const expected = readExpected(fixture);

  test("names only operators and outcomes vile knows", () => {
    for (const operator of expected.operators) expect(OPERATORS).toContain(operator);
    for (const mutant of expected.mutants) {
      expect(expected.operators).toContain(mutant.operator);
      expect(OUTCOMES).toContain(mutant.outcome);
    }
  });

  test.each(listWithOriginal(expected.mutants))(
    "finds $original on line $line of $file",
    ({ file, line, original }) => {
      // A block may run past its first line, so the original's first line has to be on the line
      // named, and the whole original in the source from there.
      const fromLine = readFromLine(fixture, file, line);
      const [firstLine = ""] = fromLine.split("\n");
      const [originalFirstLine = ""] = original.split("\n");
      expect(firstLine).toContain(originalFirstLine);
      expect(fromLine).toContain(original);
    },
  );

  test("lists each mutant once", () => {
    const keys = expected.mutants.map((mutant) => readKey(mutant));
    expect(new Set(keys).size).toBe(keys.length);
  });
});
