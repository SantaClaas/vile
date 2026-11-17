import { expect, test } from "vitest";

import { clamp } from "./clamp.ts";

test("limits a value above the maximum", () => {
  expect(clamp(12, 10)).toBe(10);
});

test("keeps a value below the maximum", () => {
  expect(clamp(3, 10)).toBe(3);
});
