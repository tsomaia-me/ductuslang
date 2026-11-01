use std::rc::Rc;

#[derive(Clone, Debug)]
pub(crate) enum Lay {
    Plain,
    At(Rc<[u32]>),
    Set(u32),
}
