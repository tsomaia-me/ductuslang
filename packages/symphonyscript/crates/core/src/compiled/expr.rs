use alloc::vec;
use alloc::vec::Vec;
use core::hash::{Hash, Hasher};

use super::expr_link::ExprLink;
use super::expr_node::ExprNode;
use super::expr_ref::ExprRef;

#[derive(Clone, Debug)]
pub struct Expr {
    pub nodes: Vec<ExprNode>,
    pub root: ExprRef,
}

impl Expr {
    pub fn leaf(node: ExprNode) -> Expr {
        Expr {
            nodes: vec![node],
            root: ExprRef(0),
        }
    }

    pub fn push(&mut self, node: ExprNode) -> ExprRef {
        let reference = ExprRef(self.nodes.len() as u32);
        self.nodes.push(node);
        reference
    }

    pub(crate) fn link(&self, parent: Option<usize>, child: ExprRef) -> ExprLink {
        let index = child.index();
        let below_parent = parent.is_none_or(|parent| index < parent);
        if index < self.nodes.len() && below_parent {
            ExprLink::Node(index)
        } else {
            ExprLink::Dangling(child.0)
        }
    }
}

impl PartialEq for Expr {
    fn eq(&self, other: &Self) -> bool {
        let mut pending = vec![(self.link(None, self.root), other.link(None, other.root))];
        while let Some(pair) = pending.pop() {
            match pair {
                (ExprLink::Node(a), ExprLink::Node(b)) => {
                    let (left, right) = (&self.nodes[a], &other.nodes[b]);
                    if !left.same_shape(right) {
                        return false;
                    }
                    let left_children = left.children();
                    let right_children = right.children();
                    for (x, y) in left_children.into_iter().zip(right_children) {
                        pending.push((self.link(Some(a), x), other.link(Some(b), y)));
                    }
                }
                (ExprLink::Dangling(a), ExprLink::Dangling(b)) if a == b => {}
                _ => return false,
            }
        }
        true
    }
}

impl Eq for Expr {}

impl Hash for Expr {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let mut pending = vec![self.link(None, self.root)];
        while let Some(link) = pending.pop() {
            match link {
                ExprLink::Node(index) => {
                    0u8.hash(state);
                    let node = &self.nodes[index];
                    node.hash_shape(state);
                    for child in node.children().into_iter().rev() {
                        pending.push(self.link(Some(index), child));
                    }
                }
                ExprLink::Dangling(reference) => {
                    1u8.hash(state);
                    reference.hash(state);
                }
            }
        }
    }
}
