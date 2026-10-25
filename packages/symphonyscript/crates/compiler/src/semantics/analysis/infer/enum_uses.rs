use std::collections::BTreeMap;

use symphonyscript_core::EnumId;

use crate::diagnostics::Code;
use crate::semantics::Internal;
use crate::semantics::analysis::engine::Engine;
use crate::semantics::analysis::eval_ty::EvalTy;
use crate::semantics::analysis::ids::NodeId;
use crate::semantics::analysis::kind::Kind;
use crate::semantics::analysis::op::Op;
use crate::semantics::ir::Loc;

impl Engine<'_> {
    pub(super) fn enum_uses(&mut self) -> Result<(), Internal> {
        let mut uses: BTreeMap<NodeId, Vec<(NodeId, EnumId)>> = BTreeMap::new();
        for ((node, ty), value) in &self.memo.evals {
            let EvalTy::Enum(id) = ty else {
                continue;
            };
            if value.is_error() {
                continue;
            }
            let Op::Use(inner) = self.node(*node)?.op else {
                continue;
            };
            let source = self.through_uses(inner)?;
            let bare = self
                .memo
                .sigs
                .get(&source)
                .is_some_and(|sig| !sig.reactive && sig.kind == Kind::Bare);
            if bare {
                uses.entry(source).or_default().push((*node, *id));
            }
        }
        for (source, mut found) in uses {
            found.sort();
            let Some(&(first, first_enum)) = found.first() else {
                continue;
            };
            let Some(&(other, _)) = found.iter().find(|(_, id)| *id != first_enum) else {
                continue;
            };
            self.typing.untyped_values.insert(source);
            let (first_loc, other_loc) = (self.node(first)?.loc, self.node(other)?.loc);
            let scope = self.node(source)?.scope;
            let (loc, secondary) = match self.writer_of(source)? {
                Some(writer) => (writer, vec![first_loc, other_loc]),
                None => (first_loc, vec![other_loc]),
            };
            self.error_with_chain(
                Code::TypeMismatch,
                loc,
                secondary,
                scope,
                "this variant takes its enum from its uses, and they expect different enums",
            )?;
        }
        Ok(())
    }

    fn writer_of(&self, source: NodeId) -> Result<Option<Loc>, Internal> {
        let at = self.node(source)?.loc;
        let mut found: Option<(u32, Loc)> = None;
        let keys: Vec<_> = self.memo.lets.keys().copied().collect();
        for (scope, local) in keys {
            let Some(decl) = self.let_decl(scope, local)? else {
                continue;
            };
            let value = self.hir.expr(decl.value)?.loc;
            if !value.contains(at) {
                continue;
            }
            let width = value.end.saturating_sub(value.start);
            if found.is_none_or(|(best, _)| width < best) {
                found = Some((width, decl.loc));
            }
        }
        Ok(found.map(|(_, loc)| loc))
    }
}
