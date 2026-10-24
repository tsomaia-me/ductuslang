#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Role {
    Attribute,
    Gate,
    Initializer,
}
