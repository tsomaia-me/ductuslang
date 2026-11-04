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
}
