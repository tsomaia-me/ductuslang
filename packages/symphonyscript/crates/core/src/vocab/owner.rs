#[derive(Clone, Copy)]
pub(crate) enum Owner {
    Node(usize),
    Wire(usize),
}
