use std::collections::BTreeMap;
use std::rc::Rc;

use super::local_name::LocalName;
use super::locals_owner::LocalsOwner;
use super::param_source::ParamSource;

#[derive(Clone, PartialEq, Debug)]
pub(crate) struct Locals {
    pub owner: LocalsOwner,
    pub params: Vec<ParamSource>,
    pub names: Rc<BTreeMap<String, LocalName>>,
    pub recursive: bool,
}
