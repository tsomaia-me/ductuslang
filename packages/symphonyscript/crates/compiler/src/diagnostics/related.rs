use symphonyscript_core::Span;

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Related {
    pub span: Span,
    pub label: String,
}
