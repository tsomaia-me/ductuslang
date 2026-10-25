use std::ops::Range;

use crate::diagnostics::Diagnostic;

pub(super) struct Faults {
    marks: Vec<(u32, u32)>,
}

impl Faults {
    pub fn new(diagnostics: &[Diagnostic], detached: &[Range<u32>]) -> Self {
        let key = |range: &Range<u32>| (range.start, range.end);
        let mut all: Vec<Range<u32>> = diagnostics
            .iter()
            .map(|diagnostic| diagnostic.span.range.clone())
            .collect();
        all.sort_by_key(key);
        let mut unmarked = detached.to_vec();
        unmarked.sort_by_key(key);
        let mut unmarked = unmarked.into_iter().peekable();
        let mut ranges = Vec::with_capacity(all.len());
        for range in all {
            while unmarked.next_if(|skip| key(skip) < key(&range)).is_some() {}
            if unmarked.next_if(|skip| *skip == range).is_none() {
                ranges.push(range);
            }
        }
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
