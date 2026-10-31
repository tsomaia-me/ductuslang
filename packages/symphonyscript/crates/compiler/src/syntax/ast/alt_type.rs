use super::support::child;
use super::type_ref::TypeRef;

ast_node!(AltType);

impl<'p> AltType<'p> {
    pub fn ty(&self) -> Option<TypeRef<'p>> {
        child(&self.0)
    }
}
