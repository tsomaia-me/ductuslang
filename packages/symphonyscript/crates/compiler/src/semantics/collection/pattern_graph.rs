use std::collections::BTreeSet;

use super::decl::Decl;
use super::is_cycle::is_cycle;
use super::namespace::Namespace;
use super::stand_in::StandIn;
use super::strongly_connected::strongly_connected;
use crate::diagnostics::Code;
use crate::semantics::Internal;
use crate::semantics::ir::{BodyItem, Hir, PatternId};
use crate::semantics::reporting::Report;

pub(crate) struct PatternGraph {
    pub cyclic: Vec<bool>,
    pub stand_ins: Vec<Option<StandIn>>,
    pub placeholders: Vec<bool>,
}

impl PatternGraph {
    pub fn of(
        hir: &Hir,
        source: &Namespace,
        prelude: &Namespace,
        report: &mut Report,
    ) -> Result<PatternGraph, Internal> {
        let mut edges: Vec<Vec<usize>> = Vec::with_capacity(hir.patterns.len());
        for (index, decl) in hir.patterns.iter().enumerate() {
            let id = PatternId(u32::try_from(index).unwrap_or(u32::MAX));
            let namespace = if hir.is_prelude_pattern(id) {
                prelude
            } else {
                source
            };
            let mut targets = BTreeSet::new();
            for placement in hir.nested_placements(decl.body)? {
                if let Some(kind) = &hir.placement(placement)?.kind
                    && let Some(Decl::Pattern(target)) = namespace.get(&kind.text)
                {
                    targets.insert(target.index());
                }
            }
            edges.push(targets.into_iter().collect());
        }
        let mut cyclic = vec![false; hir.patterns.len()];
        let mut stand_ins = vec![None; hir.patterns.len()];
        let mut placeholders = vec![false; hir.patterns.len()];
        for component in strongly_connected(&edges)? {
            if is_cycle(&component, &edges)? {
                Self::report_cycle(hir, &component, report)?;
                for member in &component {
                    *cyclic.get_mut(*member).ok_or(Internal::missed("pattern"))? = true;
                }
                continue;
            }
            let &[member] = component.as_slice() else {
                return Err(Internal::missed("graph component"));
            };
            let id = PatternId(u32::try_from(member).unwrap_or(u32::MAX));
            let namespace = if hir.is_prelude_pattern(id) {
                prelude
            } else {
                source
            };
            let mut placeholder = false;
            let stand_in = match hir.block(hir.pattern(id)?.body)?.items.as_slice() {
                [BodyItem::Placement(placement)]
                    if hir
                        .placement(*placement)?
                        .kind
                        .as_ref()
                        .is_some_and(|kind| namespace.is_duplicated(&kind.text)) =>
                {
                    placeholder = true;
                    None
                }
                [] => {
                    placeholder = true;
                    None
                }
                [BodyItem::Placement(placement)] => {
                    match hir
                        .placement(*placement)?
                        .kind
                        .as_ref()
                        .and_then(|kind| namespace.get(&kind.text))
                    {
                        Some(Decl::Node(_)) => Some(StandIn::Node(*placement)),
                        Some(Decl::Pattern(target)) => {
                            placeholder = *placeholders
                                .get(target.index())
                                .ok_or(Internal::missed("pattern"))?;
                            stand_ins
                                .get(target.index())
                                .ok_or(Internal::missed("pattern"))?
                                .as_ref()
                                .map(|_| StandIn::Use(*placement))
                        }
                        Some(
                            Decl::Input(_)
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
            *stand_ins
                .get_mut(member)
                .ok_or(Internal::missed("pattern"))? = stand_in;
            *placeholders
                .get_mut(member)
                .ok_or(Internal::missed("pattern"))? = placeholder;
        }
        Ok(PatternGraph {
            cyclic,
            stand_ins,
            placeholders,
        })
    }

    fn report_cycle(hir: &Hir, component: &[usize], report: &mut Report) -> Result<(), Internal> {
        let mut members: Vec<PatternId> = component
            .iter()
            .map(|member| PatternId(u32::try_from(*member).unwrap_or(u32::MAX)))
            .collect();
        members.sort();
        let decls = members
            .iter()
            .map(|id| hir.pattern(*id))
            .collect::<Result<Vec<_>, _>>()?;
        let names: Vec<String> = decls
            .iter()
            .filter_map(|decl| decl.name.as_ref())
            .map(|name| format!("`{}`", name.text))
            .collect();
        let decl = decls.first().ok_or(Internal::missed("graph component"))?;
        let loc = decl.name.as_ref().map_or(decl.loc, |name| name.loc);
        let others: Vec<_> = decls
            .iter()
            .skip(1)
            .map(|other| other.name.as_ref().map_or(other.loc, |name| name.loc))
            .collect();
        let message = if names.len() == 1 {
            format!("the pattern {} uses itself", names.join(""))
        } else {
            format!("the patterns {} use each other", names.join(", "))
        };
        report.error_with(hir, Code::RecursivePattern, loc, &others, message);
        Ok(())
    }
}
