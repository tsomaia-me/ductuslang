use super::limits::Limits;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct LimitOverrides {
    pub recursive_calls: Option<u64>,
    pub recursive_calls_warning: Option<u64>,
    pub nesting_depth: Option<u64>,
    pub nesting_depth_warning: Option<u64>,
    pub inlined_calls: Option<u64>,
    pub inlined_calls_warning: Option<u64>,
    pub expansion_steps: Option<u64>,
    pub expansion_steps_warning: Option<u64>,
    pub expanded_items: Option<u64>,
    pub expanded_items_warning: Option<u64>,
    pub memory_bytes: Option<u64>,
    pub memory_bytes_warning: Option<u64>,
    pub reload_edit_distance: Option<u64>,
    pub shared_body_threshold: Option<u64>,
}

impl LimitOverrides {
    pub fn applied_to(self, limits: Limits) -> Limits {
        Limits {
            recursive_calls: self.recursive_calls.unwrap_or(limits.recursive_calls),
            recursive_calls_warning: self
                .recursive_calls_warning
                .unwrap_or(limits.recursive_calls_warning),
            nesting_depth: self.nesting_depth.unwrap_or(limits.nesting_depth),
            nesting_depth_warning: self
                .nesting_depth_warning
                .unwrap_or(limits.nesting_depth_warning),
            inlined_calls: self.inlined_calls.unwrap_or(limits.inlined_calls),
            inlined_calls_warning: self
                .inlined_calls_warning
                .unwrap_or(limits.inlined_calls_warning),
            expansion_steps: self.expansion_steps.unwrap_or(limits.expansion_steps),
            expansion_steps_warning: self
                .expansion_steps_warning
                .unwrap_or(limits.expansion_steps_warning),
            expanded_items: self.expanded_items.unwrap_or(limits.expanded_items),
            expanded_items_warning: self
                .expanded_items_warning
                .unwrap_or(limits.expanded_items_warning),
            memory_bytes: self.memory_bytes.unwrap_or(limits.memory_bytes),
            memory_bytes_warning: self
                .memory_bytes_warning
                .unwrap_or(limits.memory_bytes_warning),
            reload_edit_distance: self
                .reload_edit_distance
                .unwrap_or(limits.reload_edit_distance),
            shared_body_threshold: self
                .shared_body_threshold
                .unwrap_or(limits.shared_body_threshold),
        }
    }
}
