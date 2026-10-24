use std::collections::BTreeSet;

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

struct Site {
    callee: FnId,
    defaulted: Vec<usize>,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct InlineCounts {
    body: Vec<u64>,
    defaults: Vec<Vec<u64>>,
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
        };
        let sites = |index: usize, roots: Vec<ExprId>| -> Result<Vec<Site>, Internal> {
            let id = FnId(u32::try_from(index).unwrap_or(u32::MAX));
            let namespace = if hir.is_prelude_fn(id) {
                prelude
            } else {
                source
            };
            let decl = hir.fns.get(index).ok_or(Internal::missed("function"))?;
            call_sites(hir, decl, namespace, recursive, roots)
        };
        let mut state = vec![Visit::New; count];
        for start in 0..count {
            if state.get(start) != Some(&Visit::New) {
                continue;
            }
            let mut stack = vec![start];
            while let Some(&index) = stack.last() {
                let decl = hir.fns.get(index).ok_or(Internal::missed("function"))?;
                let mut roots: Vec<ExprId> = decl.lets.iter().map(|local| local.value).collect();
                roots.push(decl.value);
                let body_sites = sites(index, roots)?;
                let default_sites: Vec<Vec<Site>> = decl
                    .params
                    .iter()
                    .map(|param| sites(index, param.default.into_iter().collect()))
                    .collect::<Result<_, _>>()?;
                if state.get(index) == Some(&Visit::New) {
                    if let Some(slot) = state.get_mut(index) {
                        *slot = Visit::Open;
                    }
                    let pending: Vec<usize> = body_sites
                        .iter()
                        .chain(default_sites.iter().flatten())
                        .map(|site| site.callee.index())
                        .filter(|callee| state.get(*callee) == Some(&Visit::New))
                        .collect();
                    if !pending.is_empty() {
                        stack.extend(pending);
                        continue;
                    }
                }
                stack.pop();
                if state.get(index) == Some(&Visit::Done) {
                    continue;
                }
                let body = body_sites
                    .iter()
                    .fold(0u64, |total, site| total.saturating_add(counts.site(site)));
                let defaults: Vec<u64> = default_sites
                    .iter()
                    .map(|sites| {
                        sites
                            .iter()
                            .fold(0u64, |total, site| total.saturating_add(counts.site(site)))
                    })
                    .collect();
                if let Some(slot) = counts.body.get_mut(index) {
                    *slot = body;
                }
                if let Some(slot) = counts.defaults.get_mut(index) {
                    *slot = defaults;
                }
                if let Some(slot) = state.get_mut(index) {
                    *slot = Visit::Done;
                }
            }
        }
        Ok(counts)
    }

    fn site(&self, site: &Site) -> u64 {
        self.call(site.callee, &site.defaulted)
    }

    pub fn call(&self, callee: FnId, defaulted: &[usize]) -> u64 {
        let body = self.body.get(callee.index()).copied().unwrap_or(0);
        let defaults = self.defaults.get(callee.index());
        defaulted
            .iter()
            .filter_map(|param| defaults.and_then(|counts| counts.get(*param)))
            .fold(body.saturating_add(1), |total, count| {
                total.saturating_add(*count)
            })
    }
}

fn call_sites(
    hir: &Hir,
    decl: &FnDecl,
    namespace: &Namespace,
    recursive: &[bool],
    roots: Vec<ExprId>,
) -> Result<Vec<Site>, Internal> {
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
        let kind = &hir.expr(expr)?.kind;
        if let ExprKind::Call(callee, args) = kind
            && let ExprKind::Name(name) = &hir.expr(*callee)?.kind
            && !locals.contains(name.as_str())
            && let Some(Decl::Fn(target)) = namespace.get(name)
            && !recursive.get(target.index()).copied().unwrap_or(true)
        {
            found.push(Site {
                callee: target,
                defaulted: defaulted(hir.fn_decl(target)?, args),
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
