use super::expr::Expr;
use super::support::child;

ast_node!(Size);

impl<'p> Size<'p> {
    pub fn value(&self) -> Option<Expr<'p>> {
        child(&self.0)
    }
}
