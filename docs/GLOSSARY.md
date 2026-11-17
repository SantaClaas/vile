# Glossary

Use one name per thing, the same way in the CLI's output, the docs and the code.

**Wrong names (look-up only)** lists words people use for a term, often from other tools or from the research. They aren't synonyms: when someone uses one, read it as the term, and answer, write and code with the term alone. Where the research has its own word, the Notes say so, because the docs cite papers that use it.

| Term | Meaning | Wrong names (look-up only) | Notes |
| --- | --- | --- | --- |
| **Mutant** | One small change to the code under test, such as `<` turned into `<=`, which the tests should notice. | mutation (for the changed code), variant, fault | Code: `Mutant`. "Mutation" is the act of changing; the result is a mutant |
| **Operator** | A kind of change that makes mutants: relational, logical, arithmetic, unary, block removal, body replacement. | mutator, mutation type, rule, genre | Code: `Operator`. The research says "mutation operator"; cargo-mutants says "genre"; Stryker says "mutator" |
| **Site** | A place in the code where an operator applies. One site can give several mutants. | location, point, target | Code: `Site` |
| **Patch** | The text a mutant replaces: a span of the original and what goes there instead. What reports show as a diff. | edit, diff, replacement | Code: `Patch` |
| **Schema** | All of a file's mutants in one module, each behind a switch, so tests run against any of them without transforming the file again. | instrumented file, switched code | Plural: schemata. The research says "mutant schemata"; Stryker says "mutation switching" |
| **Baseline** | The run of the tests against the code before any mutant, which must pass. It sets the timeout and the reach map. | dry run, initial run, clean run | |
| **Reach map** | Which tests run which sites, recorded during the baseline, so each mutant only runs the tests that reach it. | coverage, per-test coverage, coverage analysis | Code: `ReachMap` |
| **Outcome** | What happened to a mutant: one of caught, missed, timeout, unviable, equivalent, not reached, skipped. | status, result, verdict | Code: `Outcome` |
| **Caught** | A test failed with the mutant in place. | killed, detected | The research and Stryker say "killed" |
| **Missed** | Every test that reaches the mutant passed: the code could be wrong in this way and no test would notice. | survived, survivor, live, alive, uncaught | The research says "live" or "survived". A missed mutant is "a survivor" in the research too; use "missed mutant" |
| **Timeout** | The tests ran past the time limit with the mutant in place and were stopped. Often a loop that never ends. | hang, hung, timed out | |
| **Unviable** | The mutant doesn't type-check, so its tests never run. | invalid, compile error, stillborn | cargo-mutants' word. Stryker says "compile error" |
| **Equivalent** | The mutant can't change what the program does, so no test could catch it. vile finds some of them by comparing minified code. | redundant (for this meaning), no-op | "Equivalent mutant" is the research's term |
| **Duplicate** | Two mutants that change the program in the same way. One of them runs. | redundant, same mutant | |
| **Not reached** | No test runs the mutant's site. It's untested code rather than a weak test. | no coverage, uncovered | Stryker says "no coverage" |
| **Skipped** | A disable comment or an arid rule kept the site from being mutated. | ignored, excluded, filtered | |
| **Arid** | Code whose mutants aren't worth a test, such as logging, so it's never mutated. | uninteresting, noise, trivial | From Google's "arid nodes" (Petrović et al.) |
| **Productive** | A mutant worth seeing: a test for it would improve the tests, or it shows a bug. | useful, interesting, valuable | Google's term. Its opposite is unproductive |
| **Mutation score** | Caught and timed-out mutants as a share of those that were caught, timed out, missed or not reached. Unviable, equivalent and skipped mutants don't count. | kill rate, mutation coverage, score | The same definition as Stryker's. It isn't comparable between tools or projects |
