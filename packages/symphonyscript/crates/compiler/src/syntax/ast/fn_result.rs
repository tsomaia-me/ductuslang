use super::support::child;
use super::r#type::Type;

ast_node!(FnResult);

impl<'p> FnResult<'p> {
    pub fn ty(&self) -> Option<Type<'p>> {
        child(&self.0)
    }
}
