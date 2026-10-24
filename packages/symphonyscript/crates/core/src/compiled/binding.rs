use alloc::vec::Vec;

use crate::ids::{CellId, InputSlot};
use crate::types::Value;

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Binding {
    Const(Value),
    Input(InputSlot),
    Cell(CellId),
    Array(Vec<Binding>),
}
