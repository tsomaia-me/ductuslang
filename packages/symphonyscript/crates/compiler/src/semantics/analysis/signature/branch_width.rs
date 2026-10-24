use super::branch_width_of::branch_width_of;
use crate::semantics::analysis::kind::Kind;
use crate::semantics::analysis::sig::Sig;
use crate::semantics::analysis::width::Width;

pub(super) fn branch_width(kind: Kind, a: &Sig, b: &Sig) -> Width {
    branch_width_of(kind, a.width, b.width)
}
