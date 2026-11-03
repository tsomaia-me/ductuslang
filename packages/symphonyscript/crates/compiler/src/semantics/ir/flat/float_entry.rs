#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) struct FloatEntry {
    pub negative: bool,
    pub digits: (u32, u32),
    pub exponent: i64,
    pub value: f64,
}
