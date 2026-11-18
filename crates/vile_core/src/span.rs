//! Where something is in a source file.

use serde::{Deserialize, Serialize};

/// A range of a source file, in UTF-8 byte offsets from its start.
///
/// `start` is the first byte, `end` the one after the last. Oxc's spans and tsrs's positions count
/// the same way, so a span from one means the same text in the other. The fields are private so
/// that every span, read from JSON too, starts at or before its end.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "UncheckedSpan")]
pub struct Span {
    start: u32,
    end: u32,
}

/// A span as the JSON writes it, before it's checked.
#[derive(Deserialize)]
struct UncheckedSpan {
    start: u32,
    end: u32,
}

/// Why a span can't be made.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum SpanError {
    #[error("the span's end comes before its start")]
    Reversed,
}

impl TryFrom<UncheckedSpan> for Span {
    type Error = SpanError;

    fn try_from(span: UncheckedSpan) -> Result<Self, Self::Error> {
        Self::new(span.start, span.end)
    }
}

/// A line and a column, both counted from 1. Columns count UTF-16 code units, as the
/// mutation-testing-elements report, editors (LSP) and SARIF do by default.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Position {
    pub line: u32,
    pub column: u32,
}

/// Where a span is, for people: its start and its end, which is exclusive. The report schema calls
/// this a location.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

/// The position of a byte offset, or `None` when it isn't a character boundary of the source.
fn find_position(source: &str, offset: u32) -> Option<Position> {
    let before = source.get(..usize::try_from(offset).ok()?)?;
    let line_start = before.rfind('\n').map_or(0, |newline| newline + 1);
    let line = before.matches('\n').count() + 1;
    let column = before[line_start..].encode_utf16().count() + 1;
    Some(Position {
        line: u32::try_from(line).ok()?,
        column: u32::try_from(column).ok()?,
    })
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

    /// Where the span starts and ends, or `None` when either end isn't a character boundary of
    /// the source.
    #[must_use]
    pub fn locate(self, source: &str) -> Option<Range> {
        Some(Range {
            start: find_position(source, self.start)?,
            end: find_position(source, self.end)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{Position, Range, Span, SpanError};

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

    fn position(line: u32, column: u32) -> Position {
        Position { line, column }
    }

    #[test]
    fn locates_both_ends_from_one() {
        let source = "let a = 1;\nlet b = a < 2;";
        let start = u32::try_from(source.find('<').expect("the source has a <")).expect("it fits");
        assert_eq!(
            span(start, start + 1).locate(source),
            Some(Range {
                start: position(2, 11),
                end: position(2, 12),
            })
        );
        assert_eq!(
            span(0, 3).locate(source),
            Some(Range {
                start: position(1, 1),
                end: position(1, 4),
            })
        );
    }

    #[test]
    fn locates_a_span_across_lines() {
        let source = "if (a) {\n  b();\n}";
        let start = u32::try_from(source.find('{').expect("the source has a {")).expect("it fits");
        let end = u32::try_from(source.len()).expect("it fits");
        assert_eq!(
            span(start, end).locate(source),
            Some(Range {
                start: position(1, 8),
                end: position(3, 2),
            })
        );
    }

    #[test]
    fn counts_columns_in_utf_16_code_units() {
        // "ä" is one UTF-16 unit and two bytes; "😀" is two UTF-16 units and four bytes.
        let source = "ä😀 < b";
        let start = u32::try_from(source.find('<').expect("the source has a <")).expect("it fits");
        assert_eq!(
            span(start, start + 1).locate(source),
            Some(Range {
                start: position(1, 5),
                end: position(1, 6),
            })
        );
    }
}
