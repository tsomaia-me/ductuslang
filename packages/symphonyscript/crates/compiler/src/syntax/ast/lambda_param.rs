use super::name::Name;
use super::support::{child, name_at};
use super::r#type::Type;

ast_node!(LambdaParam);

impl<'p> LambdaParam<'p> {
    pub fn name(&self) -> Option<Name<'p>> {
        name_at(&self.0, 0)
    }

    pub fn ty(&self) -> Option<Type<'p>> {
        child(&self.0)
    }
}
