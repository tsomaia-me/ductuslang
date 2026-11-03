use super::leaf::Leaf;

#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) struct Fact {
    pub leaf: Leaf,
    pub dims: Vec<u32>,
}

impl Fact {
    pub const PLACEHOLDER: Fact = Fact {
        leaf: Leaf::Placeholder,
        dims: Vec::new(),
    };

    pub const ERROR: Fact = Fact {
        leaf: Leaf::Error,
        dims: Vec::new(),
    };

    pub fn scalar(leaf: Leaf) -> Fact {
        Fact {
            leaf,
            dims: Vec::new(),
        }
    }
}
