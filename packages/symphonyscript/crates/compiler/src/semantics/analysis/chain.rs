use std::fmt;
use std::rc::Rc;

use crate::semantics::ir::Loc;

#[derive(Clone, Default)]
pub(crate) struct Chain(Option<Rc<Link>>);

struct Link {
    loc: Loc,
    rest: Chain,
}

impl Chain {
    pub fn push(&self, loc: Loc) -> Chain {
        Chain(Some(Rc::new(Link {
            loc,
            rest: self.clone(),
        })))
    }

    pub fn to_vec(&self) -> Vec<Loc> {
        let mut locs = Vec::new();
        let mut current = self.0.as_ref();
        while let Some(link) = current {
            locs.push(link.loc);
            current = link.rest.0.as_ref();
        }
        locs
    }
}

impl PartialEq for Chain {
    fn eq(&self, other: &Self) -> bool {
        self.to_vec() == other.to_vec()
    }
}

impl fmt::Debug for Chain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.to_vec()).finish()
    }
}

impl Drop for Chain {
    fn drop(&mut self) {
        let mut current = self.0.take();
        while let Some(link) = current {
            current = match Rc::try_unwrap(link) {
                Ok(mut link) => link.rest.0.take(),
                Err(_) => None,
            };
        }
    }
}
