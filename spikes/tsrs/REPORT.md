# S2: tsrs as a Rust library

The S2 spike from [the plan](../../docs/PLAN.md#phase-1-spikes-two-subagents-in-parallel): can vile embed tsrs's crates to check mutants for compile errors and to find the return types body replacement needs? Issue #13.

## Verdict

**Embed the crates in process.** All four things #13 asks for work through tsrs's public Rust API, with no `unsafe` in vile's code:

- **Loading** a project from its `tsconfig.json` takes 23 to 92 ms.
- **Return types** at a byte offset come straight from the checker.
- **Checking** a patched file in memory, through tsrs's overlay file system, takes **1.4 to 2 ms per mutant** on the fixtures and **4 to 5 ms (median)** on the week budget planner's `src/plan/`. That's roughly 50 times faster than a fresh `tsrs` process per mutant (68 to 85 ms on the fixtures, 275 to 367 ms on the planner).
- **The acceptance check passes:** every compile-error mutant in `fixtures/operators` and `fixtures/body-replacement` gets a diagnostic, and every other mutant of those fixtures type-checks.

Two findings shape how `vile_types` should be built:

1. **Memory grows with every check**, about 1.4 MB per mutant on the planner (53 MB to 508 MB over 329 checks), because tsrs keeps every program version in its arena. A run of a few thousand mutants needs a plan for it; see [Memory](#memory).
2. **Diagnostics about unused declarations must be left out.** With `noUnusedLocals` or `noUnusedParameters`, nearly every body replacement leaves a parameter unused. Counting TS6133 would mark 128 of the planner's 129 body replacements as compile errors, though they run fine. tsrs marks those diagnostics with `reports_unnecessary()`, and the spike drops them.

## Publish-age exception

- **What:** the `tsrs_*` crates at 0.24.0 were taken before their week was up, approved by the maintainer. All 25 tsrs crates were resolved once with `CARGO_RESOLVER_INCOMPATIBLE_PUBLISH_AGE=allow`, in a commit of its own that records the approval.
- **When it ends:** on 2026-11-18 at 12:12 UTC, once `tsrs_project`, the last of them, is a week old. After that nothing stays to remove. No config changed.
- **Pinned back:** the same run picked two other crates under a week old, errno 0.3.15 and xxhash-rust 0.8.21. They're pinned back to 0.3.14 and 0.8.19 without the override.

## What was built

`spikes/tsrs/` is the crate `vile_spike_tsrs`:

| File or function | What it does |
| --- | --- |
| `Project::load(config_file)` | Parses the config and loads, parses and binds the program |
| `Project::find_function(file, offset)` | The innermost function whose body contains the offset: its body's span (braces included) and its return type as TypeScript prints it |
| `Project::functions(file)` | The same for every function with a body in the file |
| `Project::check(file, &Patch)` | Applies a `vile_core` patch in memory and returns the file's diagnostics, as `vile_core` spans in the patched text |
| `tests/fixtures.rs` | The acceptance check |
| `src/bin/measure.rs` | The numbers below: `cargo run --release -p vile_spike_tsrs --bin measure -- <tsconfig.json> <path part>` |

## Acceptance

Each patch is applied in memory as [`fixtures/README.md`](../../fixtures/README.md) describes it: an operator patches its token or expression, and body replacement patches the body that `find_function` finds on the mutant's line.

| Fixture | Compile errors (expected and found) | Type-checks (expected and found) |
| --- | --- | --- |
| `operators` | 1: `src/name.ts:8`, `nickname ?? name` → `nickname` gives TS2322 | 14 |
| `body-replacement` | 3: `src/cents.ts:4`, `return 0`, `1` and `-1` each give TS2322 (`number` isn't `Cents`) | 12 |

The other tests check four more things:

- **Unpatched files:** every file of both fixtures checks without diagnostics, and tsrs reuses the program for it.
- **Diagnostic span:** the diagnostic's span cuts out exactly `return` from the patched text. So tsrs's positions are UTF-8 byte offsets, as `vile_core` uses them.
- **Return types:** `find_function` gives the return types body replacement needs (`string`, `number[]`, `string | undefined`, `void`, `boolean`, `number`, `Cents`) and the exact span of each body.
- **Unused declarations:** a project with `noUnusedParameters` gets no diagnostic for `{ return true; }` and still gets TS2322 for `{ return 0; }`.

## Numbers

**Machine:** a shared cloud container with 4 vCPUs, so wall times vary; ranges are over three runs. **Build:** the release profile, Cargo's defaults (no LTO). **Mutants:** the measuring tool makes two kinds:

- **Relational swaps:** each relational operator with a space on each side, outside comments, gets two token swaps.
- **Body replacement:** each function whose return type is `void`, `boolean`, `number`, `string`, an array or `… | undefined` gets the plan's values.

Each check covers the patched file only.

| | Fixture `operators` | Fixture `body-replacement` | Planner `src/plan/` |
| --- | --- | --- | --- |
| Project | 8 files and the libs | 6 files and the libs | `tsconfig.app.json`: 95 files of `src/`, 508 with libraries |
| Mutants | 18 | 18 | 329: 200 relational, 129 body replacement |
| Load: parse and bind | 28–35 ms | 23–37 ms | 38–92 ms |
| Return types of every function in the mutated files | 2 ms | 2 ms | 13–17 ms |
| First check | 1.5–5.5 ms | 2.0–2.2 ms | 3.4–6.9 ms |
| Later checks: median | 1.4–2.0 ms | 1.7–2.0 ms | 4.0–5.1 ms |
| Later checks: 90th percentile | 1.8–4.7 ms | 2.3–6.0 ms | 5.2–8.4 ms |
| Later checks: slowest | 2.0–5.6 ms | 3.3–21 ms | 7.6–22 ms |
| All checks | 23–40 ms | 30–59 ms | 1.35–1.85 s |
| Compile errors | 0 of 18 | 0 of 18 (the `Cents` return type isn't in the tool's table) | relational 51 of 200, body replacement 2 of 129 |
| Resident memory: loaded, then after the checks | 28–32 MB, 45–48 MB | 28–31 MB, 46–47 MB | 51–54 MB, 508–509 MB |

**Program reuse:** `update_program` reused the other files' trees in every check, so no program was built again from scratch.

**What the planner's compile errors are:**
- **Relational:** the 51 are real type errors. TS18048 and TS18047 come from `!==` turned into `<` or `>` on a value that may be missing; the rest are TS2322, TS2638 and others.
- **Body replacement:** the two are TS2339.

**The planner copy** was installed with `pnpm install --frozen-lockfile --ignore-scripts` in a scratch directory. The planner's own checkout wasn't changed.

**The fallback, measured:** a fresh `tsrs --noEmit -p` process per mutant costs 68–85 ms on the fixtures and 275–367 ms on the planner's `tsconfig.app.json`. It uses the npm package vile already installs, `tsrs@0.24.0-ts7.1.0-beta`, and checks the whole project. It finds the same TS2322 errors. The npm package also has a Node SDK over `tsrs --api` (`tsrs/sync`, `tsrs/async`), which wasn't measured.

**Compile time** (4 vCPUs, nothing cached):

| Build | Wall time | CPU time summed over the crates |
| --- | --- | --- |
| Release | 120 s | 378 s, of which 284 s is tsrs's crates |
| Dev | 47 s | |

- **The slowest crates:** `tsrs_checker` takes 50 s, `tsrs_compiler` 23 s and `tsrs_ast` 22 s.
- **The language server's share:** `tsrs_ls` (37 s), `tsrs_lsproto` (21 s) and `tsrs_project` (16 s) come in only for the overlay file system.

**Binary size:** the measuring binary is 20.0 MB, 15.5 MB stripped. About 4 MB of that is TypeScript's embedded `lib.*.d.ts` files. tsrs's own CLI binary is 31.4 MB.

## The Rust API

Every tsrs crate is pinned at **0.24.0** (TypeScript 7.1.0-beta, microsoft/TypeScript@3e9236361fa9). The spike depends directly on eight of them:

| Crate | What the spike uses |
| --- | --- |
| `tsrs_tsoptions` | `get_parsed_command_line_of_config_file(path, None, None, &dyn ParseConfigHost, None)` → `(Option<ParsedCommandLine>, Vec<P<Diagnostic>>)` |
| `tsrs_vfs` | `osvfs::fs()`, `bundled::wrap_fs` (the embedded libs), `bundled::lib_path()`, `cachedvfs::from` |
| `tsrs_compiler` | `new_compiler_host(…)` and `new_program(ProgramOptions::new(config, host))` → `&'static Program` |
| | `Program::update_program(path, host, None, None)` → `(&'static Program, Option<P<SourceFile>>, bool)` |
| | `get_syntactic_diagnostics` / `get_semantic_diagnostics(&Context, Some(file))` |
| | `get_type_checker_for_file(&Context, file)` → `CheckerHandle` |
| The checker, through `CheckerHandle` | `get_signature_from_declaration_exported`, `get_return_type_of_signature_exported`, `type_to_string_exported` |
| `tsrs_project` | `new_overlay`, `new_overlay_fs(FsRef::Host(fs), overlays, encoding, to_path)` |
| `tsrs_lsproto` | `PositionEncodingKind::UTF8` |
| `tsrs_ast` | `for_each_child`, `body()`, `pos()` / `end()`, `is_function_like`, `Diagnostic::{code, pos, end, localize, reports_unnecessary}` |
| `tsrs_scanner` | `skip_trivia`: a node's position includes its leading trivia |
| `tsrs_core` | `P`, `Context::background()`, `tspath::to_path`, `ensure_script_kind_from_file_name` |

**How it feels:**
- **It's Go, typed in Rust:**
  - checker methods with an `_exported` suffix;
  - `(Option, Vec<Diagnostic>)` pairs instead of `Result`;
  - a `Context` argument that's always `Context::background()`;
  - six-argument constructors where most arguments are `None`.
- **Everything is `&'static`.** Handles live in a leak arena, so they're easy to hold, but that's also why memory grows.
- **Panics stand for Go's nil dereference.** Wrong input can panic, so `vile_types` should check its input first.
- **It's thread-safe enough for tests.** Six tests load projects in parallel in one process without trouble.

**Stability:**
- **The functions used didn't change.** All 11 functions the spike calls have the same signatures in 0.24.0 and in tsrs's main of 2026-11-17 (past 7.1.0-beta.tsrs2).
- **Breaks will still come.** The API is a port of TypeScript's internal API, and the version scheme just changed (0.24.0 → 7.1.0-beta.tsrsN), so expect breaks. Pin exactly, and keep every tsrs call inside `vile_types`.

**Licenses and notice:**
- **`deny.toml`** allows three permissive licenses that tsrs's dependencies bring, each with a comment naming the crate: BSL-1.0 (xxhash-rust, and dragonbox_ecma as an alternative), `Apache-2.0 WITH LLVM-exception` (dragonbox_ecma) and Unicode-DFS-2016 (unicode-id-start).
- **`vile_core`** gets its version, `=0.0.0`, in `[workspace.dependencies]`, so cargo-deny doesn't count the path dependency as a wildcard.
- **tsrs's `NOTICE`** is copied unchanged to [`spikes/tsrs/NOTICE`](NOTICE), and typos skips it.

## Memory

**What happens:** `update_program` returns a new `&'static Program` for every check, and nothing frees it. At about 1.4 MB per check, 5,000 mutants would need about 7 GB.

**What tsrs offers:** tsrs's `free_program` is an `unsafe fn`, which vile's crates forbid. Reading its source, it also only frees the `Program` struct: the checker's types and the patched tree stay in the arena unless checker regions are on. That's from reading, untried.

**The options:**
1. **A worker process** that starts again after N checks or above a memory limit. Reloading the planner costs 40–90 ms, so well under 0.1 ms per check at N = 1,000.
2. **Checker regions with `free_program`**, behind one reviewed `unsafe` exception.
3. **Ask tsrs upstream** for a safe way to drop a program version.

## What isn't covered

- **Errors in other files:** only the patched file is checked. Mutants that change an inferred return type need the importing files checked too, as the plan's risk table says; that wasn't measured.
- **Measurement scope:** the planner's relational mutants come from a text scan, not Oxc, and only Linux x64 was measured.
- **Not tried:** checker regions, `free_program` and the Node SDK fallback.

## Decisions for the maintainer

1. **Memory:** a worker that restarts, checker regions behind one `unsafe` exception, or asking tsrs upstream.
2. **Whether to keep tsrs's overlay file system.** It pulls in the language-server crates (about 75 s of compile CPU), where about 30 lines of vile's own `FS` over `tsrs_vfs`'s public trait would do. #13 asked for the overlay, so the spike uses it.
3. **Which diagnostics to leave out:** every `reports_unnecessary()` diagnostic, as the spike does, or a narrower list of codes.
4. **Who owns tsrs updates,** with this spike's tests as the check.
