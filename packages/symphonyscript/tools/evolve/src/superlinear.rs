use crate::measured::Measured;

pub const SCALING_FLOOR: u64 = 1_000;
const GROWTH_LIMIT: u64 = 8;

pub fn superlinear(once: &Measured, doubled: &Measured, quadrupled: &Measured) -> bool {
    let (base, grown) = (once.work, quadrupled.work);
    base.allocations >= SCALING_FLOOR
        && doubled.distinct_codes == once.distinct_codes
        && quadrupled.distinct_codes == once.distinct_codes
        && (grown.allocations > base.allocations.saturating_mul(GROWTH_LIMIT)
            || grown.peak_bytes > base.peak_bytes.saturating_mul(GROWTH_LIMIT))
}
