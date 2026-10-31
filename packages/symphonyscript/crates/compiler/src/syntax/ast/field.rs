use super::expr::Expr;
use super::name::Name;
use super::support::{child, name_at};

ast_node!(Field);

impl<'p> Field<'p> {
    pub fn name(&self) -> Option<Name<'p>> {
        name_at(&self.0, 0)
    }

    pub fn value(&self) -> Option<Expr<'p>> {
        child(&self.0)
    }
}
