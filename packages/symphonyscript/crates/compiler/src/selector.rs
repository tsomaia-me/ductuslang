#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Selector {
    OldEngine,
    NewCore,
    #[default]
    Default,
}
