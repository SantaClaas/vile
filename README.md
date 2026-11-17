# vile

Mutation testing for TypeScript and JavaScript, built on [Oxc](https://oxc.rs), [tsrs](https://github.com/oxc-project/tsrs) and [Vitest](https://vitest.dev).

Work in progress.

## License

Licensed under either of

- Apache License, Version 2.0 ([`LICENSE-APACHE`](LICENSE-APACHE))
- MIT license ([`LICENSE-MIT`](LICENSE-MIT))

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.

## Credits

vile stands on the work of others:

- **[Oxc](https://github.com/oxc-project/oxc)** (MIT): parsing, the AST, the minifier, the transformer and the diagnostics.
- **[tsrs](https://github.com/oxc-project/tsrs)** (Apache-2.0): type checking. tsrs is a port of TypeScript, Copyright (c) Microsoft Corporation, and embeds TypeScript's `lib.*.d.ts` files. Its [`NOTICE`](https://github.com/oxc-project/tsrs/blob/main/NOTICE) ships with every vile binary that includes it.
- **[Vitest](https://github.com/vitest-dev/vitest)** (MIT): running the tests.
- **[napi-rs](https://github.com/napi-rs/napi-rs)** (MIT): the binding between Rust and Node.
- **[cargo-mutants](https://github.com/sourcefrog/cargo-mutants)** by Martin Pool (MIT): much of vile's design, from its outcomes, baseline, timeouts and output folder to `--in-diff`, `--iterate` and replacing function bodies.
- **[Stryker](https://stryker-mutator.io/)**: mutation switching, and the mutation-testing-elements report format that vile writes.
- **Research** that shapes which mutants vile makes and which it leaves out, cited in [`docs/PLAN.md`](docs/PLAN.md#sources): DeMillo, Lipton and Sayward; Offutt et al.; Kaminski, Ammann and Offutt; Papadakis et al.; Just et al.; Petrović, Ivanković, Fraser and Just.
