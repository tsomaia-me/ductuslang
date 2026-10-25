use super::default_limits::{
    EXPANDED_ITEMS, EXPANSION_STEPS, INLINED_CALLS, NESTING_DEPTH, RECURSIVE_CALLS,
    RELOAD_EDIT_DISTANCE,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Limits {
    pub recursive_calls: u64,
    pub nesting_depth: u64,
    pub inlined_calls: u64,
    pub expansion_steps: u64,
    pub expanded_items: u64,
    pub reload_edit_distance: u64,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            recursive_calls: RECURSIVE_CALLS,
            nesting_depth: NESTING_DEPTH,
            inlined_calls: INLINED_CALLS,
            expansion_steps: EXPANSION_STEPS,
            expanded_items: EXPANDED_ITEMS,
            reload_edit_distance: RELOAD_EDIT_DISTANCE,
        }
    }
}
