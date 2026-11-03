use super::element_view::ElementView;

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum Walk<'p> {
    Enter(ElementView<'p>),
    Leave(ElementView<'p>),
}
