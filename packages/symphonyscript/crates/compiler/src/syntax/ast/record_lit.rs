use super::field::Field;
use super::support::children;

ast_node!(RecordLit);

impl<'p> RecordLit<'p> {
    pub fn fields(&self) -> impl Iterator<Item = Field<'p>> + use<'p> {
        children(&self.0)
    }
}
