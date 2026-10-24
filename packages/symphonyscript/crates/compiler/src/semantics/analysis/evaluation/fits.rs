pub(super) fn fits(value: i128, bits: u32) -> bool {
    let max = (1i128 << (bits - 1)) - 1;
    let min = -(1i128 << (bits - 1));
    (min..=max).contains(&value)
}
