use super::support::children;
use super::r#type::Type;

ast_node!(MapType);

impl<'p> MapType<'p> {
    pub fn key_and_value(&self) -> impl Iterator<Item = Type<'p>> + use<'p> {
        children(&self.0)
    }
}
