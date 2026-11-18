import { expect, test } from "vitest";

import { score, summarize, write } from "./outcome.ts";
import type { Outcome, Record } from "./outcome.ts";

const record = (outcome: Outcome): Record => ({
  id: outcome,
  outcome,
  durationMilliseconds: 1,
  failedTests: [],
});

test("scores like vile_core's Summary", () => {
  const outcomes: Outcome[] = ["killed", "timeout", "survived", "no-coverage"];
  const summary = summarize(outcomes.map((outcome) => record(outcome)));
  expect(summary.killed).toBe(1);
  expect(summary.noCoverage).toBe(1);
  expect(score(summary)).toBe(0.5);
  expect(score(summarize([record("runtime-error")]))).toBeNull();
});

test("writes outcomes.json in vile_core's shape", () => {
  const written: unknown = JSON.parse(write([record("killed")]));
  expect(written).toMatchObject({
    formatVersion: 1,
    records: [{ id: "killed", outcome: "killed", durationMilliseconds: 1, failedTests: [] }],
    summary: { killed: 1, compileError: 0 },
    score: 1,
  });
});
