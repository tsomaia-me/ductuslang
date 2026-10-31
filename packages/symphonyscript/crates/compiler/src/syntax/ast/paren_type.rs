use super::support::child;
use super::r#type::Type;

ast_node!(ParenType);

impl<'p> ParenType<'p> {
    pub fn inner(&self) -> Option<Type<'p>> {
        child(&self.0)
    }
}
