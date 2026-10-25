use super::grouped;

#[test]
fn a_bound_prints_with_its_digits_grouped_by_thousands() {
    for (value, expected) in [
        (0, "0"),
        (10, "10"),
        (999, "999"),
        (1_000, "1,000"),
        (1_024, "1,024"),
        (99_999, "99,999"),
        (100_000, "100,000"),
        (1_000_000, "1,000,000"),
        (2_000_000, "2,000,000"),
        (4_294_967_296, "4,294,967,296"),
        (u64::MAX, "18,446,744,073,709,551,615"),
    ] {
        assert_eq!(grouped(value), expected, "{value}");
    }
}
