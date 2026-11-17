import { expect, test } from "vitest";

import { displayName, isVisible } from "./name.ts";

test("shows what isn't hidden", () => {
  expect(isVisible(false)).toBe(true);
});

test("prefers the nickname", () => {
  expect(displayName("Ada", "Augusta Ada King")).toBe("Ada");
});

test("falls back to the name", () => {
  expect(displayName(undefined, "Grace Hopper")).toBe("Grace Hopper");
});
