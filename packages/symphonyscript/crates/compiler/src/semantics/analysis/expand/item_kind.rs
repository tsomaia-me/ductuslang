use symphonyscript_core::{NodeKind, WireKind};

#[derive(Clone, Copy)]
pub(crate) enum ItemKind {
    Node(NodeKind),
    Wire(WireKind),
}
