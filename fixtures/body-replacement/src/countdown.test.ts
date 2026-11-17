import { expect, test } from "vitest";

import { countDown } from "./countdown.ts";

test("counts down to zero", () => {
  expect(countDown(3)).toBe(3);
});
