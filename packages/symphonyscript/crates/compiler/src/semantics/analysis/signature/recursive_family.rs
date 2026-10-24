use crate::diagnostics::Code;
use crate::semantics::analysis::engine::Engine;
use crate::semantics::analysis::family_key::FamilyKey;
use crate::semantics::analysis::ids::{NodeId, ScopeId};
use crate::semantics::analysis::query::Query;
use crate::semantics::analysis::rec_family::RecFamily;
use crate::semantics::analysis::sig::{Kind, Sig, Width};
use crate::semantics::analysis::template_key::{ArgShape, ArgSig};
use crate::semantics::analysis::val::Val;
use crate::semantics::ir::FnId;

impl Engine<'_> {
    pub(super) fn recursive_sig(
        &mut self,
        node: NodeId,
        function: FnId,
        args: &[Option<Val>],
        need: &mut Vec<Query>,
    ) -> Option<Sig> {
        let key = FamilyKey {
            function,
            args: args
                .iter()
                .map(|arg| self.family_shape(arg.as_ref()))
                .collect(),
        };
        loop {
            match self.memo.rec_families.get(&key).copied() {
                None => {
                    let scope = self.instantiate(function, &key.args, node);
                    self.memo.probes.insert(scope);
                    self.memo
                        .rec_families
                        .insert(key.clone(), RecFamily::Probing(scope));
                }
                Some(RecFamily::Probing(scope)) => {
                    let Some(root) = self.template_root(function, scope, need)? else {
                        self.memo
                            .rec_families
                            .insert(key.clone(), RecFamily::Known(None));
                        continue;
                    };
                    let Some(sig) = self.memo.sigs.get(&root).copied() else {
                        return self.pending(root, Sig::open(false), need);
                    };
                    let family = if sig.open { None } else { sig.family };
                    let settled = self.instantiate(function, &key.args, node);
                    self.memo.rec_families.insert(
                        key.clone(),
                        RecFamily::Settling {
                            family,
                            scope: settled,
                        },
                    );
                }
                Some(RecFamily::Settling { family, scope }) => {
                    let Some(root) = self.template_root(function, scope, need)? else {
                        self.memo
                            .rec_families
                            .insert(key.clone(), RecFamily::Known(family));
                        continue;
                    };
                    let assumed = Sig::of(Kind::Any, Width::Flexible, false, family);
                    let Some(sig) = self.memo.sigs.get(&root).copied() else {
                        return self.pending(root, assumed, need);
                    };
                    let state = if sig.is_error() {
                        RecFamily::Failed
                    } else if sig.open || sig.family == family {
                        RecFamily::Known(family)
                    } else {
                        self.error_at_node(
                            Code::MixedFamilies,
                            node,
                            "this recursive call's result takes different unit families through its recursive calls",
                        );
                        RecFamily::Failed
                    };
                    self.memo.rec_families.insert(key.clone(), state);
                }
                Some(RecFamily::Known(family)) => {
                    return Some(Sig::of(Kind::Any, Width::Flexible, false, family));
                }
                Some(RecFamily::Failed) => return Some(Sig::ERROR),
            }
        }
    }

    fn pending(&self, root: NodeId, provisional: Sig, need: &mut Vec<Query>) -> Option<Sig> {
        let query = Query::Sig(root);
        if self.in_progress(query) {
            return Some(provisional);
        }
        need.push(query);
        None
    }

    fn template_root(
        &mut self,
        function: FnId,
        scope: ScopeId,
        need: &mut Vec<Query>,
    ) -> Option<Option<NodeId>> {
        let body = self.hir.fn_decl(function).value;
        match self.val(Query::Expr(body, scope), need)? {
            Val::Scalar(root) => Some(Some(root)),
            Val::Array(_) | Val::Error => Some(None),
        }
    }

    fn family_shape(&self, arg: Option<&Val>) -> ArgShape {
        let sig_of = |node: &NodeId| {
            let sig = self.memo.sigs.get(node).copied().unwrap_or(Sig::ERROR);
            ArgSig {
                kind: sig.kind,
                width: Width::Flexible,
                family: sig.family,
            }
        };
        match arg {
            None => ArgShape::Default,
            Some(Val::Scalar(node)) => ArgShape::Scalar(sig_of(node)),
            Some(Val::Array(array)) => ArgShape::Array(
                array.shape.lengths(),
                array.elements.iter().map(sig_of).collect(),
            ),
            Some(Val::Error) => ArgShape::Default,
        }
    }
}
