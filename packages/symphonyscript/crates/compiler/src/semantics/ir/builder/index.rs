pub(super) fn index(length: usize) -> u32 {
    u32::try_from(length).unwrap_or(u32::MAX)
}
