use super::name::Name;
use super::support::{child, name_at, token};
use super::r#type::Type;
use crate::syntax::SyntaxKind;

ast_node!(GenericParam);

impl<'p> GenericParam<'p> {
    pub fn is_const(&self) -> bool {
        token(&self.0, SyntaxKind::ConstKw).is_some()
    }

    pub fn name(&self) -> Option<Name<'p>> {
        name_at(&self.0, 0)
    }

    pub fn constraint(&self) -> Option<Type<'p>> {
        child(&self.0)
    }
}
