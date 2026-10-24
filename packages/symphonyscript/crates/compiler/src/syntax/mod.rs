mod lex;
mod lexed;
mod lexer;
mod nfc;
mod syntax_kind;
mod token;

pub use lex::lex;
pub use lexed::Lexed;
pub use syntax_kind::SyntaxKind;
pub use token::Token;
