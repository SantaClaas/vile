//! The data vile's stages pass between them: where a mutant is ([`span`]), what it changes
//! ([`patch`]), which kind of change it is ([`operator`]), the mutant and its identity
//! ([`mutant`]), what happened to it ([`outcome`]), and the JSON files other programs read
//! ([`output`]).
//!
//! These types are the contracts between the workstreams in docs/PLAN.md, so they change only in
//! pull requests of their own.

pub mod mutant;
pub mod operator;
pub mod outcome;
pub mod output;
pub mod patch;
pub mod span;
