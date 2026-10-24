use crate::syntax::NodeView;

pub trait AstNode<'p>: Sized {
    fn cast(node: NodeView<'p>) -> Option<Self>;
    fn syntax(&self) -> &NodeView<'p>;
}
