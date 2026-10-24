use super::interpolation::Interpolation;
use crate::syntax::TokenView;

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum StringPart<'p> {
    Text(TokenView<'p>),
    Interpolation(Interpolation<'p>),
}
