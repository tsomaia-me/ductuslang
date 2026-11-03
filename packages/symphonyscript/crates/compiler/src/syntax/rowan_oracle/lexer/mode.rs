pub(super) enum Mode {
    String { start: usize, depth: u32 },
    Interp { depth: u32 },
    Brace { depth: u32 },
    Json { depth: u32 },
    Code { depth: u32 },
}
