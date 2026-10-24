use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

use crate::semantics::ir::Loc;

#[derive(Clone, Default)]
pub(crate) struct Chain(Option<Rc<Link>>);

struct Link {
    loc: Loc,
    also: RefCell<Vec<Loc>>,
    rest: Chain,
}

impl Chain {
    pub fn push(&self, loc: Loc) -> Chain {
        Chain(Some(Rc::new(Link {
            loc,
            also: RefCell::new(Vec::new()),
            rest: self.clone(),
        })))
    }

    pub fn also(&self, loc: Loc) {
        if let Some(link) = &self.0 {
            link.also.borrow_mut().push(loc);
        }
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

    pub fn paths(&self) -> Vec<Vec<Loc>> {
        let mut paths = vec![Vec::new()];
        let mut current = self.0.as_ref();
        while let Some(link) = current {
            let also = link.also.borrow();
            let mut next = Vec::with_capacity(paths.len() * (1 + also.len()));
            for path in &paths {
                for loc in std::iter::once(&link.loc).chain(also.iter()) {
                    let mut extended = path.clone();
                    extended.push(*loc);
                    next.push(extended);
                }
            }
            paths = next;
            current = link.rest.0.as_ref();
        }
        paths
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
