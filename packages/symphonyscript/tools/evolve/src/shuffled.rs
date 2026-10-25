pub fn shuffled<T: Clone>(items: &[T], seed: u64) -> Vec<T> {
    let mut state = seed;
    let mut result = items.to_vec();
    for upper in (1..result.len()).rev() {
        state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut mixed = state;
        mixed = (mixed ^ (mixed >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        mixed = (mixed ^ (mixed >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        mixed ^= mixed >> 31;
        let bound = u64::try_from(upper + 1).unwrap_or(u64::MAX);
        let pick = usize::try_from(mixed % bound).unwrap_or(0);
        result.swap(upper, pick);
    }
    result
}
