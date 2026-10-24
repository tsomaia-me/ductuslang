use std::collections::BTreeSet;

use super::decl::Decl;
use super::namespace::Namespace;
use super::stand_in::StandIn;
use super::strongly_connected::strongly_connected;
use crate::diagnostics::Code;
use crate::semantics::ir::{BodyItem, Hir, PatternId};
use crate::semantics::reporting::Report;

pub(crate) struct PatternGraph {
    pub cyclic: Vec<bool>,
    pub stand_ins: Vec<Option<StandIn>>,
}

impl PatternGraph {
    pub fn of(
        hir: &Hir,
        source: &Namespace,
        prelude: &Namespace,
        report: &mut Report,
    ) -> PatternGraph {
        let mut edges: Vec<Vec<usize>> = Vec::with_capacity(hir.patterns.len());
        for (index, decl) in hir.patterns.iter().enumerate() {
            let id = PatternId(u32::try_from(index).unwrap_or(u32::MAX));
            let namespace = if hir.is_prelude_pattern(id) {
                prelude
            } else {
                source
            };
            let mut targets = BTreeSet::new();
            for placement in hir.nested_placements(decl.body) {
                if let Some(kind) = &hir.placement(placement).kind
                    && let Some(Decl::Pattern(target)) = namespace.get(&kind.text)
                {
                    targets.insert(target.index());
                }
            }
            edges.push(targets.into_iter().collect());
        }
        let mut cyclic = vec![false; hir.patterns.len()];
        let mut stand_ins = vec![None; hir.patterns.len()];
        for component in strongly_connected(&edges) {
            let self_use = component.first().is_some_and(|only| {
                edges
                    .get(*only)
                    .is_some_and(|targets| targets.contains(only))
            });
            if component.len() > 1 || self_use {
                Self::report_cycle(hir, &component, report);
                for member in &component {
                    if let Some(flag) = cyclic.get_mut(*member) {
                        *flag = true;
                    }
                }
                continue;
            }
            let Some(&member) = component.first() else {
                continue;
            };
            let id = PatternId(u32::try_from(member).unwrap_or(u32::MAX));
            let namespace = if hir.is_prelude_pattern(id) {
                prelude
            } else {
                source
            };
            let stand_in = match hir.block(hir.pattern(id).body).items.as_slice() {
                [BodyItem::Placement(placement)] => {
                    match hir
                        .placement(*placement)
                        .kind
                        .as_ref()
                        .and_then(|kind| namespace.get(&kind.text))
                    {
                        Some(Decl::Node(_)) => Some(StandIn::Node(*placement)),
                        Some(Decl::Pattern(target))
                            if stand_ins.get(target.index()).is_some_and(Option::is_some) =>
                        {
                            Some(StandIn::Use(*placement))
                        }
                        Some(
                            Decl::Pattern(_)
                            | Decl::Input(_)
                            | Decl::Let(_)
                            | Decl::Fn(_)
                            | Decl::Instance(_)
                            | Decl::Builtin(_)
                            | Decl::Wire(_)
                            | Decl::Enum(_),
                        )
                        | None => None,
                    }
                }
                _ => None,
            };
            if let Some(slot) = stand_ins.get_mut(member) {
                *slot = stand_in;
            }
        }
        PatternGraph { cyclic, stand_ins }
    }

    fn report_cycle(hir: &Hir, component: &[usize], report: &mut Report) {
        let mut members: Vec<PatternId> = component
            .iter()
            .map(|member| PatternId(u32::try_from(*member).unwrap_or(u32::MAX)))
            .collect();
        members.sort();
        let names: Vec<String> = members
            .iter()
            .filter_map(|id| hir.pattern(*id).name.as_ref())
            .map(|name| format!("`{}`", name.text))
            .collect();
        let Some(first) = members.first() else {
            return;
        };
        let decl = hir.pattern(*first);
        let loc = decl.name.as_ref().map_or(decl.loc, |name| name.loc);
        let others: Vec<_> = members
            .iter()
            .skip(1)
            .map(|id| {
                let other = hir.pattern(*id);
                other.name.as_ref().map_or(other.loc, |name| name.loc)
            })
            .collect();
        let message = if names.len() == 1 {
            format!("the pattern {} uses itself", names.join(""))
        } else {
            format!("the patterns {} use each other", names.join(", "))
        };
        report.error_with(hir, Code::RecursivePattern, loc, &others, message);
    }
}
