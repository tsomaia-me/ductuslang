#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct BoundRange {
    pub min: Option<i128>,
    pub max: Option<i128>,
}

impl BoundRange {
    pub const UNBOUNDED: BoundRange = BoundRange {
        min: None,
        max: None,
    };

    pub fn exactly(value: i128) -> BoundRange {
        BoundRange {
            min: Some(value),
            max: Some(value),
        }
    }

    pub fn between(min: i128, max: Option<i128>) -> BoundRange {
        BoundRange {
            min: Some(min),
            max,
        }
    }

    pub fn smaller(self, other: BoundRange) -> BoundRange {
        BoundRange {
            min: self.min.zip(other.min).map(|(a, b)| a.min(b)),
            max: either(self.max, other.max, i128::min),
        }
    }

    pub fn larger(self, other: BoundRange) -> BoundRange {
        BoundRange {
            min: either(self.min, other.min, i128::max),
            max: self.max.zip(other.max).map(|(a, b)| a.max(b)),
        }
    }

    pub fn sum(self, other: BoundRange) -> BoundRange {
        BoundRange {
            min: self.min.zip(other.min).map(|(a, b)| a.saturating_add(b)),
            max: self.max.zip(other.max).map(|(a, b)| a.saturating_add(b)),
        }
    }

    pub fn difference(self, other: BoundRange) -> BoundRange {
        BoundRange {
            min: self.min.zip(other.max).map(|(a, b)| a.saturating_sub(b)),
            max: self.max.zip(other.min).map(|(a, b)| a.saturating_sub(b)),
        }
    }
}

fn either(a: Option<i128>, b: Option<i128>, pick: fn(i128, i128) -> i128) -> Option<i128> {
    match (a, b) {
        (Some(a), Some(b)) => Some(pick(a, b)),
        (Some(one), None) | (None, Some(one)) => Some(one),
        (None, None) => None,
    }
}
