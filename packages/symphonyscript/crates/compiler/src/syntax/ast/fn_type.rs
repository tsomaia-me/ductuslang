use super::fn_param::FnParam;
use super::support::{child, children};
use super::r#type::Type;

ast_node!(FnType);

impl<'p> FnType<'p> {
    pub fn params(&self) -> impl Iterator<Item = FnParam<'p>> + use<'p> {
        children(&self.0)
    }

    pub fn result(&self) -> Option<Type<'p>> {
        child(&self.0)
    }
}
