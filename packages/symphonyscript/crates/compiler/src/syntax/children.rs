use super::child_list::ChildList;
use super::element_view::ElementView;
use super::node_view::NodeView;
use super::parsed::Parsed;
use super::token_view::TokenView;

pub(crate) struct Children {
    reversed: ChildList,
}

impl Children {
    pub(crate) fn of(parsed: &Parsed, node: u32) -> Self {
        let tree = parsed.tree();
        let mut reversed = ChildList::new();
        let mut child = tree.last_child(node);
        while let Some(found) = child {
            reversed.push(found);
            child = tree.previous_sibling(node, found);
        }
        Self { reversed }
    }
}

impl Iterator for Children {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        self.reversed.pop()
    }
}

pub(crate) struct Elements<'p> {
    parsed: &'p Parsed,
    children: Children,
    pending: Option<u32>,
    token: u32,
    end: u32,
}

impl<'p> Elements<'p> {
    pub(crate) fn of(parsed: &'p Parsed, node: u32) -> Self {
        let (token, end) = parsed.tree().tokens(node);
        let mut children = Children::of(parsed, node);
        let pending = children.next();
        Self {
            parsed,
            children,
            pending,
            token,
            end,
        }
    }
}

impl<'p> Iterator for Elements<'p> {
    type Item = ElementView<'p>;

    fn next(&mut self) -> Option<ElementView<'p>> {
        let limit = match self.pending {
            Some(child) => self.parsed.tree().tokens(child).0,
            None => self.end,
        };
        if self.token < limit {
            let token = TokenView::new(self.parsed, self.token);
            self.token += 1;
            return Some(ElementView::Token(token));
        }
        let child = self.pending?;
        self.token = self.parsed.tree().tokens(child).1;
        self.pending = self.children.next();
        Some(ElementView::Node(NodeView::new(self.parsed, child)))
    }
}
