# vile

Mutation testing for TypeScript and JavaScript, built on Oxc, tsrs and Vitest. It changes the code a little at a time (a `<` to `<=`, a function body to `return []`) and runs the tests against each change: a change no test notices is a place where the code could be wrong without anyone knowing.

## Where things are

- [`docs/PLAN.md`](docs/PLAN.md): the plan, from the architecture and the operators to the phases and how the work is split onto subagents. Read it before starting any work, and keep it current when a decision changes.
- [`docs/GLOSSARY.md`](docs/GLOSSARY.md): the words for things, which load in every session. "Caught" and "missed", never "killed" and "survived", except when quoting the research.
- [`docs/guidelines/`](docs/guidelines/): how code is named, split, styled and checked, one file per kind of code, listed in [`docs/CODING-GUIDELINES.md`](docs/CODING-GUIDELINES.md). `principles.md` always applies, including what a tool needs that an app doesn't; the others load by themselves when you touch matching files, through [`.claude/rules/`](.claude/rules/). Read the one for the code you're about to write before planning it.
- [`README.md`](README.md): what vile is, its license and its credits. Credit every project and paper vile takes from.
- **Future work: [#1](https://github.com/SantaClaas/vile/issues/1)**, ideas left for after version 1.

## Stack

- **Rust** for the analysis: [Oxc](https://github.com/oxc-project/oxc) parses, finds mutation sites, minifies (for equivalent mutants) and prints diagnostics; [tsrs](https://github.com/oxc-project/tsrs), a Rust port of TypeScript 7, checks types. Both are pinned exactly, since their minor releases break APIs. See [`docs/guidelines/rust.md`](docs/guidelines/rust.md).
- **TypeScript on Node** for running tests: a package on Vitest's Node API, and a napi-rs binding to the Rust crates.
- **Don't write Oxc, tsrs or Vitest code from memory.** Their APIs change quickly and most examples online are out of date. Read the docs and the source of the pinned version first.
- **pnpm 12**, pinned with its hash in `packageManager`. [`pnpm-workspace.yaml`](pnpm-workspace.yaml) is strict about the supply chain: a version must be a week old before it installs, trust can't drop between releases, only listed packages run install scripts, and versions are saved exact. Each exception there names the exact version and says why. Never loosen these settings, or pass flags that skip them, to get an install through; ask instead.
- **Node 26** for development ([`.node-version`](.node-version)); the published packages support every maintained Node release (see [A tool, not an app](docs/guidelines/principles.md#a-tool-not-an-app)).
- **Checks before committing:** `pnpm format`, `pnpm lint`, `pnpm typecheck`, `pnpm test`, `pnpm links`, `pnpm knip` and `typos`, plus `cargo fmt`, `cargo clippy` and `cargo test` once there's Rust. CI runs them all.
- **GitHub Actions:** every action is pinned to a commit hash with its version in a comment, CI lints the workflows with actionlint and zizmor, and Dependabot proposes updates a week after each release. See [`docs/guidelines/workflows.md`](docs/guidelines/workflows.md).

## Working agreements

- **The consumer's needs come first**: the developer reading vile's results, and the programs reading its JSON. See [`docs/guidelines/principles.md`](docs/guidelines/principles.md).
- **Small pull requests.** One per workstream at most, and smaller where a workstream splits into parts that can be reviewed on their own. Agents open pull requests; the maintainer merges.
- **Follow the state of the art.** vile is a general tool: where mutation testing research or established tools (cargo-mutants, PIT, Stryker, Google's practice) have learned something, follow it, cite it in the docs, and credit it.
- **Each workstream owns its directory.** The shared contracts in `vile_core` change only in their own pull requests.
