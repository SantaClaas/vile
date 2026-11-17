import { expect, test } from "vitest";

import { totalPrice } from "./price.ts";

test("takes the discount off", () => {
  expect(totalPrice(4, 1, 1)).toBe(3);
});
