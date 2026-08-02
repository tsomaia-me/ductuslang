//! The `.duir` token vocabulary: the closed `TokenKind` inventory (R19) and the
//! spanned `Token` the lexer produces and the parser consumes.

// The file is named after the entity it holds (S1), and the entity is `Token`,
// so `token/token.rs` stutters — the same trade `value/value.rs` makes.
// `expect` rather than `allow` so the annotation reports itself as stale if the
// layout ever changes.
#[expect(
    clippy::module_inception,
    reason = "S1 names files after their entity; `Token` lives in `token/`"
)]
pub mod token;

pub mod token_kind;

pub use self::token::Token;
pub use self::token_kind::TokenKind;
