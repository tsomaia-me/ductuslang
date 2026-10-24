use alloc::string::String;
use alloc::vec::Vec;
use core::ops::Range;

use crate::ids::{FamilyId, InputSlot};
use crate::source::Span;
use crate::types::{Ty, Value};

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Input {
    pub name: String,
    pub ty: Ty,
    pub family: Option<FamilyId>,
    pub slots: Range<InputSlot>,
    pub initial: Vec<Value>,
    pub span: Span,
}
