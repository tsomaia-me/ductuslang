use super::leaf::Leaf;
use super::width::{combined, is_fixed_float};
use crate::semantics::operators::EvalTy;

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub(crate) struct Fact {
    pub leaf: Leaf,
    pub dims: Vec<u32>,
    pub fixed: Option<EvalTy>,
    pub read: Option<EvalTy>,
}

impl Fact {
    pub const PLACEHOLDER: Fact = Fact {
        leaf: Leaf::Placeholder,
        dims: Vec::new(),
        fixed: None,
        read: None,
    };

    pub const ERROR: Fact = Fact {
        leaf: Leaf::Error,
        dims: Vec::new(),
        fixed: None,
        read: None,
    };

    pub fn scalar(leaf: Leaf) -> Fact {
        Fact {
            leaf,
            dims: Vec::new(),
            fixed: None,
            read: None,
        }
    }

    pub fn fixed(leaf: Leaf, fixed: Option<EvalTy>) -> Fact {
        Fact {
            leaf,
            dims: Vec::new(),
            fixed,
            read: None,
        }
    }

    pub fn numeric(leaf: Leaf, fixed: Option<EvalTy>, read: Option<EvalTy>) -> Fact {
        Fact {
            leaf,
            dims: Vec::new(),
            fixed,
            read,
        }
    }

    pub fn own(&self) -> Option<EvalTy> {
        combined(self.fixed, self.read, self.leaf == Leaf::Float)
    }
}

pub(crate) fn reads(a: Option<EvalTy>, b: Option<EvalTy>, float: bool) -> Option<EvalTy> {
    let kept = |read: Option<EvalTy>| read.filter(|read| !float || is_fixed_float(*read));
    combined(kept(a), kept(b), float)
}
