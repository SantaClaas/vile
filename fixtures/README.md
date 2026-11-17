# Fixtures

Small TypeScript projects with Vitest tests, written to give known outcomes. They are vile's acceptance tests: every stage, from generating mutants to the report, is tested against them (see [`docs/guidelines/tests.md`](../docs/guidelines/tests.md)).

| Fixture | Shows |
| --- | --- |
| [`operators/`](operators/) | Relational, logical, arithmetic, unary and block removal mutants: caught, missed (a boundary no test checks, an operation the tests can't tell apart, and one equivalent only by reasoning), and unviable (dropping `?? name` breaks the return type) |
| [`body-replacement/`](body-replacement/) | Function bodies replaced with values of their return type: caught, missed (a side effect no test looks at), timeout (a loop guard that never ends the loop) and unviable (a branded return type) |
| [`filtering/`](filtering/) | Mutants filtered before any test runs: arid (a console call's arguments), equivalent (in a block the minifier removes), skipped (a disable comment) and not reached (a function no test calls) |

## `expected.json`

Each fixture lists the operators it runs with and every mutant vile should make with them, no more and no fewer, with the outcome each one should have:

```json
{
  "operators": ["relational"],
  "mutants": [
    { "file": "src/range.ts", "line": 4, "operator": "relational", "original": "<", "replacement": "!==", "outcome": "missed", "note": "no test has a value above the maximum" }
  ]
}
```

- **`file`** is the path from the fixture's folder, and **`line`** the line the patch starts on, counted from 1.
- **`original`** is the text the patch replaces. Body replacement leaves it out, since it's the whole body.
- **`replacement`** is the text that goes in its place.
- **`outcome`** is one of the outcomes in the glossary ([`docs/GLOSSARY.md`](../docs/GLOSSARY.md)), as the JSON writes it.
- **`note`** says why, where it isn't obvious.

A mutant is matched by its file, line, operator, original and replacement. Its ID isn't listed, so the fixtures don't depend on how IDs are made.

## How mutants are written

These are the conventions the generator follows, which the expectations rely on:

- **An operator that swaps a token patches only the token:** `<` becomes `<=`, and `&&` becomes `||`.
- **Comparisons use strict equality:** the relational sufficient sets use `===` and `!==`, never `==` and `!=`, which convert their operands first.
- **A replacement of a whole expression patches the whole expression:** `value < maximum` becomes `false`, and `nickname ?? name` becomes `nickname`.
- **Removing a unary operator patches it with nothing:** `!isHidden` becomes `isHidden`.
- **Block removal patches the block, braces included, with `{}`.**
- **Body replacement patches the function's body, braces included,** with `{ return <value>; }`, or `{}` for a function that returns nothing.

## Checking the expectations

[`expected.test.ts`](expected.test.ts) checks that each `original` is on the line it names and that every operator and outcome is one vile knows, so an edit to a fixture can't leave its expectations behind. The fixtures' own tests run with `pnpm test`, and their type check with `pnpm typecheck`: they must pass unmutated, as vile's baseline needs.

The outcomes of caught, missed, timeout and unviable mutants were checked by hand when the fixtures were written, by applying each patch and running the tests or the type check. Skipped, equivalent and not reached depend on vile's own filtering, so the stages that do it check those.
