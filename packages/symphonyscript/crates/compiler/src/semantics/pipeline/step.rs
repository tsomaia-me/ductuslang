use crate::semantics::operators::EvalTy;

#[derive(Clone, Copy, Debug)]
pub(crate) enum Step {
    Visit(u32, EvalTy),
    Apply(u32, EvalTy),
    Choose(u32, EvalTy),
    Runtime(u32),
    Pick(u32, EvalTy),
    Record(u32, bool, u32, EvalTy),
    Fit(u32, EvalTy),
    Widen(EvalTy),
    Float(u32, EvalTy),
    Enter(u32, EvalTy),
    Fold(u32, EvalTy),
    FoldRun(u32, EvalTy),
}
