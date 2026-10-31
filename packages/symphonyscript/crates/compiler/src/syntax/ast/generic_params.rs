use super::generic_param::GenericParam;
use super::support::children;

ast_node!(GenericParams);

impl<'p> GenericParams<'p> {
    pub fn params(&self) -> impl Iterator<Item = GenericParam<'p>> + use<'p> {
        children(&self.0)
    }
}
