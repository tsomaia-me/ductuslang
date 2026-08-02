//! The punctuation recognizer: the marks of the module (§3) and behavior (§4)
//! grammars.

use crate::text::token::TokenKind;

/// A punctuation mark of the module (§3) and behavior (§4) grammars.
///
/// `->` is the only two-byte mark. `>` is always taken singly, so a nested type
/// tag (`portal<handle<i32>>`) closes with two `Gt`s — there is no shift token
/// to munch them into.
pub(crate) fn punctuation(input: &str) -> Option<(usize, TokenKind)> {
    if input.starts_with("->") {
        return Some((2, TokenKind::Arrow));
    }

    let byte = *input.as_bytes().first()?;

    let kind = match byte {
        b'{' => TokenKind::LBrace,
        b'}' => TokenKind::RBrace,
        b'[' => TokenKind::LBracket,
        b']' => TokenKind::RBracket,
        b'(' => TokenKind::LParen,
        b')' => TokenKind::RParen,
        b'<' => TokenKind::Lt,
        b'>' => TokenKind::Gt,
        b':' => TokenKind::Colon,
        b',' => TokenKind::Comma,
        b'.' => TokenKind::Dot,
        b';' => TokenKind::Semicolon,
        b'=' => TokenKind::Eq,
        b'%' => TokenKind::Percent,
        b'#' => TokenKind::Hash,
        b'@' => TokenKind::At,
        _ => return None,
    };

    Some((1, kind))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn punctuation_takes_every_mark_of_the_inventory() {
        let marks = [
            ("{", TokenKind::LBrace),
            ("}", TokenKind::RBrace),
            ("[", TokenKind::LBracket),
            ("]", TokenKind::RBracket),
            ("(", TokenKind::LParen),
            (")", TokenKind::RParen),
            ("<", TokenKind::Lt),
            (">", TokenKind::Gt),
            (":", TokenKind::Colon),
            (",", TokenKind::Comma),
            (".", TokenKind::Dot),
            (";", TokenKind::Semicolon),
            ("=", TokenKind::Eq),
            ("%", TokenKind::Percent),
            ("#", TokenKind::Hash),
            ("@", TokenKind::At),
        ];

        for (text, kind) in marks {
            assert_eq!(punctuation(text), Some((1, kind)));
        }
    }

    #[test]
    fn punctuation_takes_the_arrow_whole_and_never_munches_two_gts() {
        assert_eq!(punctuation("-> i32"), Some((2, TokenKind::Arrow)));
        assert_eq!(punctuation(">> uses"), Some((1, TokenKind::Gt)));
        assert_eq!(punctuation("-5"), None);
        assert_eq!(punctuation("count"), None);
        assert_eq!(punctuation(""), None);
    }
}
