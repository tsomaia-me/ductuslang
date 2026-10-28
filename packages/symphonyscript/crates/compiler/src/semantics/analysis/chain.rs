use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fmt;
use std::rc::Rc;

use crate::semantics::ir::Loc;
use crate::semantics::reporting::grouped;

#[derive(Clone, Default)]
pub(crate) struct Chain(Option<Rc<Link>>);

struct Link {
    loc: Loc,
    label: &'static str,
    also: RefCell<Vec<Loc>>,
    rest: Chain,
}

impl Chain {
    pub fn push(&self, loc: Loc, label: &'static str) -> Chain {
        Chain(Some(Rc::new(Link {
            loc,
            label,
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
            crate::semantics::counted_walk::CountedWalk::ChainToVec.step();
            locs.push(link.loc);
            current = link.rest.0.as_ref();
        }
        locs
    }

    pub fn depth(&self) -> usize {
        let mut depth = 0;
        let mut current = self.0.as_ref();
        while let Some(link) = current {
            crate::semantics::counted_walk::CountedWalk::ChainDepth.step();
            depth += 1;
            current = link.rest.0.as_ref();
        }
        depth
    }

    pub fn rebase(&self, keep: usize, base: &Chain) -> Chain {
        let mut links = Vec::new();
        let mut current = self.0.as_ref();
        while let Some(link) = current {
            crate::semantics::counted_walk::CountedWalk::ChainRebase.step();
            if links.len() == keep {
                break;
            }
            links.push((link.loc, link.label));
            current = link.rest.0.as_ref();
        }
        links
            .into_iter()
            .rev()
            .fold(base.clone(), |chain, (loc, label)| chain.push(loc, label))
    }

    pub fn labelled(&self) -> Vec<(Loc, String)> {
        let mut links = Vec::new();
        let mut current = self.0.as_ref();
        while let Some(link) = current {
            crate::semantics::counted_walk::CountedWalk::ChainLabelled.step();
            links.push((link.loc, link.label));
            current = link.rest.0.as_ref();
        }
        repeats_named_once(&links)
    }

    pub fn outermost_paths(&self) -> Vec<Vec<(Loc, String)>> {
        let mut inner = Vec::new();
        let mut current = self.0.as_ref();
        while let Some(link) = current {
            crate::semantics::counted_walk::CountedWalk::ChainOutermostPaths.step();
            match link.rest.0.as_ref() {
                Some(_) => inner.push((link.loc, link.label)),
                None => {
                    let also = link.also.borrow();
                    return std::iter::once(&link.loc)
                        .chain(also.iter())
                        .map(|outermost| {
                            let mut path = inner.clone();
                            path.push((*outermost, link.label));
                            repeats_named_once(&path)
                        })
                        .collect();
                }
            }
            current = link.rest.0.as_ref();
        }
        vec![repeats_named_once(&inner)]
    }
}

fn repeats_named_once(links: &[(Loc, &'static str)]) -> Vec<(Loc, String)> {
    let mut levels: BTreeMap<Loc, u64> = BTreeMap::new();
    for (loc, _) in links {
        *levels.entry(*loc).or_insert(0) += 1;
    }
    let mut named = Vec::new();
    for (loc, label) in links {
        let Some(count) = levels.remove(loc) else {
            continue;
        };
        named.push(match count {
            1 => (*loc, (*label).to_string()),
            _ => (
                *loc,
                format!("{label}, repeated over {} levels", grouped(count)),
            ),
        });
    }
    named
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
            crate::semantics::counted_walk::CountedWalk::ChainDrop.step();
            current = match Rc::try_unwrap(link) {
                Ok(mut link) => link.rest.0.take(),
                Err(_) => None,
            };
        }
    }
}
