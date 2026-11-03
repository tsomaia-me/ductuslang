use crate::semantics::operators::EvalTy;

#[derive(Clone, Copy, Debug)]
pub(crate) enum Step {
    Visit(u32, EvalTy),
    Apply(u32, EvalTy),
    Choose(u32, u32, EvalTy),
    Pick(u32, EvalTy),
    Record(u32, bool, u32, EvalTy),
    Fit(u32, EvalTy),
}
