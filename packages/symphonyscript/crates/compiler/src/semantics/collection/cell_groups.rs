use std::collections::BTreeSet;

use super::decl::Decl;
use super::namespace::Namespace;
use super::strongly_connected::strongly_connected;
use crate::semantics::Internal;
use crate::semantics::ir::{ExprId, ExprKind, Hir, LetId, StateId};

#[derive(Clone, Debug, Default)]
pub(crate) struct CellGroups {
    of_state: Vec<usize>,
    states: Vec<Vec<StateId>>,
    lets: Vec<Vec<LetId>>,
}

impl CellGroups {
    pub fn of(hir: &Hir, source: &Namespace) -> Result<Self, Internal> {
        let states = hir.states.len();
        let lets = hir.lets.len();
        let mut roots: Vec<(Vec<ExprId>, BTreeSet<&str>)> =
            Vec::with_capacity(states + lets + hir.fns.len());
        for decl in &hir.states {
            let mut exprs = vec![decl.value];
            exprs.extend(decl.clock.iter().copied());
            roots.push((exprs, BTreeSet::new()));
        }
        for decl in &hir.lets {
            roots.push((vec![decl.value], BTreeSet::new()));
        }
        for decl in &hir.fns {
            let locals: BTreeSet<&str> = decl
                .params
                .iter()
                .filter_map(|param| param.name.as_ref())
                .chain(decl.lets.iter().filter_map(|local| local.name.as_ref()))
                .map(|name| name.text.as_str())
                .collect();
            let mut exprs: Vec<ExprId> = decl
                .params
                .iter()
                .filter_map(|param| param.default)
                .collect();
            exprs.extend(decl.lets.iter().map(|local| local.value));
            exprs.push(decl.value);
            roots.push((exprs, locals));
        }
        let target = |name: &str| match source.get(name) {
            Some(Decl::State(id)) => Some(id.index()),
            Some(Decl::Let(id)) => Some(states + id.index()),
            Some(Decl::Fn(id)) => Some(states + lets + id.index()),
            Some(_) | None => None,
        };
        let mut edges = Vec::with_capacity(roots.len());
        for (exprs, locals) in roots {
            let mut targets = BTreeSet::new();
            let mut stack = exprs;
            while let Some(expr) = stack.pop() {
                crate::semantics::counted_walk::CountedWalk::CellGroupsOf.step();
                let kind = &hir.expr(expr)?.kind;
                let name = if let ExprKind::Name(name) = kind {
                    Some(name.as_str())
                } else if let ExprKind::History { target, .. } = kind {
                    Some(target.text.as_str())
                } else {
                    None
                };
                if let Some(name) = name
                    && !locals.contains(name)
                    && let Some(found) = target(name)
                {
                    targets.insert(found);
                }
                stack.extend(kind.children());
            }
            edges.push(targets.into_iter().collect::<Vec<usize>>());
        }
        let components = strongly_connected(&edges)?;
        let mut groups = CellGroups {
            of_state: vec![0; states],
            states: vec![Vec::new(); components.len()],
            lets: vec![Vec::new(); components.len()],
        };
        for (group, component) in components.into_iter().enumerate() {
            for member in component {
                if member < states {
                    *groups
                        .of_state
                        .get_mut(member)
                        .ok_or(Internal::missed("cell group"))? = group;
                    groups
                        .states
                        .get_mut(group)
                        .ok_or(Internal::missed("cell group"))?
                        .push(StateId(u32::try_from(member).unwrap_or(u32::MAX)));
                } else if member < states + lets {
                    groups
                        .lets
                        .get_mut(group)
                        .ok_or(Internal::missed("cell group"))?
                        .push(LetId(u32::try_from(member - states).unwrap_or(u32::MAX)));
                }
            }
        }
        Ok(groups)
    }

    pub fn group_of(&self, id: StateId) -> (&[StateId], &[LetId]) {
        let group = self.of_state.get(id.index()).copied();
        let states = group.and_then(|group| self.states.get(group));
        let lets = group.and_then(|group| self.lets.get(group));
        (
            states.map_or(&[], Vec::as_slice),
            lets.map_or(&[], Vec::as_slice),
        )
    }
}
