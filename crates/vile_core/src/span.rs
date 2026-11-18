//! Where something is in a source file.

use serde::{Deserialize, Serialize};

/// A range of a source file, in UTF-8 byte offsets from its start.
///
/// `start` is the first byte, `end` the one after the last. Oxc's spans and tsrs's positions count
/// the same way, so a span from one means the same text in the other. The fields are private so
/// that every span, read from JSON too, starts at or before its end.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "Range")]
pub struct Span {
    start: u32,
    end: u32,
}

/// A span as the JSON writes it, before it's checked.
#[derive(Deserialize)]
struct Range {
    start: u32,
    end: u32,
}

/// Why a span can't be made.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpanError {
    /// The end comes before the start.
    Reversed,
}

impl std::fmt::Display for SpanError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("a span's end comes before its start")
    }
}

impl TryFrom<Range> for Span {
    type Error = SpanError;

    fn try_from(range: Range) -> Result<Self, Self::Error> {
        Self::new(range.start, range.end)
    }
}

/// A position for people: the line and the column, both counted from 1, the column in characters
/// (Unicode scalar values) the way editors count them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Location {
    pub line: u32,
    pub column: u32,
}

impl Span {
    /// The span from `start` to `end`.
    ///
    /// # Errors
    ///
    /// [`SpanError::Reversed`] when `end` comes before `start`.
    pub const fn new(start: u32, end: u32) -> Result<Self, SpanError> {
        if end < start {
            return Err(SpanError::Reversed);
        }
        Ok(Self { start, end })
    }

    #[must_use]
    pub const fn start(self) -> u32 {
        self.start
    }

    #[must_use]
    pub const fn end(self) -> u32 {
        self.end
    }

    /// The text the span covers, or `None` when it doesn't fit the source: past its end, or
    /// cutting a character in half.
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
    use super::{Location, Span, SpanError};

    fn span(start: u32, end: u32) -> Span {
        Span::new(start, end).expect("the test's span isn't reversed")
    }

    #[test]
    fn reads_the_text_it_covers() {
        assert_eq!(span(2, 3).text("a < b"), Some("<"));
    }

    #[test]
    fn refuses_a_reversed_span() {
        assert_eq!(Span::new(3, 2), Err(SpanError::Reversed));
        assert!(Span::new(2, 2).is_ok());
    }

    #[test]
    fn refuses_a_reversed_span_in_json() {
        let read: Result<Span, _> = serde_json::from_str(r#"{ "start": 3, "end": 2 }"#);
        assert!(read.is_err());
        let read: Span = serde_json::from_str(r#"{ "start": 2, "end": 3 }"#).expect("it reads");
        assert_eq!(read, span(2, 3));
    }

    #[test]
    fn refuses_a_span_outside_the_source() {
        assert_eq!(span(4, 9).text("a < b"), None);
    }

    #[test]
    fn refuses_a_span_that_cuts_a_character() {
        // "ä" is two bytes, so byte 1 is inside it.
        assert_eq!(span(1, 2).text("ä"), None);
        assert_eq!(span(1, 1).locate("ä"), None);
    }

    #[test]
    fn locates_lines_and_columns_from_one_counting_characters() {
        let source = "let a = 1;\nlet ä = a < 2;";
        // The "<" on the second line: "let ä = a " is 10 characters but 11 bytes.
        let start = u32::try_from(source.find('<').expect("the source has a <")).expect("it fits");
        assert_eq!(
            span(start, start + 1).locate(source),
            Some(Location {
                line: 2,
                column: 11
            })
        );
        assert_eq!(
            span(0, 0).locate(source),
            Some(Location { line: 1, column: 1 })
        );
    }
}
