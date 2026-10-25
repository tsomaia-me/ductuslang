pub const DEFAULT_MEMORY_BOUND: u64 = 50_000;
pub const MEMORY_FLOOR_BYTES: u64 = 64 * 1024 * 1024;

pub fn over_memory_bound(peak_bytes: u64, source_bytes: usize, bytes_per_source_byte: u64) -> bool {
    let source_bytes = u64::try_from(source_bytes).unwrap_or(u64::MAX);
    peak_bytes
        > source_bytes
            .saturating_mul(bytes_per_source_byte)
            .max(MEMORY_FLOOR_BYTES)
}
