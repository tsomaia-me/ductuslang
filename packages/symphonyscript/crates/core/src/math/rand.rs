pub fn rand(seed: i64, n: i64) -> f32 {
    let x = (seed as u64).wrapping_shl(32).wrapping_add(n as u64);
    let mut z = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^= z >> 31;
    let top = u32::try_from(z >> 40).unwrap_or(0);
    top as f32 / 16_777_216.0
}
