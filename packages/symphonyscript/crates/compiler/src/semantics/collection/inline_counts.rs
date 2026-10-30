use std::collections::BTreeSet;

use super::call_site::CallSite;
use super::decl::Decl;
use super::namespace::Namespace;
use crate::semantics::Internal;
use crate::semantics::ir::{Args, ExprId, ExprKind, FnDecl, FnId, Hir};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Visit {
    New,
    Open,
    Done,
}

type FnSites = (Vec<CallSite>, Vec<Vec<CallSite>>);

#[derive(Clone, Debug, Default)]
pub(crate) struct InlineCounts {
    body: Vec<u64>,
    defaults: Vec<Vec<u64>>,
    body_sites: Vec<Vec<CallSite>>,
    default_sites: Vec<Vec<Vec<CallSite>>>,
    #[cfg(test)]
    pub walked: u64,
}

impl InlineCounts {
    pub fn of(
        hir: &Hir,
        source: &Namespace,
        prelude: &Namespace,
        recursive: &[bool],
    ) -> Result<Self, Internal> {
        let count = hir.fns.len();
        let mut counts = InlineCounts {
            body: vec![0; count],
            defaults: hir
                .fns
                .iter()
                .map(|decl| vec![0; decl.params.len()])
                .collect(),
            body_sites: vec![Vec::new(); count],
            default_sites: hir
                .fns
                .iter()
                .map(|decl| vec![Vec::new(); decl.params.len()])
                .collect(),
            #[cfg(test)]
            walked: 0,
        };
        let mut walked = 0u64;
        let mut sites = |index: usize, roots: Vec<ExprId>| -> Result<Vec<CallSite>, Internal> {
            let id = FnId(u32::try_from(index).unwrap_or(u32::MAX));
            let namespace = if hir.is_prelude_fn(id) {
                prelude
            } else {
                source
            };
            let decl = hir.fns.get(index).ok_or(Internal::missed("function"))?;
            call_sites(hir, decl, namespace, recursive, roots, &mut walked)
        };
        let mut state = vec![Visit::New; count];
        let mut found: Vec<Option<FnSites>> = vec![None; count];
        for start in 0..count {
            if visit_of(&state, start)? != Visit::New {
                continue;
            }
            let mut stack = vec![start];
            while let Some(&index) = stack.last() {
                crate::semantics::counted_walk::CountedWalk::InlineCountsOf.step();
                let visit = visit_of(&state, index)?;
                if visit == Visit::Done {
                    stack.pop();
                    continue;
                }
                let slot = found.get_mut(index).ok_or(Internal::missed("function"))?;
                if slot.is_none() {
                    let decl = hir.fns.get(index).ok_or(Internal::missed("function"))?;
                    let mut roots: Vec<ExprId> =
                        decl.lets.iter().map(|local| local.value).collect();
                    roots.push(decl.value);
                    let body_sites = sites(index, roots)?;
                    let default_sites: Vec<Vec<CallSite>> = decl
                        .params
                        .iter()
                        .map(|param| sites(index, param.default.into_iter().collect()))
                        .collect::<Result<_, _>>()?;
                    *slot = Some((body_sites, default_sites));
                }
                let (body_sites, default_sites) =
                    slot.as_ref().ok_or(Internal::missed("call sites"))?;
                if visit == Visit::New {
                    let mut pending: Vec<usize> = Vec::new();
                    for site in body_sites.iter().chain(default_sites.iter().flatten()) {
                        let callee = site.callee.index();
                        if !site.recursive && visit_of(&state, callee)? == Visit::New {
                            pending.push(callee);
                        }
                    }
                    *visit_mut(&mut state, index)? = Visit::Open;
                    if !pending.is_empty() {
                        stack.extend(pending);
                        continue;
                    }
                }
                stack.pop();
                let (body_sites, default_sites) = found
                    .get_mut(index)
                    .and_then(Option::take)
                    .ok_or(Internal::missed("call sites"))?;
                let body = counts.total(&body_sites)?;
                let defaults: Vec<u64> = default_sites
                    .iter()
                    .map(|sites| counts.total(sites))
                    .collect::<Result<_, _>>()?;
                *counts
                    .body
                    .get_mut(index)
                    .ok_or(Internal::missed("function"))? = body;
                *counts
                    .defaults
                    .get_mut(index)
                    .ok_or(Internal::missed("function"))? = defaults;
                *counts
                    .body_sites
                    .get_mut(index)
                    .ok_or(Internal::missed("function"))? = body_sites;
                *counts
                    .default_sites
                    .get_mut(index)
                    .ok_or(Internal::missed("function"))? = default_sites;
                *visit_mut(&mut state, index)? = Visit::Done;
            }
        }
        #[cfg(test)]
        {
            counts.walked = walked;
        }
        Ok(counts)
    }

    fn total(&self, sites: &[CallSite]) -> Result<u64, Internal> {
        sites
            .iter()
            .filter(|site| !site.recursive)
            .try_fold(0u64, |total, site| {
                Ok(total.saturating_add(self.call(site.callee, &site.defaulted)?))
            })
    }

    pub fn body(&self, function: FnId) -> Result<u64, Internal> {
        self.body
            .get(function.index())
            .copied()
            .ok_or(Internal::missed("function"))
    }

    pub fn default(&self, function: FnId, param: usize) -> Result<u64, Internal> {
        self.defaults
            .get(function.index())
            .and_then(|defaults| defaults.get(param))
            .copied()
            .ok_or(Internal::missed("parameter default"))
    }

    pub fn body_sites(&self, function: FnId) -> Result<&[CallSite], Internal> {
        self.body_sites
            .get(function.index())
            .map(Vec::as_slice)
            .ok_or(Internal::missed("function"))
    }

    pub fn default_sites(&self, function: FnId, param: usize) -> Result<&[CallSite], Internal> {
        self.default_sites
            .get(function.index())
            .and_then(|sites| sites.get(param))
            .map(Vec::as_slice)
            .ok_or(Internal::missed("parameter default"))
    }

    pub fn call(&self, callee: FnId, defaulted: &[usize]) -> Result<u64, Internal> {
        let missed = Internal::missed("function");
        let body = *self.body.get(callee.index()).ok_or(missed)?;
        let defaults = self.defaults.get(callee.index()).ok_or(missed)?;
        defaulted
            .iter()
            .try_fold(body.saturating_add(1), |total, param| {
                let count = defaults
                    .get(*param)
                    .ok_or(Internal::missed("parameter default"))?;
                Ok(total.saturating_add(*count))
            })
    }
}

fn visit_of(state: &[Visit], index: usize) -> Result<Visit, Internal> {
    state
        .get(index)
        .copied()
        .ok_or(Internal::missed("function"))
}

fn visit_mut(state: &mut [Visit], index: usize) -> Result<&mut Visit, Internal> {
    state.get_mut(index).ok_or(Internal::missed("function"))
}

fn call_sites(
    hir: &Hir,
    decl: &FnDecl,
    namespace: &Namespace,
    recursive: &[bool],
    roots: Vec<ExprId>,
    walked: &mut u64,
) -> Result<Vec<CallSite>, Internal> {
    let locals: BTreeSet<&str> = decl
        .params
        .iter()
        .filter_map(|param| param.name.as_ref())
        .chain(decl.lets.iter().filter_map(|local| local.name.as_ref()))
        .map(|name| name.text.as_str())
        .collect();
    let mut found = Vec::new();
    let mut stack = roots;
    while let Some(expr) = stack.pop() {
        crate::semantics::counted_walk::CountedWalk::InlineCountsCallSites.step();
        *walked = walked.saturating_add(1);
        let kind = &hir.expr(expr)?.kind;
        if let ExprKind::Call(callee, args) = kind
            && let ExprKind::Name(name) = &hir.expr(*callee)?.kind
            && !locals.contains(name.as_str())
            && let Some(Decl::Fn(target)) = namespace.get(name)
        {
            found.push(CallSite {
                callee: target,
                defaulted: defaulted(hir.fn_decl(target)?, args),
                recursive: *recursive
                    .get(target.index())
                    .ok_or(Internal::missed("function"))?,
            });
        }
        stack.extend(kind.children());
    }
    Ok(found)
}

fn defaulted(callee: &FnDecl, args: &Args) -> Vec<usize> {
    callee
        .params
        .iter()
        .enumerate()
        .filter(|(position, param)| {
            param.default.is_some()
                && match args {
                    Args::Positional(values) => *position >= values.len(),
                    Args::Named(values) => !values.iter().any(|(name, _)| {
                        param.name.as_ref().is_some_and(|own| own.text == name.text)
                    }),
                }
        })
        .map(|(position, _)| position)
        .collect()
}
