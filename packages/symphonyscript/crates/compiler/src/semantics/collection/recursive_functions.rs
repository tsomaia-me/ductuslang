use std::collections::BTreeSet;

use super::decl::Decl;
use super::is_cycle::is_cycle;
use super::namespace::Namespace;
use super::strongly_connected::strongly_connected;
use crate::semantics::Internal;
use crate::semantics::ir::{ExprId, ExprKind, FnId, Hir};

type Recursion = (Vec<bool>, Vec<bool>, Vec<bool>, Vec<Vec<(usize, ExprId)>>);

pub(crate) fn recursive_functions(
    hir: &Hir,
    source: &Namespace,
    prelude: &Namespace,
) -> Result<Recursion, Internal> {
    let mut edges: Vec<Vec<usize>> = Vec::with_capacity(hir.fns.len());
    let mut calls: Vec<Vec<(usize, ExprId)>> = Vec::with_capacity(hir.fns.len());
    let mut unresolved = Vec::with_capacity(hir.fns.len());
    for (index, decl) in hir.fns.iter().enumerate() {
        let id = FnId(u32::try_from(index).unwrap_or(u32::MAX));
        let namespace = if hir.is_prelude_fn(id) {
            prelude
        } else {
            source
        };
        let locals: BTreeSet<&str> = decl
            .params
            .iter()
            .filter_map(|param| param.name.as_ref())
            .chain(decl.lets.iter().filter_map(|local| local.name.as_ref()))
            .map(|name| name.text.as_str())
            .collect();
        let mut roots: Vec<ExprId> = decl
            .params
            .iter()
            .filter_map(|param| param.default)
            .collect();
        roots.extend(decl.lets.iter().map(|local| local.value));
        roots.push(decl.value);
        roots.extend(decl.bound.iter().map(|bound| bound.fallback));
        let mut targets = BTreeSet::new();
        let mut sites = Vec::new();
        let mut unsure = false;
        let mut stack = roots;
        while let Some(expr) = stack.pop() {
            let kind = &hir.expr(expr)?.kind;
            if let ExprKind::Call(callee, _) = kind
                && let ExprKind::Name(name) = &hir.expr(*callee)?.kind
                && !locals.contains(name.as_str())
            {
                match namespace.get(name) {
                    Some(Decl::Fn(target)) => {
                        targets.insert(target.index());
                        sites.push((target.index(), expr));
                    }
                    Some(_) => {}
                    None => unsure = true,
                }
            }
            stack.extend(kind.children());
        }
        edges.push(targets.into_iter().collect());
        sites.sort_by_key(|(_, expr)| *expr);
        calls.push(sites);
        unresolved.push(unsure);
    }
    let mut recursive = vec![false; hir.fns.len()];
    let mut bounded = vec![false; hir.fns.len()];
    for component in strongly_connected(&edges)? {
        if is_cycle(&component, &edges)? {
            let mut every = true;
            for member in &component {
                let decl = hir.fns.get(*member).ok_or(Internal::missed("function"))?;
                every &= decl.bound.is_some();
            }
            for member in component {
                *recursive
                    .get_mut(member)
                    .ok_or(Internal::missed("function"))? = true;
                *bounded
                    .get_mut(member)
                    .ok_or(Internal::missed("function"))? = every;
            }
        }
    }
    Ok((recursive, bounded, unresolved, calls))
}
