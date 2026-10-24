pub fn canonical_f64(value: f64) -> f64 {
    if value.is_nan() {
        f64::from_bits(0x7FF8_0000_0000_0000)
    } else {
        value
    }
}
