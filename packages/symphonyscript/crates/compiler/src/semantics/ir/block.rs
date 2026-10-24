use super::body_item::BodyItem;

#[derive(Clone, PartialEq, Debug, Default)]
pub(crate) struct Block {
    pub items: Vec<BodyItem>,
}
