use super::shaped::Shaped;

#[derive(Clone)]
pub(crate) struct LayoutFound {
    pub shaped: Shaped,
    pub certain: bool,
}
