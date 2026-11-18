import { expect, test } from "vitest";

import { apply, parse } from "./mutant.ts";
import type { Mutant } from "./mutant.ts";

const mutant: Mutant = {
  id: "src/range.ts:isInRange:relational:a:b:0",
  file: "src/range.ts",
  operator: "relational",
  original: "<",
  start: 2,
  end: 3,
  replacement: "<=",
  line: 1,
};

const file = {
  formatVersion: 1,
  mutants: [
    {
      id: mutant.id,
      file: mutant.file,
      operator: mutant.operator,
      original: mutant.original,
      patch: { span: { start: 2, end: 3 }, replacement: "<=" },
      range: { start: { line: 1, column: 3 }, end: { line: 1, column: 4 } },
    },
  ],
};

test("reads the mutants Rust writes", () => {
  expect(parse(JSON.stringify(file))).toEqual([mutant]);
});

test("refuses what isn't a mutants file", () => {
  expect(parse("{")).toBe("invalid-file");
  expect(parse(JSON.stringify({ formatVersion: 1, mutants: [{ id: 1 }] }))).toBe("invalid-file");
  expect(parse(JSON.stringify({ ...file, formatVersion: 2 }))).toBe("newer-version");
});

test("patches bytes, not UTF-16 code units", () => {
  // "ä" takes two bytes, so the `<` after it starts at byte 4, though it's the fourth character.
  const source = Buffer.from("ä < b");
  expect(apply(source, { ...mutant, start: 3, end: 4 })).toBe("ä <= b");
});
