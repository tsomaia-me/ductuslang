#[test]
fn a_view_cannot_outlive_its_parse_result() {
    let cases = trybuild::TestCases::new();
    cases.compile_fail("tests/ui/*.rs");
}
