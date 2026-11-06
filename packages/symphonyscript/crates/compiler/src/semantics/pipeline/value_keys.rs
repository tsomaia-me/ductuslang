use std::collections::HashSet;

use super::core::Core;
use super::core_value::CoreValue;
use super::decl_ref::DeclRef;
use super::memo_arg::MemoArg;
use crate::semantics::Internal;
use crate::semantics::ir::flat::{NONE, Op};
use crate::semantics::operators::{EvalTy, Value};

const UNSET: u32 = NONE - 1;

pub(crate) type OwnCall = (u32, u32, Vec<CoreValue>);

impl Core<'_> {
    pub fn keyed_params(&mut self, function: u32) -> Result<Vec<bool>, Internal> {
        if self.keyed_params.is_none() {
            self.keyed_params = Some(self.param_keys()?);
        }
        Ok(self
            .keyed_params
            .as_ref()
            .and_then(|keyed| keyed.get(function as usize))
            .cloned()
            .unwrap_or_default())
    }

    fn param_keys(&self) -> Result<Vec<Vec<bool>>, Internal> {
        let functions = self.functions();
        let mut keyed = Vec::with_capacity(functions as usize);
        for function in 0..functions {
            let rec = self
                .ir
                .fns
                .get(function as usize)
                .ok_or(Internal::missed("IR function"))?;
            keyed.push(vec![false; (rec.params.1 - rec.params.0) as usize]);
        }
        let folds = u32::try_from(self.ir.fors.len()).unwrap_or(0);
        let mut fold_nodes = vec![NONE; folds as usize];
        let mut calls: Vec<Vec<u32>> = vec![Vec::new(); functions as usize];
        let mut bounds = Vec::new();
        for local in 0..self.ir.op.len() {
            let node = self.ir.base + u32::try_from(local).unwrap_or(0);
            let op = self.ir.op.get(local).copied();
            if op == Some(Op::For) {
                let (_, record, _, _) = self.ir.node(node)?;
                if let Some(slot) = fold_nodes.get_mut(record as usize) {
                    *slot = node;
                }
                let rec = self.fold_rec(node)?;
                if rec.range {
                    bounds.push((node, rec.iterable.0));
                    bounds.push((node, rec.iterable.1));
                }
            } else if matches!(op, Some(Op::Call | Op::NamedCall))
                && let Some(callee) = self.callee(node)?
                && let Some(list) = calls.get_mut(callee as usize)
            {
                list.push(node);
            }
        }
        let mut owners = vec![UNSET; folds as usize];
        let mut work = Vec::new();
        for (fold, bound) in bounds {
            let function = self.function_of(fold, &fold_nodes, &mut owners)?;
            if function == NONE {
                continue;
            }
            for param in self.params_read(bound, function, &fold_nodes, &mut owners)? {
                mark(&mut keyed, &mut work, function, param);
            }
        }
        while let Some((callee, param)) = work.pop() {
            for call in calls.get(callee as usize).cloned().unwrap_or_default() {
                let Some(argument) = self.arguments(call)?.get(param as usize).copied() else {
                    continue;
                };
                let function = self.function_of(call, &fold_nodes, &mut owners)?;
                if function == NONE {
                    continue;
                }
                for read in self.params_read(argument, function, &fold_nodes, &mut owners)? {
                    mark(&mut keyed, &mut work, function, read);
                }
            }
        }
        Ok(keyed)
    }

    fn function_of(
        &self,
        node: u32,
        fold_nodes: &[u32],
        owners: &mut [u32],
    ) -> Result<u32, Internal> {
        let functions = self.functions();
        let mut owner = self.body_of.get(self.local(node)?).copied().unwrap_or(NONE);
        let mut passed = Vec::new();
        let found = loop {
            if owner == NONE || owner < functions {
                break owner;
            }
            let fold = owner - functions;
            let Some(fold_node) = fold_nodes.get(fold as usize).copied() else {
                break NONE;
            };
            if let Some(known) = owners.get(fold as usize).copied()
                && known != UNSET
            {
                break known;
            }
            passed.push(fold);
            if fold_node == NONE {
                break NONE;
            }
            owner = self
                .body_of
                .get(self.local(fold_node)?)
                .copied()
                .unwrap_or(NONE);
        };
        for fold in passed {
            if let Some(slot) = owners.get_mut(fold as usize) {
                *slot = found;
            }
        }
        Ok(found)
    }

    fn params_read(
        &self,
        root: u32,
        function: u32,
        fold_nodes: &[u32],
        owners: &mut [u32],
    ) -> Result<Vec<u32>, Internal> {
        let mut found = Vec::new();
        let mut roots = vec![root];
        let mut lets = HashSet::new();
        while let Some(root) = roots.pop() {
            for node in self.postorder(root)? {
                if self.ir.node(node)?.0 != Op::Name {
                    continue;
                }
                match self.bound(node)? {
                    DeclRef::Param(position) if self.binding_owner(node)? == function => {
                        found.push(position);
                    }
                    DeclRef::Let(at) => {
                        let value = self.let_root(at)?;
                        if lets.insert(at)
                            && self.function_of(value, fold_nodes, owners)? == function
                        {
                            roots.push(value);
                        }
                    }
                    DeclRef::Param(_)
                    | DeclRef::None
                    | DeclRef::Cell(_)
                    | DeclRef::Instance(_)
                    | DeclRef::Duplicate
                    | DeclRef::Fn(_)
                    | DeclRef::Pattern(_) => {}
                }
            }
        }
        Ok(found)
    }

    pub fn instance_frames(&self, inst: u32) -> Vec<OwnCall> {
        let mut frames = Vec::new();
        let mut at = inst;
        while let Some(body) = self.bodies.get(at as usize) {
            frames.push((NONE, at, body.values.clone()));
            at = body.parent;
        }
        frames.reverse();
        frames
    }

    pub fn key_values(
        &mut self,
        function: u32,
        args: &[u32],
        reactive: &[bool],
    ) -> Result<Vec<CoreValue>, Internal> {
        let keyed = self.keyed_params(function)?;
        if !keyed.iter().any(|keyed| *keyed) {
            return Ok(Vec::new());
        }
        let frames = self.instance_frames(self.inst);
        let depth = self.own_calls.len();
        self.own_calls.extend(frames);
        let mut values = Vec::with_capacity(args.len());
        let mut found = Ok(());
        for (at, argument) in args.iter().enumerate() {
            let wanted = keyed.get(at).copied().unwrap_or(false)
                && !reactive.get(at).copied().unwrap_or(true);
            if !wanted {
                values.push(Value::Error);
                continue;
            }
            let chain = std::mem::take(&mut self.eval_chain);
            let (drawn, stop) = (self.diagnostics.len(), self.stop.clone());
            let value = self.evaluate(*argument, EvalTy::Exact);
            self.diagnostics.truncate(drawn);
            self.stop = stop;
            self.eval_chain = chain;
            match value {
                Ok(value) => values.push(value.unwrap_or(Value::Error)),
                Err(internal) => {
                    found = Err(internal);
                    break;
                }
            }
        }
        self.own_calls.truncate(depth);
        found?;
        if values.iter().all(|value| MemoArg::of(value).is_none()) {
            return Ok(Vec::new());
        }
        Ok(values)
    }
}

fn mark(keyed: &mut [Vec<bool>], work: &mut Vec<(u32, u32)>, function: u32, param: u32) {
    if let Some(slot) = keyed
        .get_mut(function as usize)
        .and_then(|params| params.get_mut(param as usize))
        && !*slot
    {
        *slot = true;
        work.push((function, param));
    }
}
