use std::collections::BTreeSet;

use crate::semantics::Internal;
use crate::semantics::analysis::bound::Bound;
use crate::semantics::analysis::bound_val::BoundVal;
use crate::semantics::analysis::engine::Engine;
use crate::semantics::analysis::fast_hash::FastSet;
use crate::semantics::analysis::let_ref::LetRef;
use crate::semantics::analysis::query::Query;
use crate::semantics::collection::Decl;
use crate::semantics::ir::{ExprKind, FnId};

impl Engine<'_> {
    pub(super) fn body_reads_cell(
        &mut self,
        root: FnId,
        need: &mut Vec<Query>,
    ) -> Result<Option<bool>, Internal> {
        if let Some(known) = self.memo.reads_cell.get(&root) {
            return Ok(Some(*known));
        }
        let hir = self.hir;
        let mut seen = FastSet::default();
        let mut functions = vec![root];
        let mut reads = false;
        let mut missing = false;
        while let Some(function) = functions.pop() {
            if !seen.insert(function) {
                continue;
            }
            let decl = hir.fn_decl(function)?;
            let (namespace, global) = if hir.is_prelude_fn(function) {
                (&self.collected.prelude, self.prelude_global)
            } else {
                (&self.collected.source, self.source_global)
            };
            let locals: BTreeSet<&str> = decl
                .params
                .iter()
                .filter_map(|param| param.name.as_ref())
                .chain(decl.lets.iter().filter_map(|local| local.name.as_ref()))
                .map(|name| name.text.as_str())
                .collect();
            let mut stack: Vec<_> = decl
                .params
                .iter()
                .filter_map(|param| param.default)
                .collect();
            stack.extend(decl.lets.iter().map(|local| local.value));
            stack.push(decl.value);
            stack.extend(decl.bound.iter().map(|bound| bound.fallback));
            while let Some(expr) = stack.pop() {
                let kind = &hir.expr(expr)?.kind;
                match kind {
                    ExprKind::Name(name) if !locals.contains(name.as_str()) => {
                        match namespace.get(name) {
                            Some(Decl::Input(_) | Decl::State(_)) => reads = true,
                            Some(Decl::Let(id)) => {
                                match self.bound(Query::Let(global, LetRef::Top(id)), need) {
                                    Some(bound) => reads |= holds_cell(&bound),
                                    None => missing = true,
                                }
                            }
                            Some(Decl::Fn(callee)) => functions.push(callee),
                            Some(
                                Decl::Native(_)
                                | Decl::Pattern(_)
                                | Decl::Instance(_)
                                | Decl::Builtin(_)
                                | Decl::Node(_)
                                | Decl::Wire(_)
                                | Decl::Enum(_),
                            )
                            | None => {}
                        }
                    }
                    ExprKind::Field(..) => reads = true,
                    _ => {}
                }
                stack.extend(kind.children());
            }
        }
        if missing {
            return Ok(None);
        }
        self.memo.reads_cell.insert(root, reads);
        Ok(Some(reads))
    }
}

pub(super) fn holds_cell(bound: &BoundVal) -> bool {
    match bound {
        BoundVal::Scalar(Bound::Cell(_)) => true,
        BoundVal::Array(array) => array.1.iter().any(|item| matches!(item, Bound::Cell(_))),
        BoundVal::Scalar(Bound::Value(_)) | BoundVal::Error => false,
    }
}
