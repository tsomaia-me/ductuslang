use symphonyscript_core::WireKind;

#[derive(Clone, Copy, Debug)]
pub(crate) struct WireRow {
    pub kind: WireKind,
    pub item: u32,
    pub file: u32,
    pub source: u32,
    pub candidates: (u32, u32),
    pub fixed: bool,
    pub chosen: bool,
    pub env: u32,
}
