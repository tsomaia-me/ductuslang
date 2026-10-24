use crate::semantics::Internal;
use crate::semantics::analysis::sig::Sig;

pub(super) fn operand(sigs: &[Sig], position: usize) -> Result<Sig, Internal> {
    sigs.get(position)
        .copied()
        .ok_or(Internal::missed("operand signature"))
}
