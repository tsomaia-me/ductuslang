#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum AttrForm {
    Value(u32),
    Set(bool),
}
