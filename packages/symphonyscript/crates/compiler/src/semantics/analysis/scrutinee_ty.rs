use super::engine::Engine;
use super::eval_ty::EvalTy;
use super::evaluation::own_type;
use super::ids::NodeId;
use super::kind::Kind;
use super::sig::Sig;

impl Engine<'_> {
    pub(super) fn scrutinee_ty(&self, scrutinee: NodeId, sig: &Sig) -> Option<EvalTy> {
        if sig.kind != Kind::Bare {
            return Some(own_type(sig, EvalTy::Exact));
        }
        self.memo
            .bare_scrutinees
            .get(&scrutinee)
            .map(|id| EvalTy::Enum(*id))
    }
}
