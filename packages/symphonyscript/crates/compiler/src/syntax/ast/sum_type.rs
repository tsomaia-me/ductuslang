use super::support::children;
use super::type_ref::TypeRef;

ast_node!(SumType);

impl<'p> SumType<'p> {
    pub fn alternatives(&self) -> impl Iterator<Item = TypeRef<'p>> + use<'p> {
        children(&self.0)
    }
}
