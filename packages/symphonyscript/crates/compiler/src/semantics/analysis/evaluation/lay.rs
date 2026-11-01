use std::rc::Rc;

#[derive(Clone, Debug)]
pub(crate) enum Lay {
    Plain,
    At(Rc<Vec<u32>>),
    Set(u32),
}
