//! One small change to the code under test, and the name that finds it again.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::{operator::Operator, patch::Patch, span::Location};

/// A mutant's identity, the same from run to run as long as the code it changes stays the same.
///
/// It doesn't use the line or the column, which change whenever code above it changes; that's the
/// weak spot cargo-mutants names in `--iterate`. Instead it's made of what the mutant is: the file,
/// the names of the functions around it, the text it replaces (hashed), the operator, the
/// replacement, and which occurrence of that text it is within the function, for the rare function
/// that has the same text twice.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct MutantId(String);

/// What a [`MutantId`] is made of.
#[derive(Debug, Clone, Copy)]
pub struct Identity<'a> {
    /// The file's path from the project's root, with `/` between folders on every platform.
    pub file: &'a str,
    /// The names of the functions, classes and methods around the site, outermost first, such as
    /// `["Slot", "findFirstFree"]`. Empty at the top level of a module.
    pub scope: &'a [String],
    pub original: &'a str,
    pub operator: Operator,
    pub replacement: &'a str,
    /// Counts from 0: the how-manieth site in the same scope with the same original text, operator
    /// and replacement.
    pub occurrence: u32,
}

/// The 64-bit FNV-1a hash, written out here so a mutant's ID never depends on a library's or the
/// platform's hashing, which may change between versions.
fn hash_fnv_1a(text: &str) -> u64 {
    const OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;
    text.bytes().fold(OFFSET_BASIS, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(PRIME)
    })
}

impl MutantId {
    /// The ID for a mutant made of these parts.
    #[must_use]
    pub fn new(identity: &Identity<'_>) -> Self {
        let scope = identity.scope.join(".");
        let original = hash_fnv_1a(identity.original);
        let replacement = hash_fnv_1a(identity.replacement);
        Self(format!(
            "{file}:{scope}:{operator}:{original:016x}:{replacement:016x}:{occurrence}",
            file = identity.file,
            operator = identity.operator.name(),
            occurrence = identity.occurrence,
        ))
    }

    /// The ID as text, as it appears in the JSON.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for MutantId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// One small change to the code under test.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Mutant {
    pub id: MutantId,
    /// The file's path from the project's root, with `/` between folders.
    pub file: String,
    pub operator: Operator,
    /// The text the patch replaces, so a report can show the change without the source.
    pub original: String,
    pub patch: Patch,
    /// Where the patch starts, for people. The patch's span is what programs use.
    pub location: Location,
}

#[cfg(test)]
mod tests {
    use super::{Identity, MutantId, hash_fnv_1a};
    use crate::operator::Operator;

    fn identity<'a>(scope: &'a [String], replacement: &'a str) -> Identity<'a> {
        Identity {
            file: "src/slot.ts",
            scope,
            original: "<",
            operator: Operator::Relational,
            replacement,
            occurrence: 0,
        }
    }

    #[test]
    fn hashes_like_the_published_fnv_1a() {
        // Test vectors from the FNV reference (http://www.isthe.com/chongo/tech/comp/fnv/).
        assert_eq!(hash_fnv_1a(""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(hash_fnv_1a("a"), 0xaf63_dc4c_8601_ec8c);
        assert_eq!(hash_fnv_1a("foobar"), 0x8594_4171_f739_67e8);
    }

    #[test]
    fn stays_the_same_for_the_same_parts() {
        let scope = vec!["Slot".to_owned(), "findFirstFree".to_owned()];
        assert_eq!(
            MutantId::new(&identity(&scope, "<=")),
            MutantId::new(&identity(&scope, "<="))
        );
        assert_eq!(
            MutantId::new(&identity(&scope, "<=")).as_str(),
            "src/slot.ts:Slot.findFirstFree:relational:af63b14c8601a37b:08098b07b4c86ff2:0"
        );
    }

    #[test]
    fn differs_when_any_part_differs() {
        let scope = vec!["findFirstFree".to_owned()];
        let other_scope = vec!["layoutLanes".to_owned()];
        let base = MutantId::new(&identity(&scope, "<="));
        assert_ne!(base, MutantId::new(&identity(&scope, "!=")));
        assert_ne!(base, MutantId::new(&identity(&other_scope, "<=")));
        assert_ne!(
            base,
            MutantId::new(&Identity {
                occurrence: 1,
                ..identity(&scope, "<=")
            })
        );
        assert_ne!(
            base,
            MutantId::new(&Identity {
                file: "src/lane.ts",
                ..identity(&scope, "<=")
            })
        );
    }
}
