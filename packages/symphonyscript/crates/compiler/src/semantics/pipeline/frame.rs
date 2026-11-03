use super::core_value::CoreValue;
use super::step::Step;
use crate::semantics::operators::EvalTy;

pub(crate) struct Frame {
    pub owner: Option<u32>,
    pub ty: EvalTy,
    pub steps: Vec<Step>,
    pub values: Vec<CoreValue>,
    pub call: Option<(u32, Vec<CoreValue>)>,
    pub inst: u32,
}
