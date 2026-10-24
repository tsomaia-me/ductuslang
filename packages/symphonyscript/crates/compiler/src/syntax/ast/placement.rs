use super::ast_node::AstNode;
use super::attrs::Attrs;
use super::body::Body;
use super::default_arg::DefaultArg;
use super::flags::Flags;
use super::gate::Gate;
use super::name::Name;
use super::support::{child, name_at, token};
use crate::syntax::{NodeView, SyntaxKind};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Placement<'p>(NodeView<'p>);

impl<'p> AstNode<'p> for Placement<'p> {
    fn cast(node: NodeView<'p>) -> Option<Self> {
        (node.kind() == SyntaxKind::Placement).then_some(Self(node))
    }

    fn syntax(&self) -> &NodeView<'p> {
        &self.0
    }
}

impl<'p> Placement<'p> {
    pub fn kind(&self) -> Option<Name<'p>> {
        name_at(&self.0, 0)
    }

    pub fn flags(&self) -> Option<Flags<'p>> {
        child(&self.0)
    }

    pub fn has_as(&self) -> bool {
        token(&self.0, SyntaxKind::AsKw).is_some()
    }

    pub fn name(&self) -> Option<Name<'p>> {
        name_at(&self.0, 1)
    }

    pub fn default_arg(&self) -> Option<DefaultArg<'p>> {
        child(&self.0)
    }

    pub fn gate(&self) -> Option<Gate<'p>> {
        child(&self.0)
    }

    pub fn attrs(&self) -> Option<Attrs<'p>> {
        child(&self.0)
    }

    pub fn body(&self) -> Option<Body<'p>> {
        child(&self.0)
    }
}
