use super::support::children;
use super::use_name::UseName;

ast_node!(Use);

impl<'p> Use<'p> {
    pub fn names(&self) -> impl Iterator<Item = UseName<'p>> + use<'p> {
        children(&self.0)
    }
}
