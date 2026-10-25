use serde::Serialize;

use crate::signal::Signal;

#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
pub struct Finding {
    pub signal: Signal,
    pub codes: Vec<String>,
}
