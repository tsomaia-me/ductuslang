use super::leaf::Leaf;
use crate::semantics::operators::EvalTy;

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub(crate) struct Fact {
    pub leaf: Leaf,
    pub dims: Vec<u32>,
    pub fixed: Option<EvalTy>,
}

impl Fact {
    pub const PLACEHOLDER: Fact = Fact {
        leaf: Leaf::Placeholder,
        dims: Vec::new(),
        fixed: None,
    };

    pub const ERROR: Fact = Fact {
        leaf: Leaf::Error,
        dims: Vec::new(),
        fixed: None,
    };

    pub fn scalar(leaf: Leaf) -> Fact {
        Fact {
            leaf,
            dims: Vec::new(),
            fixed: None,
        }
    }

    pub fn fixed(leaf: Leaf, fixed: Option<EvalTy>) -> Fact {
        Fact {
            leaf,
            dims: Vec::new(),
            fixed,
        }
    }
}
