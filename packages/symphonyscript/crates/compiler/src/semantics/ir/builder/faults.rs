use std::ops::Range;

use crate::diagnostics::Diagnostic;

pub(super) struct Faults {
    marks: Vec<(u32, u32)>,
}

impl Faults {
    pub fn new(diagnostics: &[Diagnostic], detached: &[Range<u32>]) -> Self {
        let mut ranges: Vec<Range<u32>> = diagnostics
            .iter()
            .map(|diagnostic| diagnostic.span.range.clone())
            .filter(|range| !detached.contains(range))
            .collect();
        ranges.sort_by_key(|range| range.start);
        let mut furthest = 0;
        let marks = ranges
            .iter()
            .map(|range| {
                furthest = furthest.max(range.end.max(range.start + 1));
                (range.start, furthest)
            })
            .collect();
        Self { marks }
    }

    pub fn touch(&self, range: Range<u32>) -> bool {
        let end = range.end.max(range.start + 1);
        let before = self.marks.partition_point(|(start, _)| *start < end);
        self.marks
            .iter()
            .take(before)
            .next_back()
            .is_some_and(|(_, reach)| *reach > range.start)
    }
}
