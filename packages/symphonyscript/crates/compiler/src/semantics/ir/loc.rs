#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub(crate) struct Loc {
    pub file: u32,
    pub start: u32,
    pub end: u32,
}

impl Loc {
    pub fn new(file: u32, range: std::ops::Range<u32>) -> Self {
        Self {
            file,
            start: range.start,
            end: range.end,
        }
    }

    pub fn contains(self, other: Loc) -> bool {
        self.file == other.file && self.start <= other.start && other.end <= self.end
    }

    pub fn to(self, other: Loc) -> Loc {
        Loc {
            file: self.file,
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }
}
