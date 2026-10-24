use symphonyscript_core::Vocabulary;

use super::checker::Checker;
use crate::semantics::collection::Collected;
use crate::semantics::ir::{Hir, Top};
use crate::semantics::reporting::Report;

pub(crate) fn check_scopes(
    hir: &Hir,
    collected: &Collected,
    vocabulary: &Vocabulary,
    prelude: bool,
    report: &mut Report,
) {
    for (position, top) in hir.top.iter().enumerate() {
        if hir.is_prelude_top(position) != prelude {
            continue;
        }
        let namespace = if prelude {
            &collected.prelude
        } else {
            &collected.source
        };
        let mut checker = Checker::new(hir, vocabulary, namespace, report);
        match *top {
            Top::Input(id) => {
                let input = hir.input(id);
                if let Some(ty) = &input.ty {
                    checker.type_ref(ty);
                }
                checker.expr(input.value, None, false);
            }
            Top::Let(id) => {
                let decl = hir.let_decl(id);
                if let Some(ty) = &decl.ty {
                    checker.type_ref(ty);
                }
                checker.expr(decl.value, None, false);
            }
            Top::Fn(id) => checker.function(id),
            Top::Pattern(id) => checker.pattern(id),
            Top::Placement(id) => checker.top_placement(id),
        }
    }
}
