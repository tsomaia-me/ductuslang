use crate::diagnostics::Code;
use crate::semantics::Internal;
use crate::semantics::analysis::arg_reference::ArgReference;
use crate::semantics::analysis::arg_shape::ArgShape;
use crate::semantics::analysis::arg_sig::ArgSig;
use crate::semantics::analysis::c_val::CVal;
use crate::semantics::analysis::engine::Engine;
use crate::semantics::analysis::eval_ty::EvalTy;
use crate::semantics::analysis::family_arg::FamilyArg;
use crate::semantics::analysis::family_key::FamilyKey;
use crate::semantics::analysis::ids::{NodeId, ScopeId};
use crate::semantics::analysis::kind::Kind;
use crate::semantics::analysis::op::Op;
use crate::semantics::analysis::query::Query;
use crate::semantics::analysis::rec_family::RecFamily;
use crate::semantics::analysis::sig::Sig;
use crate::semantics::analysis::some::some;
use crate::semantics::analysis::unify_kinds::unify_kinds;
use crate::semantics::analysis::val::Val;
use crate::semantics::analysis::width::Width;
use crate::semantics::ir::FnId;

impl Engine<'_> {
    pub(super) fn recursive_sig(
        &mut self,
        node: NodeId,
        function: FnId,
        args: &[Option<Val>],
        need: &mut Vec<Query>,
    ) -> Result<Option<Sig>, Internal> {
        let (key, shapes) = some!(self.family_key(function, args, need)?);
        self.memo.call_families.insert(node, key.clone());
        if self.function_reads_cell(function)?.is_cell() {
            if !self.hir.fn_decl(function)?.faulty_head {
                let reading = format!(
                    "the body of the recursive function `{}` reads a cell",
                    self.function_name(function)?
                );
                self.reactive_recursion(node, function, (reading, false))?;
            }
            return Ok(Some(Sig::ERROR));
        }
        let sig = self.family_sig(node, (&key, &shapes), need)?;
        self.link_family(node, &key)?;
        Ok(sig)
    }

    pub(in crate::semantics::analysis) fn chosen_family(
        &mut self,
        call: NodeId,
        function: FnId,
        shapes: &[ArgShape],
        need: &mut Vec<Query>,
    ) -> Result<Option<Option<ScopeId>>, Internal> {
        let key = FamilyKey {
            function,
            args: shapes.iter().map(family_arg).collect(),
        };
        some!(self.family_sig(call, (&key, shapes), need)?);
        let settled = match self.memo.rec_families.get(&key) {
            Some(RecFamily::Known(..) | RecFamily::Failed) => true,
            Some(RecFamily::Probing(_) | RecFamily::Settling { .. }) | None => false,
        };
        if !settled {
            return Ok(Some(None));
        }
        let scope = match self
            .memo
            .family_scopes
            .get(&key)
            .and_then(|scopes| scopes.last())
        {
            Some(scope) => *scope,
            None => {
                let scope = self.instantiate(function, shapes, call)?;
                self.memo
                    .family_scopes
                    .entry(key.clone())
                    .or_default()
                    .push(scope);
                scope
            }
        };
        self.link_family(call, &key)?;
        let body = self.hir.fn_decl(function)?.value;
        some!(self.val(Query::Expr(body, scope), need));
        Ok(Some(Some(scope)))
    }

    fn link_family(&mut self, node: NodeId, key: &FamilyKey) -> Result<(), Internal> {
        let Some(scopes) = self.memo.family_scopes.get(key).cloned() else {
            return Ok(());
        };
        let scope = self.node(node)?.scope;
        if self.in_probe(scope)? {
            return Ok(());
        }
        let caller = self.template_of(scope)?;
        for settled in scopes {
            match caller {
                Some(template) if template == settled => {}
                Some(template) => {
                    let parents = self.memo.template_parents.entry(settled).or_default();
                    if !parents.contains(&template) {
                        parents.push(template);
                    }
                }
                None => self
                    .memo
                    .template_sites
                    .entry(settled)
                    .or_default()
                    .push(node),
            }
        }
        Ok(())
    }

    fn family_sig(
        &mut self,
        node: NodeId,
        (key, shapes): (&FamilyKey, &[ArgShape]),
        need: &mut Vec<Query>,
    ) -> Result<Option<Sig>, Internal> {
        let function = key.function;
        loop {
            match self.memo.rec_families.get(key).copied() {
                None => {
                    if self.memo.probing_families.len() as u64 >= self.limits.nesting_depth {
                        self.memo
                            .rec_families
                            .insert(key.clone(), RecFamily::Known(Kind::Any, None));
                        continue;
                    }
                    self.memo.family_shapes.insert(key.clone(), shapes.to_vec());
                    let scope = self.instantiate(function, shapes, node)?;
                    self.memo.probes.insert(scope);
                    self.memo.family_probes.insert(key.clone(), scope);
                    self.memo.probing_families.insert(key.clone());
                    self.memo
                        .rec_families
                        .insert(key.clone(), RecFamily::Probing(scope));
                }
                Some(RecFamily::Probing(scope)) => {
                    if self.body_in_progress(function, scope)? {
                        return Ok(Some(Sig::open(false)));
                    }
                    let Some(root) = some!(self.template_root(function, scope, need)?) else {
                        self.memo.probing_families.remove(key);
                        self.memo
                            .rec_families
                            .insert(key.clone(), RecFamily::Known(Kind::Any, None));
                        continue;
                    };
                    let Some(sig) = self.memo.sigs.get(&root).copied() else {
                        return Ok(self.pending(root, Sig::open(false), need));
                    };
                    self.memo.probing_families.remove(key);
                    if sig.open
                        && !sig.is_error()
                        && !self.memo.probing_families.is_empty()
                        && self.in_probe(self.node(node)?.scope)?
                    {
                        self.memo.rec_families.remove(key);
                        return Ok(Some(Sig::open(false)));
                    }
                    let family = if sig.open { None } else { sig.family };
                    let kind = if sig.is_error() { Kind::Any } else { sig.kind };
                    let settled = self.instantiate(function, &self.family_args(key)?, node)?;
                    self.memo
                        .family_scopes
                        .entry(key.clone())
                        .or_default()
                        .push(settled);
                    self.memo.rec_families.insert(
                        key.clone(),
                        RecFamily::Settling {
                            kind,
                            family,
                            scope: settled,
                        },
                    );
                }
                Some(RecFamily::Settling {
                    kind,
                    family,
                    scope,
                }) => {
                    let assumed = Sig::of(kind, Width::Flexible, false, family);
                    if self.body_in_progress(function, scope)? {
                        return Ok(Some(assumed));
                    }
                    let Some(root) = some!(self.template_root(function, scope, need)?) else {
                        self.memo
                            .rec_families
                            .insert(key.clone(), RecFamily::Known(kind, family));
                        continue;
                    };
                    let Some(sig) = self.memo.sigs.get(&root).copied() else {
                        return Ok(self.pending(root, assumed, need));
                    };
                    let state = if sig.is_error() {
                        RecFamily::Failed
                    } else if rises(kind, sig.kind) && (sig.open || sig.family == family) {
                        let settled = self.instantiate(function, &self.family_args(key)?, node)?;
                        self.memo
                            .family_scopes
                            .entry(key.clone())
                            .or_default()
                            .push(settled);
                        RecFamily::Settling {
                            kind: sig.kind,
                            family,
                            scope: settled,
                        }
                    } else if sig.open || sig.family == family {
                        RecFamily::Known(kind, family)
                    } else {
                        self.error_at_node(
                            Code::MixedFamilies,
                            node,
                            "this recursive call's result takes different unit families through its recursive calls",
                        )?;
                        RecFamily::Failed
                    };
                    self.memo.rec_families.insert(key.clone(), state);
                }
                Some(RecFamily::Known(kind, family)) => {
                    return Ok(Some(Sig::of(kind, Width::Flexible, false, family)));
                }
                Some(RecFamily::Failed) => {
                    return Ok(Some(Sig::ERROR));
                }
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

    fn body_in_progress(&self, function: FnId, scope: ScopeId) -> Result<bool, Internal> {
        let body = Query::Expr(self.hir.fn_decl(function)?.value, scope);
        Ok(self.poisoned(body) || self.in_progress(body))
    }

    fn template_root(
        &mut self,
        function: FnId,
        scope: ScopeId,
        need: &mut Vec<Query>,
    ) -> Result<Option<Option<NodeId>>, Internal> {
        let body = self.hir.fn_decl(function)?.value;
        let val = some!(self.val(Query::Expr(body, scope), need));
        Ok(Some(self.boxed_body(body, scope, &val)?))
    }

    pub(in crate::semantics::analysis) fn family_key(
        &self,
        function: FnId,
        args: &[Option<Val>],
        need: &mut Vec<Query>,
    ) -> Result<Option<(FamilyKey, Vec<ArgShape>)>, Internal> {
        let mut shapes = Vec::with_capacity(args.len());
        let mut missing = false;
        for arg in args {
            match self.family_shape(arg.as_ref(), need)? {
                Some(shape) => shapes.push(shape),
                None => missing = true,
            }
        }
        if missing {
            return Ok(None);
        }
        let key = FamilyKey {
            function,
            args: shapes.iter().map(family_arg).collect(),
        };
        Ok(Some((key, shapes)))
    }

    fn family_args(&self, key: &FamilyKey) -> Result<Vec<ArgShape>, Internal> {
        self.memo
            .family_shapes
            .get(key)
            .cloned()
            .ok_or(Internal::missed("family shapes"))
    }

    pub(in crate::semantics::analysis) fn family_shape(
        &self,
        arg: Option<&Val>,
        need: &mut Vec<Query>,
    ) -> Result<Option<ArgShape>, Internal> {
        let Some(arg) = arg else {
            return Ok(Some(ArgShape::Default));
        };
        if let Val::Scalar(node) = arg
            && self.arrays.contains(node)
            && matches!(self.node(*node)?.op, Op::Slice { .. })
        {
            let sig = self.known_sig(*node)?;
            if sig.kind == Kind::Node && !sig.reactive {
                let instances = some!(self.reference_instances(*node, need)?);
                if let Ok(length) = u32::try_from(instances.len())
                    && length > 0
                {
                    let sigs = instances
                        .into_iter()
                        .map(|item| ArgSig {
                            kind: Kind::Node,
                            width: Width::Flexible,
                            family: sig.family,
                            node: ArgReference::Instance(item),
                        })
                        .collect();
                    return Ok(Some(ArgShape::Array(vec![length], sigs)));
                }
            }
        }
        let mut sigs = Vec::new();
        let mut missing = false;
        for node in arg.nodes() {
            crate::semantics::counted_walk::CountedWalk::RecursiveFamilyFamilyShape.step();
            let sig = self.known_sig(node)?;
            let reference = match self.arg_reference(node, &sig, need)? {
                Some(reference) => reference,
                None => {
                    missing = true;
                    ArgReference::Unknown
                }
            };
            sigs.push(ArgSig {
                kind: sig.kind,
                width: Width::Flexible,
                family: sig.family,
                node: reference,
            });
        }
        if missing {
            return Ok(None);
        }
        Ok(Some(match arg {
            Val::Scalar(_) => match sigs.pop() {
                Some(sig) => ArgShape::Scalar(sig),
                None => return Err(Internal::missed("argument signature")),
            },
            Val::Array(array) => ArgShape::Array(array.shape.lengths(), sigs),
            Val::Error => ArgShape::Default,
        }))
    }

    fn arg_reference(
        &self,
        node: NodeId,
        sig: &Sig,
        need: &mut Vec<Query>,
    ) -> Result<Option<ArgReference>, Internal> {
        if sig.kind != Kind::Node || sig.reactive {
            return Ok(Some(ArgReference::Unknown));
        }
        if sig.frame {
            let instances = some!(self.reference_instances(node, need)?);
            return Ok(Some(match instances.as_slice() {
                [instance] => ArgReference::Instance(*instance),
                _ => ArgReference::Among(instances.into()),
            }));
        }
        Ok(Some(
            match some!(self.untaken_cval(Query::Eval(node, EvalTy::Node), need)) {
                CVal::Node(item) => ArgReference::Instance(item),
                CVal::Int(_)
                | CVal::F32(_)
                | CVal::F64(_)
                | CVal::Bool(_)
                | CVal::Enum(..)
                | CVal::Str(_)
                | CVal::Array(_)
                | CVal::Error => ArgReference::Unknown,
            },
        ))
    }
}

fn family_arg(shape: &ArgShape) -> FamilyArg {
    match shape {
        ArgShape::Default => FamilyArg::Default,
        ArgShape::Scalar(sig) => FamilyArg::Scalar(sig.clone()),
        ArgShape::Array(lengths, sigs) => FamilyArg::Array {
            rows: lengths
                .split_last()
                .map(|(_, rows)| rows.to_vec())
                .unwrap_or_default(),
            element: sigs
                .iter()
                .cloned()
                .reduce(|one, other| ArgSig {
                    kind: unify_kinds(one.kind, other.kind).unwrap_or(Kind::Error),
                    width: Width::Flexible,
                    family: if one.family == other.family {
                        one.family
                    } else {
                        None
                    },
                    node: ArgReference::Unknown,
                })
                .map(|element| ArgSig {
                    node: element_references(sigs),
                    ..element
                }),
        },
    }
}

fn element_references(sigs: &[ArgSig]) -> ArgReference {
    let mut items = Vec::with_capacity(sigs.len());
    for sig in sigs {
        match &sig.node {
            ArgReference::Unknown => return ArgReference::Unknown,
            ArgReference::Instance(item) => items.push(*item),
            ArgReference::Among(among) => items.extend(among.iter().copied()),
        }
    }
    items.sort_unstable();
    items.dedup();
    match items.as_slice() {
        [] => ArgReference::Unknown,
        [item] => ArgReference::Instance(*item),
        _ => ArgReference::Among(items.into()),
    }
}

fn rises(assumed: Kind, found: Kind) -> bool {
    found != assumed && unify_kinds(assumed, found) == Some(found)
}
