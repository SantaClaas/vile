//! The kinds of change that make mutants (see Mutation operators in docs/PLAN.md).

use serde::{Deserialize, Serialize};

/// A kind of change that makes mutants.
///
/// The names are part of vile's interface: they appear in
/// disable comments (`vile-disable-next-line relational -- why`), in the config and in the JSON,
/// so renaming one is a breaking change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Operator {
    /// Replaces a function's body with a value of its return type (cargo-mutants' `FnValue`,
    /// extreme mutation in PIT's Descartes).
    BodyReplacement,
    /// Replaces a comparison with the sufficient set for it (Kaminski, Ammann, Offutt): `<` with
    /// `<=`, `!=` and `false`, and so on.
    Relational,
    /// Swaps `&&` and `||`, and replaces `a ?? b` with `a`.
    Logical,
    /// Swaps arithmetic operators: `+` and `-`, `*` and `/`, `%` with `*`.
    Arithmetic,
    /// Removes `!` and unary `-`.
    Unary,
    /// Empties a block that isn't a function's body (Google's SBR).
    BlockRemoval,
}

impl Operator {
    /// Every operator, in a fixed order.
    pub const ALL: [Self; 6] = [
        Self::BodyReplacement,
        Self::Relational,
        Self::Logical,
        Self::Arithmetic,
        Self::Unary,
        Self::BlockRemoval,
    ];

    /// The operator's name, as in disable comments, the config and the JSON.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::BodyReplacement => "body-replacement",
            Self::Relational => "relational",
            Self::Logical => "logical",
            Self::Arithmetic => "arithmetic",
            Self::Unary => "unary",
            Self::BlockRemoval => "block-removal",
        }
    }

    /// The operator with this name, as a user writes it in a disable comment or the config.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|operator| operator.name() == name)
    }
}

#[cfg(test)]
mod tests {
    use super::Operator;

    #[test]
    fn reads_back_every_name_it_writes() {
        for operator in Operator::ALL {
            assert_eq!(Operator::from_name(operator.name()), Some(operator));
        }
        assert_eq!(Operator::from_name("relation"), None);
    }

    #[test]
    fn writes_the_same_names_in_the_json() {
        for operator in Operator::ALL {
            let json = serde_json::to_string(&operator).expect("an operator serializes");
            assert_eq!(json, format!("\"{}\"", operator.name()));
        }
    }
}
