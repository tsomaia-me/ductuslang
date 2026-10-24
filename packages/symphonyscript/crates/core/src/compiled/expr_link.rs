#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum ExprLink {
    Node(usize),
    Dangling(u32),
}
