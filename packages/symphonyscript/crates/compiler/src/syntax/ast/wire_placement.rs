use super::ast_node::AstNode;
use super::attrs::Attrs;
use super::default_arg::DefaultArg;
use super::destination::Destination;
use super::flags::Flags;
use super::gate::Gate;
use super::name::Name;
use super::support::{child, name_at, token};
use crate::syntax::{SyntaxKind, SyntaxNode};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct WirePlacement(SyntaxNode);

impl AstNode for WirePlacement {
    fn cast(node: SyntaxNode) -> Option<Self> {
        (node.kind() == SyntaxKind::WirePlacement).then_some(Self(node))
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl WirePlacement {
    pub fn kind(&self) -> Option<Name> {
        name_at(&self.0, 0)
    }

    pub fn flags(&self) -> Option<Flags> {
        child(&self.0)
    }

    pub fn has_as(&self) -> bool {
        token(&self.0, SyntaxKind::AsKw).is_some()
    }

    pub fn name(&self) -> Option<Name> {
        name_at(&self.0, 1)
    }

    pub fn default_arg(&self) -> Option<DefaultArg> {
        child(&self.0)
    }

    pub fn gate(&self) -> Option<Gate> {
        child(&self.0)
    }

    pub fn attrs(&self) -> Option<Attrs> {
        child(&self.0)
    }

    pub fn destination(&self) -> Option<Destination> {
        child(&self.0)
    }
}
