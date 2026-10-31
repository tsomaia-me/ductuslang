use super::name::Name;
use super::support::name_at;

ast_node!(QualifiedName);

impl<'p> QualifiedName<'p> {
    pub fn package(&self) -> Option<Name<'p>> {
        name_at(&self.0, 0)
    }

    pub fn name(&self) -> Option<Name<'p>> {
        name_at(&self.0, 1)
    }
}
