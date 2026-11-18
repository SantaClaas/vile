//! The text a mutant changes.

use serde::{Deserialize, Serialize};

use crate::span::Span;

/// One contiguous edit: the span of the original that changes and the text that goes there
/// instead.
///
/// vile applies mutants as patches to the text, never by printing a changed syntax tree,
/// so formatting, comments and line numbers stay as they were (as in cargo-mutants).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Patch {
    pub span: Span,
    pub replacement: String,
}

/// Why a patch doesn't fit a source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatchError {
    /// The span reaches past the end of the source, or cuts a character in half.
    OutsideSource,
}

impl Patch {
    /// The source with the patch applied.
    ///
    /// # Errors
    ///
    /// [`PatchError::OutsideSource`] when the span doesn't fit the source.
    pub fn apply(&self, source: &str) -> Result<String, PatchError> {
        let start = usize::try_from(self.span.start()).map_err(|_| PatchError::OutsideSource)?;
        let end = usize::try_from(self.span.end()).map_err(|_| PatchError::OutsideSource)?;
        let before = source.get(..start).ok_or(PatchError::OutsideSource)?;
        let after = source.get(end..).ok_or(PatchError::OutsideSource)?;
        Ok([before, &self.replacement, after].concat())
    }
}

#[cfg(test)]
mod tests {
    use super::{Patch, PatchError};
    use crate::span::Span;

    fn patch(start: u32, end: u32, replacement: &str) -> Patch {
        Patch {
            span: Span::new(start, end).expect("the test's span isn't reversed"),
            replacement: replacement.to_owned(),
        }
    }

    #[test]
    fn replaces_the_span_and_keeps_the_rest() {
        assert_eq!(
            patch(2, 3, "<=").apply("a < b // keep"),
            Ok("a <= b // keep".to_owned())
        );
    }

    #[test]
    fn inserts_and_deletes() {
        assert_eq!(patch(0, 0, "!").apply("a"), Ok("!a".to_owned()));
        assert_eq!(patch(0, 1, "").apply("!a"), Ok("a".to_owned()));
    }

    #[test]
    fn refuses_a_span_that_doesnt_fit() {
        assert_eq!(
            patch(2, 9, "x").apply("a < b"),
            Err(PatchError::OutsideSource)
        );
        assert_eq!(patch(1, 2, "x").apply("ä"), Err(PatchError::OutsideSource));
    }
}
