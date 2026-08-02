//! The `Token` struct: a kind plus the source it was read from.

use super::token_kind::TokenKind;
use crate::diag::Span;

/// One token: what it is, and where in the source it came from.
///
/// The span is `diag::Span`, not a text-layer twin (R32) — the text layer
/// depends on `diag`, never the reverse, so a token can be pointed at by a
/// diagnostic with no conversion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    /// What the token is, with its payload.
    pub kind: TokenKind,
    /// The byte range of the token in the source text.
    pub span: Span,
}

impl Token {
    /// A token of `kind` covering `span`.
    ///
    /// The fields stay public — literal construction reads well in tests — so
    /// this is additive convenience, not an encapsulation boundary.
    #[must_use]
    pub fn new(kind: TokenKind, span: Span) -> Token {
        Token { kind, span }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_carries_kind_and_span() {
        let token = Token::new(TokenKind::Ident("count".to_string()), Span::new(6, 5));

        assert_eq!(token.kind, TokenKind::Ident("count".to_string()));
        assert_eq!(token.span, Span::new(6, 5));
        assert_eq!(token.span.end(), 11);
    }

    #[test]
    fn tokens_compare_on_both_kind_and_span() {
        let token = Token::new(TokenKind::Dot, Span::new(9, 1));

        assert_eq!(token, Token::new(TokenKind::Dot, Span::new(9, 1)));
        assert_ne!(token, Token::new(TokenKind::Dot, Span::new(3, 1)));
        assert_ne!(token, Token::new(TokenKind::Comma, Span::new(9, 1)));
    }
}
