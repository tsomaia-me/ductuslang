//! The `TypeDef` struct: one row of a module's type table.

use crate::diag::Span;
use crate::types::Layout;

/// One entry of the `types` table — the `type_def` production (IR_GRAMMAR §3,
/// §15.4.6):
///
/// ```text
/// type_def ::= '%' NAME '=' layout 'size' INT 'align' INT
/// ```
///
/// `size` and `align` are STATED, not computed. The text form is a
/// serialization of a lowering that already knows the target ABI, so the layout
/// beside them is structure and these two are the memory facts; nothing here
/// re-derives one from the other, and a mismatch between them is a loader
/// question (F4).
///
/// Both are STRUCTURAL numbers (R57): the parser converts them to `u64` at the
/// literal, reporting `int_out_of_range` on a signed or oversized spelling.
/// That is the opposite treatment from a TYPED value such as a cell's `init`,
/// which stays raw text until the loader converts it under the cell's type —
/// the difference being that a structural number's type is known here and a
/// typed value's is not.
///
/// No `PartialEq` (R17): sameness is emitted-text equality, and a derived
/// equality would fold the span into it.
#[derive(Debug, Clone)]
pub struct TypeDef {
    /// The type's NAME — what follows the `%` sigil, without it.
    pub id: String,
    /// The `%` through the `align` literal's last byte (R17).
    pub span: Span,
    /// The layout: record, enum, or tuple.
    pub layout: Layout,
    /// The stated size in bytes.
    pub size: u64,
    /// The stated alignment in bytes.
    pub align: u64,
}
