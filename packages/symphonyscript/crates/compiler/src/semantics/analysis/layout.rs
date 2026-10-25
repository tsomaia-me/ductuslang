#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub(crate) struct Layout {
    pub lengths: Vec<u32>,
    pub variable: bool,
}

impl Layout {
    pub fn fixed(lengths: Vec<u32>) -> Layout {
        Layout {
            lengths,
            variable: false,
        }
    }

    pub fn variable(inner: Vec<u32>) -> Layout {
        Layout {
            lengths: inner,
            variable: true,
        }
    }

    pub fn inner(&self) -> &[u32] {
        if self.variable {
            &self.lengths
        } else {
            self.lengths.split_last().map_or(&[], |(_, inner)| inner)
        }
    }

    pub fn outer(&self) -> Option<u32> {
        if self.variable {
            None
        } else {
            self.lengths.last().copied()
        }
    }
}
