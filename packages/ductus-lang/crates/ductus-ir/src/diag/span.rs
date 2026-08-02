//! The `Span` struct: a byte range in source text.

/// A byte range in source text: `start` offset, `len` bytes (R27).
///
/// Byte offsets only. Line and column are derived on demand by
/// [`Span::line_col`] at diagnostic-render time and never stored, so a span
/// stays cheap to copy and cannot go stale against the text it points into.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    /// Byte offset of the span's first byte.
    pub start: usize,
    /// Length of the span in bytes.
    pub len: usize,
}

impl Span {
    /// A span of `len` bytes beginning at byte offset `start`.
    ///
    /// The fields stay public — literal construction reads well in tests — so
    /// this is additive convenience, not an encapsulation boundary.
    #[must_use]
    pub fn new(start: usize, len: usize) -> Span {
        Span { start, len }
    }

    /// The byte offset one past the span's last byte.
    ///
    /// The single home of that arithmetic: nothing else writes `start + len`.
    #[must_use]
    pub fn end(&self) -> usize {
        self.start + self.len
    }

    /// The 1-based `(line, column)` of this span's `start` within `source`.
    ///
    /// The first character of the source is line 1, column 1. Only `\n` ends a
    /// line, and the `\n` itself reports as the last column of the line it
    /// closes. A `\r` is an ordinary character that keeps its own column, so in
    /// a CRLF ending the `\r` sits one column before the `\n`.
    ///
    /// Columns count Unicode scalar values (`char`s) — not bytes, and not
    /// grapheme clusters. A multi-byte character advances the column by one; a
    /// base character plus a combining mark advances it by two. A leading BOM
    /// (`U+FEFF`) is an ordinary character too and occupies column 1, so with a
    /// BOM present the first visible character reports column 2. The lexer
    /// strips a BOM when one exists (increment 3), and spans taken after that
    /// stripping are unaffected.
    ///
    /// A `start` that is not a character boundary rounds **up** to the next
    /// boundary: an offset landing inside a multi-byte character reports the
    /// column of the character that *follows* it. A past-the-end `start`
    /// reports the position just after the final character. Reporting a
    /// slightly-off position beats panicking inside an error path.
    ///
    /// Cost is O(`start`) — every call walks `source` from byte 0. This is a
    /// diagnostic-render-time convenience only. Bulk callers (the lexer, the
    /// parser) track line and column themselves and must never call this per
    /// token.
    #[must_use]
    pub fn line_col(&self, source: &str) -> (u32, u32) {
        let mut line = 1;
        let mut column = 1;

        for (offset, character) in source.char_indices() {
            if offset >= self.start {
                break;
            }

            if character == '\n' {
                line += 1;
                column = 1;
            } else {
                column += 1;
            }
        }

        (line, column)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Byte offsets: `{` at 6, `\n` at 7, `a` at 10, `}` at 16, end at 18.
    const SOURCE: &str = "graph {\n  a = 1\n}\n";

    #[test]
    fn new_and_end_bracket_the_byte_range() {
        let span = Span::new(10, 5);

        assert_eq!(span, Span { start: 10, len: 5 });
        assert_eq!(span.end(), 15);
        assert_eq!(Span::new(0, 0).end(), 0);
    }

    #[test]
    fn line_col_starts_at_one_one() {
        assert_eq!(Span::new(0, 5).line_col(SOURCE), (1, 1));
    }

    #[test]
    fn line_col_counts_columns_within_a_line() {
        assert_eq!(Span::new(6, 1).line_col(SOURCE), (1, 7));
        // The newline closing line 1 reports as that line's last column.
        assert_eq!(Span::new(7, 1).line_col(SOURCE), (1, 8));
    }

    #[test]
    fn line_col_advances_after_a_newline() {
        assert_eq!(Span::new(8, 1).line_col(SOURCE), (2, 1));
        assert_eq!(Span::new(10, 1).line_col(SOURCE), (2, 3));
        assert_eq!(Span::new(16, 1).line_col(SOURCE), (3, 1));
    }

    #[test]
    fn line_col_clamps_past_the_end() {
        let end = SOURCE.len();

        assert_eq!(Span::new(end, 0).line_col(SOURCE), (4, 1));
        assert_eq!(Span::new(end + 999, 1).line_col(SOURCE), (4, 1));
        assert_eq!(Span::new(0, 0).line_col(""), (1, 1));
    }

    #[test]
    fn line_col_counts_characters_not_bytes() {
        // Each Greek letter is two bytes: alpha 0..2, beta 2..4, `\n` 4..5.
        let source = "αβ\nγ";

        assert_eq!(Span::new(2, 2).line_col(source), (1, 2));
        assert_eq!(Span::new(5, 2).line_col(source), (2, 1));
        // Offset 3 is inside beta, so it rounds up to the next boundary — the
        // `\n` at offset 4, which is line 1's third character.
        assert_eq!(Span::new(3, 1).line_col(source), (1, 3));
    }

    #[test]
    fn line_col_treats_carriage_return_as_an_ordinary_character() {
        // `a` 0, `\r` 1, `\n` 2, `b` 3 — only the `\n` ends the line.
        let source = "a\r\nb";

        assert_eq!(Span::new(1, 1).line_col(source), (1, 2));
        assert_eq!(Span::new(2, 1).line_col(source), (1, 3));
        assert_eq!(Span::new(3, 1).line_col(source), (2, 1));
    }

    #[test]
    fn line_col_counts_a_byte_order_mark_as_a_column() {
        // The BOM is three bytes at 0..3, so `a` starts at 3 and reports as the
        // line's SECOND character.
        let source = "\u{feff}ab";

        assert_eq!(Span::new(0, 3).line_col(source), (1, 1));
        assert_eq!(Span::new(3, 1).line_col(source), (1, 2));
        assert_eq!(Span::new(4, 1).line_col(source), (1, 3));
    }
}
