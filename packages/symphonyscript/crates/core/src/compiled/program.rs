use alloc::string::String;
use alloc::vec::Vec;

use super::cell::Cell;
use super::input::Input;
use super::instance::Instance;
use super::wire::Wire;
use crate::ids::InstanceId;
use crate::vocab::VocabularyStamp;

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Program {
    pub vocabulary: VocabularyStamp,
    pub top_level: Vec<InstanceId>,
    pub instances: Vec<Instance>,
    pub wires: Vec<Wire>,
    pub inputs: Vec<Input>,
    pub cells: Vec<Cell>,
    pub strings: Vec<String>,
}
