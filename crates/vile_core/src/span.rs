//! Where something is in a source file.

use serde::{Deserialize, Serialize};

/// A range of a source file, in UTF-8 byte offsets from its start.
///
/// `start` is the first byte, `end` the one after the last. Oxc's spans and tsrs's positions count the same way, so a span
/// from one means the same text in the other.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Span {
    pub start: u32,
    pub end: u32,
}

/// A position for people: the line and the column, both counted from 1, the column in characters
/// (Unicode scalar values) the way editors count them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Location {
    pub line: u32,
    pub column: u32,
}

impl Span {
    /// The span's length in bytes.
    #[must_use]
    pub const fn length(self) -> u32 {
        self.end.saturating_sub(self.start)
    }

    /// The text the span covers, or `None` when it doesn't fit the source: past its end, reversed,
    /// or cutting a character in half.
    #[must_use]
    pub fn text(self, source: &str) -> Option<&str> {
        source.get(usize::try_from(self.start).ok()?..usize::try_from(self.end).ok()?)
    }

    /// Where the span starts, as a line and column, or `None` when the start isn't a character
    /// boundary of the source.
    #[must_use]
    pub fn locate(self, source: &str) -> Option<Location> {
        let before = source.get(..usize::try_from(self.start).ok()?)?;
        let line_start = before.rfind('\n').map_or(0, |newline| newline + 1);
        let line = before.matches('\n').count() + 1;
        let column = before[line_start..].chars().count() + 1;
        Some(Location {
            line: u32::try_from(line).ok()?,
            column: u32::try_from(column).ok()?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{Location, Span};

    #[test]
    fn reads_the_text_it_covers() {
        let span = Span { start: 2, end: 3 };
        assert_eq!(span.text("a < b"), Some("<"));
        assert_eq!(span.length(), 1);
    }

    #[test]
    fn refuses_a_span_outside_the_source_or_reversed() {
        assert_eq!(Span { start: 4, end: 9 }.text("a < b"), None);
        assert_eq!(Span { start: 3, end: 2 }.text("a < b"), None);
    }

    #[test]
    fn refuses_a_span_that_cuts_a_character() {
        // "ä" is two bytes, so byte 1 is inside it.
        assert_eq!(Span { start: 1, end: 2 }.text("ä"), None);
        assert_eq!(Span { start: 1, end: 1 }.locate("ä"), None);
    }

    #[test]
    fn locates_lines_and_columns_from_one_counting_characters() {
        let source = "let a = 1;\nlet ä = a < 2;";
        // The "<" on the second line: "let ä = a " is 10 characters but 11 bytes.
        let start = u32::try_from(source.find('<').expect("the source has a <")).expect("it fits");
        let span = Span {
            start,
            end: start + 1,
        };
        assert_eq!(
            span.locate(source),
            Some(Location {
                line: 2,
                column: 11
            })
        );
        assert_eq!(
            Span { start: 0, end: 0 }.locate(source),
            Some(Location { line: 1, column: 1 })
        );
    }
}
