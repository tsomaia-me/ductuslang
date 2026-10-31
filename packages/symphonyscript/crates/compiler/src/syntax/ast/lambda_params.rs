use super::lambda_param::LambdaParam;
use super::support::children;

ast_node!(LambdaParams);

impl<'p> LambdaParams<'p> {
    pub fn params(&self) -> impl Iterator<Item = LambdaParam<'p>> + use<'p> {
        children(&self.0)
    }
}
