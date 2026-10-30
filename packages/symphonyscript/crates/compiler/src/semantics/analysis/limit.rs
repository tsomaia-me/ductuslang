#[derive(Clone, Copy)]
pub(crate) enum Limit {
    RecursiveCalls,
    NestingDepth,
    InlinedCalls,
    ExpansionSteps,
    ExpandedItems,
}

impl Limit {
    fn key(self) -> &'static str {
        match self {
            Limit::RecursiveCalls => "recursive_calls",
            Limit::NestingDepth => "nesting_depth",
            Limit::InlinedCalls => "inlined_calls",
            Limit::ExpansionSteps => "expansion_steps",
            Limit::ExpandedItems => "expanded_items",
        }
    }

    pub(crate) fn warning_help(self) -> String {
        let key = self.key();
        format!(
            "reduce the work, or raise `{key}_warning` in the package's `options` or with `--limit {key}_warning=<value>`"
        )
    }
}
