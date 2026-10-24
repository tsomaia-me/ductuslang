pub(super) enum Mode {
    String { start: usize, depth: u32 },
    Interp { depth: u32 },
}
