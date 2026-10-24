use super::interpolation::Interpolation;
use crate::syntax::SyntaxToken;

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum StringPart {
    Text(SyntaxToken),
    Interpolation(Interpolation),
}
