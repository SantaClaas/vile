//! Spike: tsrs as a Rust library (#13). Loads a project from its `tsconfig.json`, finds a
//! function's return type at a byte offset, and checks a patched file in memory. The findings are
//! in `spikes/tsrs/REPORT.md`.

pub mod project;
