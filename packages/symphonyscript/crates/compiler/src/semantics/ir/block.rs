use super::body_item::BodyItem;
use super::unknown_names::UnknownNames;

#[derive(Clone, PartialEq, Debug, Default)]
pub(crate) struct Block {
    pub items: Vec<BodyItem>,
    pub unknown: UnknownNames,
}
