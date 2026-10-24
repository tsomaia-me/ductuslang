use alloc::string::String;
use core::ops::Range;

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Span {
    pub file: String,
    pub range: Range<u32>,
}
