use crate::semantics::analysis::history_cell::HistoryCell;
use crate::semantics::analysis::ids::{BindingId, NodeId};

pub(crate) enum CellRead {
    Binding(BindingId),
    History {
        cell: HistoryCell,
        fallback: Option<NodeId>,
    },
    Named(NodeId),
    Operands(Vec<NodeId>),
}
