use super::field_decl::FieldDecl;
use super::generic_params::GenericParams;
use super::name::Name;
use super::support::{child, children, name_at};
use super::r#type::Type;

ast_node!(TypeDecl);

impl<'p> TypeDecl<'p> {
    pub fn name(&self) -> Option<Name<'p>> {
        name_at(&self.0, 0)
    }

    pub fn generic_params(&self) -> Option<GenericParams<'p>> {
        child(&self.0)
    }

    pub fn alias(&self) -> Option<Type<'p>> {
        child(&self.0)
    }

    pub fn fields(&self) -> impl Iterator<Item = FieldDecl<'p>> + use<'p> {
        children(&self.0)
    }
}
