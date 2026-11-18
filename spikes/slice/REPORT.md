# S1: the vertical slice

Phase 1 spike S1 ([#12](https://github.com/SantaClaas/vile/issues/12), [Phase 1](../../docs/PLAN.md#phase-1-spikes-two-subagents-in-parallel) in the plan): the smallest run from Oxc to Vitest, with text patches and no schemata. Phase 2 can take from it or drop it.

## What it is

- **[`src/`](src/)**, the Rust crate `vile_spike_slice`: Oxc parses each source file, a visitor finds the sites, and the binary writes `vile_core` mutants to `mutants.json` with `vile_core::output::MutantsFile`.
  - **Relational:** the sufficient set for `<`. `<=` and `!==` patch the token; `false` patches the whole comparison.
  - **Body replacement** from the written return type only, without tsrs: `boolean` and type predicates give `true` and `false`; `number` gives `0`, `1` and `-1`; `string` gives `""` and `"xyzzy"`; `T[]`, `readonly T[]` and `Array<T>` give `[]`; `void` gives `{}`; a union gives each member's values, `undefined` first; an `async` function's `Promise<T>` gives the values of `T`. Aliases, brands, object types and inferred return types give nothing.
- **[`runner/`](runner/)**, the TypeScript runner on Vitest's Node API: it runs the baseline, then every mutant in the same process, and writes `outcomes.json` in `vile_core`'s `OutcomesFile` shape. [`runner/compare.ts`](runner/compare.ts) compares a fixture's outcomes with its `expected.json`.

```sh
cargo run --release -p vile_spike_slice -- fixtures/body-replacement --out /tmp/out
node spikes/slice/runner/run.ts fixtures/body-replacement /tmp/out/mutants.json
node spikes/slice/runner/compare.ts fixtures/body-replacement /tmp/out
```

## Numbers

Measured on a 4-core Linux container, Node 26.13.1, one run at a time. The planner is `src/plan/` of the week budget planner at `d848926`, without `src/plan/test-data.ts`, the same 19 files as its Stryker config.

| Run | Mutants | Wall time | Killed | Survived | Timeout | Other |
| --- | --- | --- | --- | --- | --- | --- |
| Slice, `fixtures/operators` | 15 | 2.6 s | 13 | 2 | 0 | 0 |
| Slice, `fixtures/body-replacement` | 12 | 22.1 s (4.9 s with a 3 s timeout) | 10 | 1 | 1 | 0 |
| Slice, planner | 116 (24 relational, 92 body replacement) | 56.1 s, 58.4 s, 57.0 s | 109 | 7 | 0 | 0 |
| Slice, planner, `isolate: false` (experiment) | 116 | 34.1 s, 39.3 s | 109 | 7 | 0 | 0 |
| Stryker 10.1.1, planner, 4 processes | 982 | 107 s | 912 | 57 | 2 | 8 no coverage, 3 runtime errors |
| Stryker 10.1.1, planner, 1 process | 982 | 389 s | 912 | 57 | 2 | 8 no coverage, 3 runtime errors |

- **Generating** the planner's 116 mutants takes under 15 ms (release build), parsing included.
- **The slice's planner run:** a 6 s baseline, so a 30 s timeout; each mutant's tests took 293 ms at the median and 2.7 s at most, 49.5 s in all. Mutation score 0.94 (Stryker's: 0.934, over different mutants).
- **Per mutant,** the slice spends about 0.49 s of wall time in one process. Stryker spends 0.40 s in one process and 0.11 s with four, with its per-test coverage running 5.3 tests per mutant on average; the slice runs every test file that imports the mutated file.
- **Stryker's mutants are a superset in kind:** 17 mutators, 250 of its 982 mutants are `ConditionalExpression` and 129 `EqualityOperator`. The two tools agree wherever they make the same mutant: Stryker makes 13 of the slice's `<=` and `false` mutants at the same spans, and all 13 have the same outcome.
- **The planner's survivors** (all stable over 6 runs):
  - `!==` for `<` in `activity/look.ts:121`, `slot.ts:95` and `slot.ts:102`, and `<=` in `look.ts:121` and `slot.ts:95`. Stryker's `<=` survives at the same two places; `slot.ts:102`'s `!==` is a survivor only the sufficient set finds.
  - `{ return "xyzzy"; }` for the body at `activity/form.ts:32`; Stryker's `{}` for the same body survives too.
  - `{ return true; }` for `sanitize.ts:27`, the type guard `(value): value is Hue => PALETTE_HUES.has(value)`, which Stryker has no mutant for at that span.

## Outcomes against the fixtures

The slice makes exactly the fixtures' mutants for its operators (the Rust test `makes_the_fixtures_mutants_for_the_operators_it_covers` checks it), and every outcome matches `expected.json`:

- **`fixtures/operators`:** the three `<` mutants in `src/range.ts`: `<=` killed, `!==` survived, `false` killed. The fixture lists no body replacement, so the 12 body mutants the slice also makes there aren't compared (11 killed; `isVisible` returning `true` survives, since its only test expects `true`).
- **`fixtures/body-replacement`:** all 12 mutants in `value.ts` and `countdown.ts` match, the timeout in `countDown` included. The three `Cents` mutants (expected compile errors) aren't made: `Cents` is a brand, and without tsrs the written type doesn't say it's a number.

## Vitest's Node API

All of it is the public API of Vitest 5.1.0 (`vitest/node`), except where it says otherwise.

- **`createVitest(options, viteOverrides, vitestOptions)`** starts Vitest without running anything. The options are the CLI's: `root`, `watch: false`, `pool: "forks"`, `silent: true`, `reporters`, `fsModuleCache: false`, `configLoader: "native"`. `viteOverrides.plugins` adds a Vite plugin to the server (`vitest.projects[0].vite === vitest.vite` for a project without `projects`). `vitestOptions.stdout` and `stderr` take any `Writable`, which silences Vitest's logger.
- **`vitest.standalone()`** has to run before the first run, since it sets up the reporters. Without it a run crashes in `MinimalReporter.printLeaksSummary` (`this.ctx` is undefined).
- **`globTestSpecifications()`** lists the test files, and **`runTestSpecifications(specifications)`** runs some of them and returns a `TestRunResult`.
- **`TestRunResult.testModules` lists every module this Vitest has run, not only this run's.** A module from an earlier mutant keeps its failed state, so the outcome may only look at the modules the run selected. It was the slice's first wrong result: mutants were killed by another file's stale failure.
- **Outcomes:** `module.children.allTests("failed")` gives the failed `TestCase`s, whose `fullName` goes in `failedTests`. `module.errors()` and `result.unhandledErrors` show a module that didn't load, which the slice records as a runtime error.
- **Choosing the tests:** the slice walks Vite's module graph (`project.vite.environments[*].moduleGraph.getModulesByFile(file)` and each module's `importers`) from the mutated file up to the test files, as Vitest's watcher does (`VitestWatcher.handleFileChanged`). The baseline fills the graph. A file no test imports would be "no coverage"; on the planner every file had tests. This works per file, not per test.

### Serving the patch

- **A Vite plugin** with `enforce: "pre"` and a `load` hook returns the patched source for the mutated file's id (query removed) while its mutant runs, and nothing otherwise. Vite's normal transform runs on it afterwards, so TypeScript is stripped as usual, and the user's files never change.
- **`vitest.invalidateFile(file)`** before and after each mutant drops the module from every environment's module graph and from Vitest's module cache, so the next run loads it again. The importers need no invalidating, since each test file runs in a fresh module runner (`isolate: true`, the default).
- **`fsModuleCache` has to be off.** It's on by default in Vitest 5 and keeps transformed modules in `node_modules/.vitest-cache`. With it on, the `load` hook never runs for a file the cache has, so every mutant survived. Worse, a patched module could be written to the user's cache and show up in their next ordinary test run.
- **The forks pool** writes each transformed module to a temporary file named after the module's id (`ModuleTransformService.fetchInSpan` with `makeTmpCopies`), and the worker reads it from there. Invalidation clears `transformResult`, so the next fetch writes the new text to the same path. That held in every run here, but it's Vitest's internals.
- **Patches are bytes.** `Patch::span` counts UTF-8 bytes, so the runner patches a `Buffer`, not a JavaScript string.

### Stopping at the first failure

**Vitest's `bail: 1` made the outcomes flaky.** Over four runs of the planner, six different mutants came out survived in one run and killed in the others. The worker implements `bail` by sending `onCancel("test-failure")` over IPC when a test fails, without waiting; that message can reach the main process after the run has ended and cancel the next mutant's run, whose tests are then skipped and look like survivors. Without `bail` the outcomes were stable, but the run took twice as long (108 to 130 s).

The slice stops runs itself instead: a reporter's `onTestCaseResult` calls `vitest.cancelCurrentRun("test-failure")` from the main process, and the runner waits for that cancel before the next mutant. Three runs with it gave the same outcomes as the three runs without `bail`, at the old speed.

### Timeouts and killing a worker

- **The limit** is the plan's: 5 times the baseline's wall time, at least 20 s (`--minimum-timeout` lowers the floor).
- **A synchronous endless loop** (the fixture's `isDone` returning `false`) blocks the worker's event loop, so nothing inside the worker can stop it and `testTimeout` never fires.
- **Two `cancelCurrentRun()` calls kill it.** The first asks the pool to cancel: it sends the worker a message and waits for it to stop, which a blocked worker never does. The second finds the pool already cancelling and forces it (`PoolRunner.stop({ force: true })`): `SIGTERM`, then `SIGKILL` 500 ms later. Then both calls and the run settle, and the next mutant runs on a new worker. No worker process was left behind. Forcing on the second call is how Vitest handles a double Ctrl-C (`Pool.cancel`), not a documented API.
- `CancelReason` is typed `"keyboard-input" | "test-failure" | (string & Record<string, never>)`, so a reason of vile's own doesn't type-check; the slice passes `"keyboard-input"`.

### Where the time goes

Transforming isn't the cost: Oxc finds every site in milliseconds, and Vite transforms one patched file per mutant. With `isolate: true` every run starts a new worker process per test file and imports the test file's whole module graph again, which in the planner's own run is half the time ("import 49%"). The `isolate: false` experiment, which keeps workers warm between runs, cut the planner from 57 s to 34 to 39 s with the same outcomes. That it stayed correct relies on Vitest invalidating modules in a reused worker, which two runs don't prove.

## Oxc

Oxc 0.158.0 (`oxc_allocator`, `oxc_parser`, `oxc_ast`, `oxc_ast_visit`, `oxc_span`, `oxc_syntax`), pinned exactly in the workspace.

- **Parsing** is `Parser::new(&allocator, source, SourceType::from_path(file)?).parse()`; `diagnostics.errors()` gives the syntax errors as `OxcDiagnostic`s, which print with `Display`.
- **The visitor** is `oxc_ast_visit::Visit`: the slice overrides `visit_binary_expression`, `visit_function` (whose `ScopeFlags` come from `oxc_syntax`, not re-exported), `visit_arrow_function_expression`, `visit_class`, `visit_method_definition` and `visit_variable_declarator`, and calls `walk::walk_*` to continue. The scope names for `MutantId` come from `Function::id`, `Class::id`, `PropertyKey::static_name()` and `BindingPattern::get_identifier_name()`.
- **Spans** are UTF-8 byte offsets in `u32`, like `vile_core::span::Span`, and `Span::source_text(source)` slices the text. Nodes have spans, but **operators don't**: a `BinaryExpression` has `left`, `operator` and `right`, so the slice looks for `<` in the text between the operands. A comment between them that contains the same character would mislead it. Phase 2 needs this for every operator; the parser's token list (a `ParserConfig` that keeps tokens) would give exact spans.
- **Return types** are a `TSTypeAnnotation` around the `TSType` enum, and matching on `TSBooleanKeyword`, `TSArrayType`, `TSUnionType`, `TSTypeReference` and the rest was direct. An arrow function's body is an `ArrowFunctionBody`, a block or an expression; a block (`{ return 0; }`) is valid syntax in place of either, so both get the same patch.
- **The cost:** about 60 crates (ICU for the diagnostics, `phf`), and between them `syn` 2 and `syn` 3, which fails clippy's `multiple_crate_versions`; the crate roots `#[expect]` it with the reason. cargo-deny accepted every license as `deny.toml` stands.

## What `vile_core` lacked

- **`Operator` isn't `Ord` or `Hash`**, so it can't be part of a map key; counting a mutant's occurrence in its scope keys on `Operator::name()` instead.
- **No `Site` type**, though the glossary has one. The slice keeps its own (scope, operator, span, replacement). With A's generator, `Site` may belong in `vile_core`, with the scope that `Identity` needs.
- **Nothing on the TypeScript side.** The runner reads `mutants.json` and writes `outcomes.json` by hand, mirroring `MutantsFile`, `Record`, `Summary` and the score in [`runner/mutant.ts`](runner/mutant.ts) and [`runner/outcome.ts`](runner/outcome.ts). `OutcomesFile::new` computes the summary in Rust, where the runner can't call it. The napi binding could expose it, or a JSON schema generated from `vile_core` could keep both sides in step.
- **The JSON doesn't say its offsets are bytes.** `Patch::span`'s documentation should, since every JavaScript reader of `mutants.json` has to patch bytes, not string indices.
- **A path dependency** on `vile_core` needs `version = "=0.0.0"`, or cargo-deny's `wildcards = "deny"` refuses it. `[workspace.dependencies]` could carry it once a second crate depends on it.

## Recommendation for Phase 2

- **Run schemata, as the plan says; keep text patches for everything else.** The slice shows why: text patches are simple and correct, but the patch belongs to the whole Vite server, so mutants run one at a time, each needs invalidation, and nothing stays warm. A schema transforms each file once, and a per-run switch lets warm workers (`isolate: false` already saved 40%) run different mutants in parallel, which is how Stryker gets to 0.11 s per mutant. Text patches stay what tsrs checks, the equivalence check compares, reports show and `--in-place` applies, and the slice's runner can be D's reference for "the schema gives the same outcome as the patched text".
- **Open question for E:** how a schema learns its mutant per run and per worker. `vitest.provide()` is global to a Vitest instance; per-run environment variables or one Vitest per worker group are the candidates to try.
- **A per-test reach map** next: the slice runs whole test files, Stryker 5.3 tests per mutant.
- **napi or a separate process makes no difference to speed:** generating takes milliseconds, and a process boundary with a JSON file costs nothing next to 0.2 to 0.5 s of tests per mutant. So the choice is about packaging and types. napi, as planned, gives one `pnpm add` and lets the runner call `vile_core` (the summary, the score) instead of mirroring it. Keep the boundary coarse, one call per stage with `vile_core`'s JSON shapes, so a CLI binary stays an easy fallback. The tests themselves stay in Vitest's forks pool: separate processes are what makes a synchronous loop killable.
- **Pin the Vitest behaviour vile relies on with tests in E**, since none of it is documented: the forced kill on a second cancel, `testModules` collecting every run, `fsModuleCache` skipping `load`, the forks pool's temporary copies, and `bail`'s late cancel.

## What the slice doesn't do

- **No type check**, so `Cents` gets no mutants instead of three compile errors, and nothing filters mutants that don't type-check. That's S2's.
- **No per-test reach map**, and one mutant at a time in one Vitest.
- **Only `<`** among the relational operators.
- **It runs vile's own Vitest** (5.1.0) even on the planner, which pins 5.0.4. That worked, but a tool should use the project's Vitest (see [A tool, not an app](../../docs/guidelines/principles.md#a-tool-not-an-app)).
- **Not tried:** a config with several Vitest `projects` (the plugin may not reach every project's server), browser mode, and `vi.mock` of a mutated module.
