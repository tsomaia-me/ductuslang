//! Folder-private machinery of the two delimited payloads — the `"…"` string
//! and the `'…'` path key.
//!
//! Shared because ONE grammar governs both (033-249, 033-243, R63): the same
//! raw-scalar rule, the same carriage-return law, the same escape-span law, and
//! the same escape SET with the delimiter substituted. Each recognizer supplies
//! its delimiter and nothing else.

use super::chars::hex_run;
use crate::diag::DiagClass;
use crate::text::lexer::scan_error::ScanError;

/// A delimited lexeme's payload starts one byte past its opening delimiter.
pub(super) const PAYLOAD_START: usize = 1;

/// A Unicode scalar escape is at most 6 hex digits (033-249).
const MAX_SCALAR_DIGITS: usize = 6;

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

// The escape at `cursor` — the ONE escape grammar both payloads run (R63),
// with `delimiter` the only thing that differs: `\"` `\\` `\n` `\r` `\t`
// `\u{1..6 hex}` inside a STRING, and `\'` in place of `\"` inside a QuotedKey.
//
// Answers the cursor past the escape. The other delimiter is NOT an escape:
// `\'` inside a string and `\"` inside a key are both `invalid_escape`, because
// each is an ordinary Raw scalar in that lexeme and needs no escaping.
pub(super) fn push_escape(
    input: &str,
    cursor: usize,
    delimiter: char,
    value: &mut String,
) -> Result<usize, ScanError> {
    let escaped = escaped_character(input, cursor)?;

    let character = match escaped {
        _ if escaped == delimiter => delimiter,
        '\\' => '\\',
        'n' => '\n',
        'r' => '\r',
        't' => '\t',
        'u' => return push_scalar_escape(input, cursor, value),
        _ => {
            return Err(invalid_escape(
                input,
                cursor,
                cursor + 1 + escaped.len_utf8(),
            ));
        }
    };

    value.push(character);

    Ok(cursor + 2)
}

// `\u{H…H}` — 1 to 6 hex digits naming a Unicode scalar. A surrogate or a value
// above 0x10FFFF names no scalar and is rejected here rather than passed on.
fn push_scalar_escape(input: &str, cursor: usize, value: &mut String) -> Result<usize, ScanError> {
    let bytes = input.as_bytes();
    let open = cursor + 2;

    if bytes.get(open) != Some(&b'{') {
        return Err(invalid_escape(input, cursor, open + 1));
    }

    let digits_start = open + 1;
    let digits = hex_run(&bytes[digits_start..]);
    let close = digits_start + digits;

    if !(1..=MAX_SCALAR_DIGITS).contains(&digits) || bytes.get(close) != Some(&b'}') {
        return Err(invalid_escape(input, cursor, close + 1));
    }

    let scalar = u32::from_str_radix(&input[digits_start..close], 16)
        .ok()
        .and_then(char::from_u32)
        .ok_or_else(|| invalid_escape(input, cursor, close + 1))?;

    value.push(scalar);

    Ok(close + 1)
}

// The backslash sits at `cursor` and the escaped character follows it. End of
// input there is a malformed escape, not an unterminated lexeme — the escape is
// the nearer and more actionable error.
fn escaped_character(input: &str, cursor: usize) -> Result<char, ScanError> {
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
