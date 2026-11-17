---
paths:
  - "**/*.rs"
  - "**/Cargo.toml"
  - "Cargo.lock"
  - "rust-toolchain.toml"
  - "clippy.toml"
  - "deny.toml"
---

# Rust

How the Rust in `crates/` is named, split, checked and released. It adds to [`docs/guidelines/principles.md`](principles.md), which always applies: the names, the pure core, the modules and the stages where rules are enforced hold in Rust as they do in TypeScript. Where Rust has its own way of doing the same thing, this file says which.

## Sources

**Read the source, not your memory.** Oxc and tsrs change their APIs between releases, and tsrs is a port of TypeScript's Go code, so its Rust API keeps Go's shape (`get_return_type_of_signature_exported`). Before using a crate, read its documentation on docs.rs for the pinned version, and its source where the documentation is thin: Oxc's [`crates/`](https://github.com/oxc-project/oxc/tree/main/crates) and tsrs's [`docs/AST.md`](https://github.com/oxc-project/tsrs/blob/main/docs/AST.md), [`docs/CHECKER.md`](https://github.com/oxc-project/tsrs/blob/main/docs/CHECKER.md) and [`docs/RUST.md`](https://github.com/oxc-project/tsrs/blob/main/docs/RUST.md).

## The workspace

- **One crate per feature**, named `vile_<noun>` like Oxc's `oxc_<noun>`: `vile_core`, `vile_generate`, `vile_types`, `vile_report`. A crate passes the package test from the principles: it could be published or deleted without touching much else. The dependencies between crates go one way, and Cargo refuses a cycle.
- **Versions are pinned exactly** (`=1.2.3`) for Oxc and tsrs, whose minor releases break APIs, and in the workspace's `[workspace.dependencies]`, so every crate uses the same one.
- **The toolchain is pinned** in `rust-toolchain.toml`, with `rust-version` in `Cargo.toml` set to the same version, so clippy finds the same things everywhere. tsrs needs at least the version pinned there. An upgrade is a pull request of its own.

## Names

The principles' rules apply: **no abbreviations** (`expression`, not `expr`; `span`, not `sp`; `context`, not `ctx`), except where a name belongs to the other side of an edge, such as Oxc's `AstKind` or tsrs's `P<Node>`. Rust's own conventions come first where they differ: `snake_case` functions, `UpperCamelCase` types, and the standard names (`new`, `from`, `into_*`, `as_*`, `is_*`, `iter`).

- **A function starts with a verb**, as in TypeScript, except Rust's conventional constructors and conversions (`new`, `from_*`) and getters, which Rust names after what they return (`span()`, not `get_span()`).
- **A predicate is a question**: `is_viable`, `is_arid`.
- **Module functions drop the module's noun**, as the principles say: `operator::all()`, not `operator::all_operators()`.

## Functions

- **A pure core**, as in the principles: generating mutants, checking equivalence and laying out a report take their input and return their output. The file system, the clock, processes and Node live at the edges, in the CLI and the binding.
- **Return early.** Guards return or `continue` first. `let … else` is the guard for a pattern. An `if … else` that chooses between two values is Rust's ternary and stays; one that does two things in two branches gets a function per branch. Clippy's `redundant_else`, `single_match_else` and `collapsible_if` catch the common cases.
- **Declare before use.** Rust doesn't need it, but files still read bottom-up: types, constants and helpers first, the module's public entry point last. No lint checks it, so review does.
- **Errors are values.** What a caller can act on is a `Result` whose error is an `enum` naming each case (`ParseError::UnexpectedToken`), never a `String` and never `Box<dyn Error>` inside a library crate. The CLI turns them into messages for people at the edge. A panic is for a bug in vile, as a throw is in TypeScript.
- **`?` is the one-operation `try`.** Each fallible call gets its own `?`, with `map_err` where the error needs the context of that one operation.
- **Cleanup is `Drop`.** A guard that restores something when its scope ends, however it ends (a file written back, a worker killed), is a type with `Drop`, the way TypeScript uses `using`. A guard nobody reads is bound to `_name`, never to `_`, which drops it at once.
- **Deterministic output.** vile's results must be the same on every run: never let a hash map's iteration order reach output, a mutant's ID or a report. Sort, or use an ordered map. Clippy's `iter_over_hash_type` is on for this, after tsrs.

## Checks

**Clippy is strict, and its exceptions say why.** The workspace turns on clippy's `all`, `pedantic`, `nursery` and `cargo` groups as errors, as Oxc and tsrs do, and rustc's warnings fail the build in CI (`RUSTFLAGS=-D warnings`). An exception is `#[expect(clippy::lint_name, reason = "why it doesn't apply here")]` on the smallest item it covers. `#[allow]` isn't allowed (`clippy::allow_attributes`, `allow_attributes_without_reason`): `#[expect]` warns once the lint stops firing, so an exception can't outlive its cause.

- **No `unsafe`.** The vile crates `forbid(unsafe_code)`. The binding to Node is napi-rs's generated code, outside our crates.
- **No `unwrap()` in library code.** A value that can't be missing is shown to be present, or `expect("why it can't fail")` says why, which is Rust's version of an explained type assertion (see "Where rules are enforced" in the principles). Tests may `unwrap()`.
- **rustfmt** formats with its defaults; `cargo fmt --check` runs in CI.
- **cargo-deny** (`deny.toml`) checks the licenses of every dependency against the allowed list, refuses crates from anywhere but crates.io, and reports advisories. A license or a source that isn't allowed fails CI.
- **Third-party notices** are generated from `Cargo.lock`, never written by hand, so they can't drift from what's linked. tsrs's `NOTICE` (TypeScript and DefinitelyTyped) ships with every binary that includes it.

## Tests

Unit tests sit next to the code, in a `#[cfg(test)] mod tests` at the end of the file. Tests that need more than one crate, or a fixture project, live in the crate's `tests/`. How tests are chosen is in [`docs/guidelines/tests.md`](tests.md).
