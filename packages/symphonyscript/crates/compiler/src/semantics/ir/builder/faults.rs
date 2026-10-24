use std::ops::Range;

use crate::diagnostics::Diagnostic;

pub(super) struct Faults {
    starts: Vec<u32>,
    reach: Vec<u32>,
}

impl Faults {
    pub fn new(diagnostics: &[Diagnostic]) -> Self {
        let mut ranges: Vec<Range<u32>> = diagnostics
            .iter()
            .map(|diagnostic| diagnostic.span.range.clone())
            .collect();
        ranges.sort_by_key(|range| range.start);
        let starts = ranges.iter().map(|range| range.start).collect();
        let mut furthest = 0;
        let reach = ranges
            .iter()
            .map(|range| {
                furthest = furthest.max(range.end.max(range.start + 1));
                furthest
            })
            .collect();
        Self { starts, reach }
    }

    pub fn touch(&self, range: Range<u32>) -> bool {
        let end = range.end.max(range.start + 1);
        let before = self.starts.partition_point(|start| *start < end);
        before > 0
            && self
                .reach
                .get(before - 1)
                .is_some_and(|reach| *reach > range.start)
    }
}
