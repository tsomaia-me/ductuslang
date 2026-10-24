#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Count {
    Exactly(u32),
    AtLeast(u32),
    AtMost(u32),
    Between(u32, u32),
    Any,
}

impl Count {
    pub fn min(self) -> u32 {
        match self {
            Count::Exactly(n) | Count::AtLeast(n) | Count::Between(n, _) => n,
            Count::AtMost(_) | Count::Any => 0,
        }
    }

    pub fn max(self) -> Option<u32> {
        match self {
            Count::Exactly(n) | Count::AtMost(n) | Count::Between(_, n) => Some(n),
            Count::AtLeast(_) | Count::Any => None,
        }
    }

    pub fn contains(self, n: u32) -> bool {
        n >= self.min() && self.max().is_none_or(|max| n <= max)
    }
}
