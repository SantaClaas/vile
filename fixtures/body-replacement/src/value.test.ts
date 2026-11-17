import { expect, test } from "vitest";

import { evens, findName, greet, record } from "./value.ts";

test("greets by name", () => {
  expect(greet("Ada")).toBe("Hello, Ada");
});

test("keeps the even values", () => {
  expect(evens([1, 2, 3, 4])).toEqual([2, 4]);
});

test("finds a name by its start", () => {
  expect(findName(["Ada", "Grace"], "G")).toBe("Grace");
});

test("records without failing", () => {
  record([], "started");
});
