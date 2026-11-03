use super::element_view::ElementView;
use super::node_view::NodeView;
use super::parsed::Parsed;
use super::token_view::TokenView;
use super::walk::Walk;

enum Step {
    Enter(u32),
    Tokens(u32, u32),
    LeaveToken(u32),
    Leave(u32),
}

pub(crate) struct Preorder<'p> {
    parsed: &'p Parsed,
    pending: Vec<Step>,
}

impl<'p> Preorder<'p> {
    pub(crate) fn of(parsed: &'p Parsed, node: u32) -> Self {
        Self {
            parsed,
            pending: vec![Step::Enter(node)],
        }
    }

    fn enter(&mut self, node: u32) {
        let tree = self.parsed.tree();
        let (first, mut end) = tree.tokens(node);
        self.pending.push(Step::Leave(node));
        let mut child = tree.last_child(node);
        while let Some(found) = child {
            let (child_first, child_end) = tree.tokens(found);
            if child_end < end {
                self.pending.push(Step::Tokens(child_end, end));
            }
            self.pending.push(Step::Enter(found));
            end = child_first;
            child = tree.previous_sibling(node, found);
        }
        if first < end {
            self.pending.push(Step::Tokens(first, end));
        }
    }
}

impl<'p> Iterator for Preorder<'p> {
    type Item = Walk<'p>;

    fn next(&mut self) -> Option<Walk<'p>> {
        let parsed = self.parsed;
        Some(match self.pending.pop()? {
            Step::Enter(node) => {
                self.enter(node);
                Walk::Enter(ElementView::Node(NodeView::new(parsed, node)))
            }
            Step::Leave(node) => Walk::Leave(ElementView::Node(NodeView::new(parsed, node))),
            Step::Tokens(token, end) => {
                if token + 1 < end {
                    self.pending.push(Step::Tokens(token + 1, end));
                }
                self.pending.push(Step::LeaveToken(token));
                Walk::Enter(ElementView::Token(TokenView::new(parsed, token)))
            }
            Step::LeaveToken(token) => {
                Walk::Leave(ElementView::Token(TokenView::new(parsed, token)))
            }
        })
    }
}
