use super::expr::Expr;
use super::lambda_params::LambdaParams;
use super::support::child;

ast_node!(Lambda);

impl<'p> Lambda<'p> {
    pub fn params(&self) -> Option<LambdaParams<'p>> {
        child(&self.0)
    }

    pub fn body(&self) -> Option<Expr<'p>> {
        child(&self.0)
    }
}
