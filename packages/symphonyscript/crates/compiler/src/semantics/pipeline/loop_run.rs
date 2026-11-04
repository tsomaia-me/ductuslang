use super::core_value::CoreValue;
use super::fact::Fact;

#[derive(Clone, Debug)]
pub(crate) struct LoopRun {
    pub item: u32,
    pub items: CoreValue,
    pub count: usize,
    pub facts: Vec<Fact>,
    pub next: usize,
    pub env: u32,
    pub inst: u32,
    pub open: bool,
}

impl LoopRun {
    pub fn unrun(item: u32, env: u32) -> LoopRun {
        LoopRun {
            item,
            items: CoreValue::Error,
            count: 0,
            facts: Vec::new(),
            next: 0,
            env,
            inst: crate::semantics::ir::flat::NONE,
            open: true,
        }
    }
}
