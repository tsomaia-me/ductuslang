//! Folder-private machinery of the two delimited payloads — the `"…"` string
//! and the `'…'` path key.
//!
//! Shared because one carriage-return law and one escape-span law govern both
//! (033-249); the escape SETS differ, so each recognizer keeps its own.

use crate::diag::DiagClass;
use crate::text::lexer::scan_error::ScanError;

/// A delimited lexeme's payload starts one byte past its opening delimiter.
pub(super) const PAYLOAD_START: usize = 1;

// Copy the raw run starting at `cursor` in ONE `push_str` and answer the cursor
// it ends at — the byte that needs interpreting, or the end of input.
pub(super) fn take_raw_run(
    input: &str,
    cursor: usize,
    delimiter: char,
    value: &mut String,
) -> usize {
    let end = cursor + raw_run(input, cursor, delimiter);
    value.push_str(&input[cursor..end]);

    end
}

// The raw run at `from`: every byte up to the next one carrying meaning — the
// closing delimiter, a backslash, or a carriage return.
//
// A payload's raw content is contiguous source, so it copies wholesale; the
// caller also sizes its buffer by the FIRST run, which makes the escape-free
// payload — nearly every payload — one allocation of exactly its own length
// rather than one scaling with the rest of the file.
pub(super) fn raw_run(input: &str, from: usize, delimiter: char) -> usize {
    input[from..]
        .find([delimiter, '\\', '\r'])
        .unwrap_or(input.len() - from)
}

// CRLF normalizes to LF inside a delimited lexeme exactly as it does outside
// one; a lone `\r` is an error anywhere.
pub(super) fn push_line_ending(
    input: &str,
    cursor: usize,
    value: &mut String,
) -> Result<usize, ScanError> {
    if input.as_bytes().get(cursor + 1) != Some(&b'\n') {
        return Err(ScanError::new(
            DiagClass::LoneCarriageReturn,
            cursor,
            1,
            "a lone carriage return is not a line ending",
        ));
    }

    value.push('\n');

    Ok(cursor + 2)
}

// The backslash sits at `cursor` and the escaped character follows it. End of
// input there is a malformed escape, not an unterminated lexeme — the escape is
// the nearer and more actionable error.
pub(super) fn escaped_character(input: &str, cursor: usize) -> Result<char, ScanError> {
    input[cursor + 1..]
        .chars()
        .next()
        .ok_or_else(|| invalid_escape(input, cursor, cursor + 1))
}

// An escape's span runs from its backslash to `end`, clamped to the input so a
// truncated escape never points past the source.
pub(super) fn invalid_escape(input: &str, cursor: usize, end: usize) -> ScanError {
    ScanError::new(
        DiagClass::InvalidEscape,
        cursor,
        end.min(input.len()) - cursor,
        "escape sequence is not one this position admits",
    )
}
