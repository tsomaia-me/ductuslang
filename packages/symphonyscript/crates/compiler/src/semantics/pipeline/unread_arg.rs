use std::collections::HashSet;

use super::core::{Core, Stage};
use super::decl_ref::DeclRef;
use super::leaf::Leaf;
use super::values::flexible;
use crate::semantics::Internal;
use crate::semantics::ir::flat::{NONE, Op};
use crate::semantics::operators::{EvalTy, int_range};

impl Core<'_> {
    pub fn check_unread_arguments(&mut self) -> Stage<()> {
        let mut unread = Vec::new();
        for (env, args) in &self.use_args {
            let Some(record) = self.envs.get(*env as usize) else {
                continue;
            };
            let Some(body) = self.bodies.get(record.inst as usize) else {
                continue;
            };
            for (position, (root, caller)) in args.iter().copied().enumerate() {
                if !(caller == NONE && self.default_nodes.contains_key(&root))
                    && body.reactive_args.get(position).copied().unwrap_or(false)
                {
                    let position = u32::try_from(position).unwrap_or(u32::MAX);
                    unread.push((body.function, position, root, caller));
                }
            }
        }
        if unread.is_empty() {
            return Ok(Ok(()));
        }
        let read = self.read_params()?;
        unread.sort_unstable();
        unread.dedup();
        for (owner, position, root, caller) in unread {
            if read.contains(&(owner, position)) {
                continue;
            }
            let fact = self.fact(root)?;
            if !fact.dims.is_empty() {
                continue;
            }
            let Some(own) = self.own_ty(root)? else {
                continue;
            };
            let (ty, owning) = match fact.leaf {
                Leaf::Float if fact.fixed.is_none() => (EvalTy::F32, false),
                Leaf::Int if int_range(own).is_some() => (own, true),
                Leaf::Int | Leaf::Float => (flexible(own), false),
                _ => continue,
            };
            let saved = self.enter_env(caller);
            let owning = std::mem::replace(&mut self.owning, owning);
            let chain = std::mem::take(&mut self.eval_chain);
            let outcome = self.evaluate(root, ty);
            self.eval_chain = chain;
            self.owning = owning;
            self.leave_env(saved);
            if let Err(declined) = outcome? {
                return Ok(Err(declined));
            }
        }
        Ok(Ok(()))
    }

    fn read_params(&self) -> Result<HashSet<(u32, u32)>, Internal> {
        let mut read = HashSet::new();
        let base = self.ir.end() - u32::try_from(self.ir.op.len()).unwrap_or(0);
        for local in 0..self.ir.op.len() {
            if self.ir.op.get(local).copied() != Some(Op::Name) {
                continue;
            }
            let node = base + u32::try_from(local).unwrap_or(0);
            if let DeclRef::Param(position) = self.bound(node)? {
                read.insert((self.binding_owner(node)?, position));
            }
        }
        Ok(read)
    }
}
