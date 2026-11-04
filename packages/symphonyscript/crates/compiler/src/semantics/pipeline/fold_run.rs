use super::core_value::CoreValue;

pub(crate) struct FoldRun {
    pub node: u32,
    pub items: CoreValue,
    pub next: usize,
    pub count: usize,
    pub indexed: bool,
}
