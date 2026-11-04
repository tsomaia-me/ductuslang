use std::collections::HashMap;

use super::core::Core;
use super::decl_ref::DeclRef;
use super::fact::Fact;
use crate::semantics::Internal;
use crate::semantics::counted_walk::CountedWalk;
use crate::semantics::ir::flat::{NONE, Op};
use crate::semantics::operators::EvalTy;

type Binding = Option<(EvalTy, bool, bool, bool, Option<Fact>, Option<EvalTy>, u32)>;

impl Core<'_> {
    pub fn share_ref_bodies(&mut self) -> Result<(), Internal> {
        let mut reads: HashMap<u32, Vec<u32>> = HashMap::new();
        let mut nodes: Vec<u32> = self.per_inst_paths.iter().copied().collect();
        nodes.sort_unstable();
        for node in nodes {
            let receiver = self.ir.a.get(self.local(node)?).copied().unwrap_or(NONE);
            reads
                .entry(self.binding_owner(receiver)?)
                .or_default()
                .push(node);
        }
        let mut insts: Vec<(u32, u32)> = self
            .owner_insts
            .iter()
            .flat_map(|(owner, insts)| insts.iter().map(|inst| (*inst, *owner)))
            .collect();
        insts.sort_unstable();
        let mut typed: HashMap<(u32, u32, Vec<Binding>), u32> = HashMap::new();
        for (inst, owner) in insts {
            let mut signature = Vec::new();
            for node in reads.get(&owner).map(Vec::as_slice).unwrap_or_default() {
                let at = self.inst_reads.get(&(*node, inst)).copied();
                signature.push(match at {
                    Some(at) => self.binding(at)?,
                    None => None,
                });
            }
            let parent = self
                .bodies
                .get(inst as usize)
                .map_or(NONE, |body| body.parent);
            let parent = self.typed_as.get(&parent).copied().unwrap_or(parent);
            match typed.get(&(owner, parent, signature.clone())).copied() {
                Some(shared) => {
                    self.typed_as.insert(inst, shared);
                }
                None => {
                    CountedWalk::CoreLoopBody.step();
                    typed.insert((owner, parent, signature), inst);
                }
            }
        }
        Ok(())
    }

    fn binding(&self, at: u32) -> Result<Binding, Internal> {
        let Some(read) = self.attr_reads.get(at as usize) else {
            return Ok(None);
        };
        if read.root == NONE || read.cyclic {
            return Ok(Some((read.ty, read.cyclic, true, false, None, None, NONE)));
        }
        let local = self.local(read.root)?;
        if self.body_of.get(local).copied().unwrap_or(NONE) != NONE {
            return Ok(Some((read.ty, false, false, false, None, None, read.row)));
        }
        let reactive = self.reactive.get(local).copied().unwrap_or(false);
        let named = self.ir.node(read.root)?.0 == Op::Name;
        let cell = match self.bound(read.root)? {
            DeclRef::Cell(cell) if named => Some(self.cell_read_fact(cell)?),
            DeclRef::Cell(_)
            | DeclRef::None
            | DeclRef::Instance(_)
            | DeclRef::Duplicate
            | DeclRef::Let(_)
            | DeclRef::Param(_)
            | DeclRef::Fn(_) => None,
        };
        let fixed = self
            .facts
            .get(local)
            .cloned()
            .flatten()
            .and_then(|fact| fact.fixed);
        Ok(Some((read.ty, false, false, reactive, cell, fixed, NONE)))
    }
}
