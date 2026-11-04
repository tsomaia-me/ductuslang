#[derive(Clone, Copy, Debug)]
pub(crate) enum ArrayNode {
    Dense { layout: u32, start: u32, len: u32 },
    Join { left: u32, right: u32, len: u32 },
    Slice { base: u32, start: u32, len: u32 },
}
