use super::core_value::CoreValue;

#[derive(Clone, Debug)]
pub(crate) struct Env {
    pub parent: u32,
    pub inst: u32,
    pub values: Vec<CoreValue>,
}
