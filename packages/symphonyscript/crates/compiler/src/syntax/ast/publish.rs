use super::publish_name::PublishName;
use super::support::children;

ast_node!(Publish);

impl<'p> Publish<'p> {
    pub fn names(&self) -> impl Iterator<Item = PublishName<'p>> + use<'p> {
        children(&self.0)
    }
}
