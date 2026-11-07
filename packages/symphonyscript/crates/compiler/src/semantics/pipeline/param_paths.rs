use super::core::{Core, Stage};
use super::decl_ref::DeclRef;
use super::declined::Declined;
use super::fact::Fact;
use super::paths::End;
use super::reason::Reason;
use crate::diagnostics::Code;
use crate::semantics::Internal;
use crate::semantics::counted_walk::CountedWalk;
use crate::semantics::ir::flat::{NONE, Op};
use crate::semantics::operators::Value;

const MEMBERS: [&str; 3] = ["size", "capacity", "bytes"];

impl Core<'_> {
    pub fn is_member_path(&self, node: u32) -> Result<bool, Internal> {
        let (_, _, field, _) = self.ir.node(node)?;
        Ok(MEMBERS.contains(&self.ir.name_text(field).unwrap_or_default()))
    }

    pub fn param_path_reads(&mut self) -> Result<(), Internal> {
        if self.param_paths.is_empty() {
            return Ok(());
        }
        let mut nodes: Vec<u32> = self.param_paths.iter().collect();
        nodes.sort_unstable();
        let mut calls: Vec<Vec<u32>> = vec![Vec::new(); self.functions() as usize];
        for local in 0..self.ir.op.len() {
            if !matches!(self.ir.op.get(local), Some(Op::Call | Op::NamedCall)) {
                continue;
            }
            let node = self.ir.base + u32::try_from(local).unwrap_or(0);
            if let Some(callee) = self.callee(node)?
                && let Some(list) = calls.get_mut(callee as usize)
            {
                list.push(node);
            }
        }
        for node in nodes {
            let member = self.is_member_path(node)?;
            let (_, receiver, field, _) = self.ir.node(node)?;
            let DeclRef::Param(position) = self.bound(receiver)? else {
                continue;
            };
            let function = self.binding_owner(receiver)?;
            let mut work = vec![(function, position)];
            let mut seen = std::collections::HashSet::new();
            while let Some((callee, at)) = work.pop() {
                if !seen.insert((callee, at)) {
                    continue;
                }
                for call in calls.get(callee as usize).cloned().unwrap_or_default() {
                    CountedWalk::CorePathStep.step();
                    if self.ir.node(call)?.0 == Op::NamedCall {
                        self.unwritten_params.insert((function, position));
                        continue;
                    }
                    let Some(mut argument) = self.arguments(call)?.get(at as usize).copied() else {
                        continue;
                    };
                    let mut lets = 0;
                    while self.ir.node(argument)?.0 == Op::Name
                        && let DeclRef::Let(held) = self.bound(argument)?
                        && !self.cyclic.get(held as usize).copied().unwrap_or(true)
                        && lets < self.ir.lets.len()
                    {
                        argument = self.let_root(held)?;
                        lets += 1;
                    }
                    if self.ir.node(argument)?.0 != Op::Name {
                        self.unwritten_params.insert((function, position));
                        continue;
                    }
                    match self.bound(argument)? {
                        DeclRef::Instance(item) => {
                            let row = self.row_of_item.get(item as usize).copied().unwrap_or(NONE);
                            if member {
                                self.member_keys.insert((function, position));
                            } else if row != NONE
                                && let Some(read) = self.quiet_attr_read(row, field)?
                            {
                                self.param_reads.push((function, read));
                            }
                        }
                        DeclRef::Param(outer) => {
                            let owner = self.binding_owner(argument)?;
                            if owner < self.functions() {
                                work.push((owner, outer));
                            } else {
                                self.unwritten_params.insert((function, position));
                            }
                        }
                        DeclRef::None
                        | DeclRef::Cell(_)
                        | DeclRef::Let(_)
                        | DeclRef::Duplicate
                        | DeclRef::Fn(_)
                        | DeclRef::Pattern(_) => {
                            self.unwritten_params.insert((function, position));
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn quiet_attr_read(&mut self, row: u32, field: u32) -> Result<Option<u32>, Internal> {
        let entry = *self.ir.name(field).ok_or(Internal::missed("IR name"))?;
        let kind = self
            .instances
            .get(row as usize)
            .ok_or(Internal::missed("row"))?
            .kind;
        let decl = self
            .vocabulary
            .node_kind(kind)
            .ok_or(Internal::missed("node kind"))?;
        let text = self.ir.symbols.text(entry.sym);
        let Some(attr) = decl.attrs.iter().position(|attr| attr.name == text) else {
            return Ok(None);
        };
        let declared = decl.attrs.get(attr).ok_or(Internal::missed("attribute"))?;
        if declared.unit.is_some() {
            return Ok(None);
        }
        let Some(ty) = super::check::expected(declared) else {
            return Ok(None);
        };
        Ok(self.attr_read(row, attr, ty)?.ok())
    }

    pub fn param_path_fact(&mut self, node: u32) -> Stage<Option<Fact>> {
        let (_, receiver, field, _) = self.ir.node(node)?;
        let fact = match self.ensure_typed(receiver)? {
            Ok(fact) => fact,
            Err(declined) => return Ok(Err(declined)),
        };
        if !fact.node
            && matches!(
                fact.leaf,
                super::leaf::Leaf::Placeholder | super::leaf::Leaf::Error
            )
        {
            return Ok(Ok(Some(Fact::PLACEHOLDER)));
        }
        if !fact.node {
            if self.is_member_path(node)? && !fact.dims.is_empty() {
                self.expr_members.insert(node);
                return Ok(Ok(None));
            }
            let text = self.ir.name_text(field).unwrap_or_default().to_string();
            self.draw_at(
                Code::UnknownAttribute,
                node,
                format!(
                    "`.{text}` reads an attribute of an instance or a wire, and this is a value"
                ),
            )?;
            return Ok(Ok(Some(Fact::PLACEHOLDER)));
        }
        if !fact.dims.is_empty() {
            return Ok(Err(Declined(Reason::Member)));
        }
        let Some(inst) = self.path_inst(node) else {
            return Ok(Ok(Some(Fact::PLACEHOLDER)));
        };
        if self.inst_reads.contains_key(&(node, inst)) {
            return Ok(Ok(None));
        }
        let DeclRef::Param(position) = self.bound(receiver)? else {
            return Err(Internal::missed("a parameter path's receiver"));
        };
        if self
            .unwritten_params
            .contains(&(self.binding_owner(receiver)?, position))
        {
            return Ok(Err(Declined(Reason::Member)));
        }
        let value = self
            .bodies
            .get(inst as usize)
            .and_then(|body| body.values.get(position as usize))
            .cloned();
        let Some(Value::Node(row)) = value else {
            return Ok(Err(Declined(Reason::Member)));
        };
        match self.step_row(node, row, field)? {
            Ok(End::Attr(at)) => {
                self.inst_reads.insert((node, inst), at);
                let read = self
                    .attr_reads
                    .get(at as usize)
                    .cloned()
                    .ok_or(Internal::missed("attribute read"))?;
                let env = self.read_env(&read)?;
                if self
                    .open_reads
                    .iter()
                    .any(|(root, open, _)| root == read.root && open == env)
                {
                    return Ok(Err(Declined(Reason::Member)));
                }
                Ok(Ok(None))
            }
            Ok(End::Value | End::Placeholder) => Ok(Ok(Some(Fact::PLACEHOLDER))),
            Ok(End::Row(_) | End::Use(_) | End::To | End::Wire(_)) => {
                Ok(Err(Declined(Reason::Member)))
            }
            Err(declined) => Ok(Err(declined)),
        }
    }
}
