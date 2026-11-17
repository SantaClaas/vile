import { expect, test } from "vitest";

import { isInRange } from "./range.ts";

test("accepts a value inside the range", () => {
  expect(isInRange(5, 0, 10)).toBe(true);
});

test("accepts the minimum", () => {
  expect(isInRange(0, 0, 10)).toBe(true);
});

test("refuses the maximum", () => {
  expect(isInRange(10, 0, 10)).toBe(false);
});

test("refuses a value below the minimum", () => {
  expect(isInRange(-1, 0, 10)).toBe(false);
});
