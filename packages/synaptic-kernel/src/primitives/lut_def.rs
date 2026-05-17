use crate::primitives::triple_buffer_def::TripleBufferId;
use std::fmt;
use std::fmt::Formatter;

/// Typed identifier for a LUT within a `LutRegistryWriter`.
///
/// IDs occupy the range `[0, N-1]` where `N` is the registry's const-generic `N`.
#[repr(transparent)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LutId(pub u16);

impl fmt::Display for LutId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Definition of user-allocated LUT within the registry.
///
/// # Fields
/// - `id`: Unique identifier in `[0, N-1]`.
/// - `tb_id`: TB identifier. It's absolutely valid to use `TripleBufferId::DEFAULT` here.
/// - `size`: LUT size.
#[derive(Clone, Copy)]
pub struct LutDef {
    pub id: LutId,
    pub tb_id: TripleBufferId,
    pub size: usize,
}

impl LutDef {
    pub fn new(id: LutId, tb_id: TripleBufferId, size: usize) -> Self {
        LutDef { id, tb_id, size }
    }

    pub fn len(&self) -> usize {
        self.size
    }
}
