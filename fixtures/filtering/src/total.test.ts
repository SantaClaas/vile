import { expect, test } from "vitest";

import { isAdult, total } from "./total.ts";

test("adds", () => {
  expect(total(2, 3)).toBe(5);
});

test("knows an adult", () => {
  expect(isAdult(20)).toBe(true);
});
