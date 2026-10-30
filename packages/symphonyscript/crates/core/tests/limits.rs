use symphonyscript_core::{LimitOverrides, Limits};

#[test]
fn the_default_limits_are_the_specs_and_each_warning_is_half_its_limit() {
    assert_eq!(
        Limits::default(),
        Limits {
            recursive_calls: 1_000_000,
            recursive_calls_warning: 500_000,
            nesting_depth: 1_024,
            nesting_depth_warning: 512,
            inlined_calls: 1_000_000,
            inlined_calls_warning: 500_000,
            expansion_steps: 1_000_000,
            expansion_steps_warning: 500_000,
            expanded_items: 1_000_000,
            expanded_items_warning: 500_000,
            memory_bytes: 1_073_741_824,
            memory_bytes_warning: 536_870_912,
            reload_edit_distance: 1_000,
            shared_body_threshold: 1_000,
        }
    );
}

#[test]
fn an_override_sets_its_one_field_and_leaves_every_other_at_its_default() {
    let unset = LimitOverrides::default();
    let default = Limits::default();
    assert_eq!(unset.applied_to(default), default);
    let rows = [
        (
            LimitOverrides {
                recursive_calls: Some(7),
                ..unset
            },
            Limits {
                recursive_calls: 7,
                ..default
            },
        ),
        (
            LimitOverrides {
                recursive_calls_warning: Some(7),
                ..unset
            },
            Limits {
                recursive_calls_warning: 7,
                ..default
            },
        ),
        (
            LimitOverrides {
                nesting_depth: Some(7),
                ..unset
            },
            Limits {
                nesting_depth: 7,
                ..default
            },
        ),
        (
            LimitOverrides {
                nesting_depth_warning: Some(7),
                ..unset
            },
            Limits {
                nesting_depth_warning: 7,
                ..default
            },
        ),
        (
            LimitOverrides {
                inlined_calls: Some(7),
                ..unset
            },
            Limits {
                inlined_calls: 7,
                ..default
            },
        ),
        (
            LimitOverrides {
                inlined_calls_warning: Some(7),
                ..unset
            },
            Limits {
                inlined_calls_warning: 7,
                ..default
            },
        ),
        (
            LimitOverrides {
                expansion_steps: Some(7),
                ..unset
            },
            Limits {
                expansion_steps: 7,
                ..default
            },
        ),
        (
            LimitOverrides {
                expansion_steps_warning: Some(7),
                ..unset
            },
            Limits {
                expansion_steps_warning: 7,
                ..default
            },
        ),
        (
            LimitOverrides {
                expanded_items: Some(7),
                ..unset
            },
            Limits {
                expanded_items: 7,
                ..default
            },
        ),
        (
            LimitOverrides {
                expanded_items_warning: Some(7),
                ..unset
            },
            Limits {
                expanded_items_warning: 7,
                ..default
            },
        ),
        (
            LimitOverrides {
                memory_bytes: Some(7),
                ..unset
            },
            Limits {
                memory_bytes: 7,
                ..default
            },
        ),
        (
            LimitOverrides {
                memory_bytes_warning: Some(7),
                ..unset
            },
            Limits {
                memory_bytes_warning: 7,
                ..default
            },
        ),
        (
            LimitOverrides {
                reload_edit_distance: Some(7),
                ..unset
            },
            Limits {
                reload_edit_distance: 7,
                ..default
            },
        ),
        (
            LimitOverrides {
                shared_body_threshold: Some(7),
                ..unset
            },
            Limits {
                shared_body_threshold: 7,
                ..default
            },
        ),
    ];
    for (overrides, expected) in rows {
        assert_eq!(overrides.applied_to(default), expected, "{overrides:?}");
    }
}
