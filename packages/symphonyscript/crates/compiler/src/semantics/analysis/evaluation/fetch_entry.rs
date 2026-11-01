use std::cell::OnceCell;

use super::fetched::Fetched;
use crate::semantics::analysis::c_val::CVal;

#[derive(Debug)]
pub(crate) struct FetchEntry {
    pub fetched: Fetched,
    pub settled: OnceCell<CVal>,
}
