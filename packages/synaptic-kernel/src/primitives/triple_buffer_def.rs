use std::fmt;
use std::fmt::Formatter;

/// Typed identifier for a triple buffer within a `TripleBufferWriterRegistry`.
///
/// User assigned IDs occupy the range `[0, N-1]` where `N` is the registry's
/// const-generic `TB_COUNT`. The sentinel value `DEFAULT` (`u16::MAX`) refers
/// to the kernel-internal triple buffer that holds the `Network` of nodes and synapses.
///
/// # Reserved values
/// - `TripleBufferId::DEFAULT` - the default TB managed by the kernel.
///   Must not appear in user-supplied `TripleBufferDef` arrays.
#[repr(transparent)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TripleBufferId(pub u16);

impl TripleBufferId {
    pub const DEFAULT: TripleBufferId = TripleBufferId(u16::MAX);
}

impl fmt::Display for TripleBufferId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Definition of user-allocated triple buffer within the registry.
///
/// # Fields
/// - `id`: Unique identifier in `[0, N-1]`. Must not equal `TripleBufferId::DEFAULT`.
/// - `buffer_capacity`: Number of `i32` slots per buffer (x3 on MEM).
#[derive(Clone, Copy)]
pub struct TripleBufferDef {
    pub id: TripleBufferId,
    pub buffer_capacity: usize,
}
