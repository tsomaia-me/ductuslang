use super::default_limits::{
    EXPANDED_ITEMS, EXPANDED_ITEMS_WARNING, EXPANSION_STEPS, EXPANSION_STEPS_WARNING,
    INLINED_CALLS, INLINED_CALLS_WARNING, MEMORY_BYTES, MEMORY_BYTES_WARNING, NESTING_DEPTH,
    NESTING_DEPTH_WARNING, RECURSIVE_CALLS, RECURSIVE_CALLS_WARNING, RELOAD_EDIT_DISTANCE,
    SHARED_BODY_THRESHOLD,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Limits {
    pub recursive_calls: u64,
    pub recursive_calls_warning: u64,
    pub nesting_depth: u64,
    pub nesting_depth_warning: u64,
    pub inlined_calls: u64,
    pub inlined_calls_warning: u64,
    pub expansion_steps: u64,
    pub expansion_steps_warning: u64,
    pub expanded_items: u64,
    pub expanded_items_warning: u64,
    pub memory_bytes: u64,
    pub memory_bytes_warning: u64,
    pub reload_edit_distance: u64,
    pub shared_body_threshold: u64,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            recursive_calls: RECURSIVE_CALLS,
            recursive_calls_warning: RECURSIVE_CALLS_WARNING,
            nesting_depth: NESTING_DEPTH,
            nesting_depth_warning: NESTING_DEPTH_WARNING,
            inlined_calls: INLINED_CALLS,
            inlined_calls_warning: INLINED_CALLS_WARNING,
            expansion_steps: EXPANSION_STEPS,
            expansion_steps_warning: EXPANSION_STEPS_WARNING,
            expanded_items: EXPANDED_ITEMS,
            expanded_items_warning: EXPANDED_ITEMS_WARNING,
            memory_bytes: MEMORY_BYTES,
            memory_bytes_warning: MEMORY_BYTES_WARNING,
            reload_edit_distance: RELOAD_EDIT_DISTANCE,
            shared_body_threshold: SHARED_BODY_THRESHOLD,
        }
    }
}
