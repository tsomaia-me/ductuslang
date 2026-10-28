use symphonyscript_core::Vocabulary;

use super::checker::Checker;
use crate::semantics::Internal;
use crate::semantics::collection::{Collected, StandIn, Variants};
use crate::semantics::ir::{ExprId, Hir, Top};
use crate::semantics::reporting::Report;

pub(crate) fn check_scopes(
    hir: &Hir,
    collected: &Collected,
    vocabulary: &Vocabulary,
    variants: &Variants,
    prelude: bool,
    report: &mut Report,
) -> Result<Vec<ExprId>, Internal> {
    let mut placeholders = Vec::new();
    for (position, top) in hir.top.iter().enumerate() {
        if hir.is_prelude_top(position) != prelude {
            continue;
        }
        let namespace = if prelude {
            &collected.prelude
        } else {
            &collected.source
        };
        let mut checker = Checker::new(hir, vocabulary, variants, namespace, report);
        match *top {
            Top::Input(id) => {
                let input = hir.input(id)?;
                if let Some(ty) = &input.ty {
                    checker.type_ref(ty);
                }
                checker.expr(input.value, None, false)?;
            }
            Top::Let(id) => {
                let decl = hir.let_decl(id)?;
                if let Some(ty) = &decl.ty {
                    checker.type_ref(ty);
                }
                checker.expr(decl.value, None, false)?;
            }
            Top::State(id) => checker.state(id)?,
            Top::Fn(id) => checker.function(id)?,
            Top::Pattern(id) => {
                let stand_in = collected.stand_ins.get(id.index()).copied().flatten();
                checker.pattern(id, stand_in.map(StandIn::placement))?
            }
            Top::Placement(id) => checker.top_placement(id)?,
        }
        placeholders.extend(checker.into_placeholders());
    }
    Ok(placeholders)
}
