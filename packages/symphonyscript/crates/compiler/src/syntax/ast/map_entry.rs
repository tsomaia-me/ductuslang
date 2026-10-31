use super::expr::Expr;
use super::support::children;

ast_node!(MapEntry);

impl<'p> MapEntry<'p> {
    pub fn key_and_value(&self) -> impl Iterator<Item = Expr<'p>> + use<'p> {
        children(&self.0)
    }
}
