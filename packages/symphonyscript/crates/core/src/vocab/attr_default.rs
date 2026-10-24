use alloc::string::String;
use alloc::vec::Vec;

use crate::types::Value;

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum AttrDefault {
    Elements(Vec<Value>),
    Text(String),
}
