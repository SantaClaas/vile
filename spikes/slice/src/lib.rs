//! Phase 1 spike S1: finds `<` comparisons and function bodies with Oxc and turns them into
//! `vile_core` mutants. The binary writes them to `mutants.json` for the runner in `runner/`.
#![expect(
    clippy::multiple_crate_versions,
    reason = "Oxc's dependencies pull in syn 2 (phf_macros) and syn 3 (displaydoc); only they can unify it"
)]

pub mod generate;
pub mod value;
