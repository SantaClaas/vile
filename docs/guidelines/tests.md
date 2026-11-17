---
paths:
  - "**/*.test.ts"
  - "**/tests/**"
  - "fixtures/**"
---

# Tests

What gets a test and how the tests run. It adds to [`docs/guidelines/principles.md`](principles.md), which always applies; its "Where rules are enforced" says which stage catches what, and why an exception to the type checks owes a test.

**Test where a test adds something.** Coverage numbers are a metric, not the goal. Once a measure becomes a target, it stops being a good measure ([Goodhart's law](https://en.wikipedia.org/wiki/Goodhart%27s_law)): aim for coverage and you get tests written to touch lines, not to catch mistakes. A test is worth its code when it pins down a rule, a parser, an edge case or an exception's reason. The same holds for vile's own mutation score: a mutant worth catching gets a test, one that isn't gets a disable comment with its reason.

**Unit tests** sit next to the code they test: `*.test.ts` beside a TypeScript module, run by Vitest (`pnpm test`), and a `tests` module at the end of a Rust file, run by `cargo test` (see [`docs/guidelines/rust.md`](rust.md)). The pure core is most of vile, so most tests are input and expected output, with no setup.

**Fixture projects are the acceptance tests.** Each project in `fixtures/` is a small TypeScript project with Vitest tests, written to produce known outcomes: a mutant its tests kill, one that survives, one that doesn't type-check, one that's equivalent, one that hangs. Each lists the outcome it expects for every mutant. Every stage is tested against them, from generating mutants to the report, and a change that changes an outcome shows up as a diff in review.

- **A new operator, outcome or skip rule gets a fixture case** that shows it: the code it applies to, and a test that kills it or deliberately doesn't.
- **A fixture's code is written for the case it shows,** not taken from real projects, so it stays small enough to read in one go.

**Output is tested by snapshot.** The terminal output and the JSON in `vile.out/` are compared with a stored copy, so a change shows up as a diff in review. A change to the JSON is a change to the interface (see [A tool, not an app](principles.md#a-tool-not-an-app)).

**Run vile on real code before a release:** on the projects it was built for, starting with the [week budget planner](https://github.com/SantaClaas/week-budget-planner), and on vile's own TypeScript. Read the survivors and note which are useful and which are noise: that's how operators and arid rules get better (see the plan's [Noise control](../PLAN.md#noise-control)).

**vile tests itself with vile** once it can, the way cargo-mutants runs on its own code: the Vitest package first, the Rust crates once vile mutates Rust or through cargo-mutants until then.
