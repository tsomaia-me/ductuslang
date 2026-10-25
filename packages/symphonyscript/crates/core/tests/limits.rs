use symphonyscript_core::Limits;

#[test]
fn the_default_limits_are_the_specs() {
    assert_eq!(
        Limits::default(),
        Limits {
            recursive_calls: 1_000_000,
            nesting_depth: 1_024,
            inlined_calls: 1_000_000,
            expansion_steps: 1_000_000,
            expanded_items: 1_000_000,
            reload_edit_distance: 1_000,
        }
    );
}
