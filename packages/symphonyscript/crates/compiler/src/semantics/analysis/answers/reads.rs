#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub(crate) enum Reads {
    Value,
    Chosen,
    Cell,
}

impl Reads {
    pub fn join(self, other: Reads) -> Reads {
        self.max(other)
    }

    pub fn of_all(reads: impl IntoIterator<Item = Reads>) -> Reads {
        reads.into_iter().fold(Reads::Value, Reads::join)
    }

    pub fn is_cell(self) -> bool {
        self == Reads::Cell
    }

    pub fn is_value(self) -> bool {
        self == Reads::Value
    }
}
