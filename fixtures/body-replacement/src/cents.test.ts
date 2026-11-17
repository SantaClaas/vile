import { expect, test } from "vitest";

import { toCents } from "./cents.ts";

test("rounds to whole cents", () => {
  expect(toCents(1.234)).toBe(123);
});
