use super::core_value::CoreValue;
use super::fact::Fact;
use crate::diagnostics::Diagnostic;

pub(crate) struct BodyInstance {
    pub function: u32,
    pub parent: u32,
    pub args: Vec<Fact>,
    pub reactive_args: Vec<bool>,
    pub diagnostics: Vec<Diagnostic>,
    pub values: Vec<CoreValue>,
}
