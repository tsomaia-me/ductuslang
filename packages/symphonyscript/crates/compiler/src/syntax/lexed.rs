use super::token::Token;
use crate::diagnostics::Diagnostic;

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Lexed {
    pub tokens: Vec<Token>,
    pub diagnostics: Vec<Diagnostic>,
}
