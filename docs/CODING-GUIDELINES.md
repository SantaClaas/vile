# Coding guidelines

How code in vile is named, split, styled and checked. The guidelines live in [`docs/guidelines/`](guidelines/), one file per kind of code, so that each context only needs the parts that apply to it.

| File | Applies to | What's in it |
| --- | --- | --- |
| [`principles.md`](guidelines/principles.md) | all code, always | the consumer's needs first, what a tool needs that an app doesn't, the rule of least power, names, pure functions, early returns, one operation per `try` and cleanup with `using`, modules (one noun each, named in the singular) and coupling, refactoring, where rules are enforced, linking files and drawing diagrams in the docs |
| [`rust.md`](guidelines/rust.md) | `.rs`, `Cargo.toml` | reading Oxc's and tsrs's sources first, the workspace, names, errors as values, `Drop` for cleanup that can't fail and explicit methods for the rest, deterministic output, strict clippy with reasoned `#[expect]`, cargo-deny and notices |
| [`typescript.md`](guidelines/typescript.md) | `.ts`, `.js` | function and boolean names, branded types, results, discriminated unions, iterator helpers instead of array chains, formatting and linting, no type assertions, explained exceptions |
| [`tests.md`](guidelines/tests.md) | `*.test.ts`, `tests/`, `fixtures/` | what earns a test, unit tests, fixture projects as acceptance tests, snapshots of the output, running vile on real code and on itself |
| [`workflows.md`](guidelines/workflows.md) | [`.github/`](../.github/) | hash-pinned actions, workflow linting, least privilege, Dependabot |

Start with the principles; they hold everywhere. Before working on a kind of code, read its file too. Claude Code loads them by itself: [`.claude/rules/`](../.claude/rules/) links to each file, and each file lists the paths it applies to in its front matter, so it loads the first time a matching file is read or edited. The principles have no paths, so they load at the start of every session.

**These are idealistic documents.** They describe the code we strive for, not code we expect to reach perfectly, and if one guideline turns out wrong, the others still apply (see the start of `principles.md`).
