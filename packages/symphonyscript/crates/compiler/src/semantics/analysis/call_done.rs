use super::c_val::CVal;
use super::evaluation::Mode;
use super::ids::CallId;

pub(crate) struct CallDone {
    pub value: CVal,
    pub mode: Mode,
    pub opened: Vec<CallId>,
    pub deep: bool,
    pub callees: Vec<CallId>,
}
