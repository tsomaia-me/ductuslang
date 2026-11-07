use super::core::{Core, Stage};
use super::decl_ref::DeclRef;
use crate::semantics::Internal;
use crate::semantics::ir::flat::{NONE, Op};

impl Core<'_> {
    pub fn head_path_read(&self, node: u32) -> Result<Option<u32>, Internal> {
        let at = self.paths.get(self.local(node)?).copied().unwrap_or(NONE);
        Ok(self.attr_reads.get(at as usize).map(|read| read.root))
    }

    pub fn head_paths(&mut self, root: u32) -> Stage<bool> {
        let mut fields = Vec::new();
        if !self.head_fields(root, &mut fields)? {
            return Ok(Ok(false));
        }
        let mut reads = Vec::new();
        let mut met = super::column::ColumnSet::new();
        while let Some(node) = fields.pop() {
            if !met.insert(node) {
                return Ok(Ok(false));
            }
            let (_, receiver, field, missing) = self.ir.node(node)?;
            if missing == 1 || self.is_field_receiver(node) || self.ir.node(receiver)?.0 != Op::Name
            {
                return Ok(Ok(false));
            }
            let DeclRef::Instance(item) = self.bound(receiver)? else {
                return Ok(Ok(false));
            };
            if self.pattern_items.contains_key(&item) {
                return Ok(Ok(false));
            }
            let row = self.row_of_item.get(item as usize).copied().unwrap_or(NONE);
            let Some(record) = self.instances.get(row as usize).copied() else {
                return Ok(Ok(false));
            };
            let decl = self
                .vocabulary
                .node_kind(record.kind)
                .ok_or(Internal::missed("node kind"))?;
            let text = self.ir.name_text(field).unwrap_or_default();
            let Some(attr) = decl.attrs.iter().position(|attr| attr.name == text) else {
                return Ok(Ok(false));
            };
            let Some(ty) = decl.attrs.get(attr).and_then(super::check::expected) else {
                return Ok(Ok(false));
            };
            let read = match self.attr_read(row, attr, ty)? {
                Ok(read) => read,
                Err(declined) => return Ok(Err(declined)),
            };
            let written = self
                .attr_reads
                .get(read as usize)
                .map_or(NONE, |found| found.root);
            if written != NONE && !self.head_fields(written, &mut fields)? {
                return Ok(Ok(false));
            }
            reads.push((node, read));
        }
        for (node, read) in reads {
            let local = self.local(node)?;
            if let Some(slot) = self.paths.get_mut(local) {
                *slot = read;
            }
        }
        Ok(Ok(true))
    }

    fn head_fields(&self, root: u32, fields: &mut Vec<u32>) -> Result<bool, Internal> {
        let mut seen = super::column::ColumnSet::new();
        let mut stack = vec![root];
        while let Some(current) = stack.pop() {
            if !seen.insert(current) {
                continue;
            }
            for node in self.deep_postorder(current)? {
                let op = self.ir.node(node)?.0;
                if op == Op::Field && !self.expr_members.contains(&node) {
                    fields.push(node);
                    continue;
                }
                if matches!(op, Op::To | Op::Past | Op::Stepped) {
                    return Ok(false);
                }
                if op == Op::Name
                    && let DeclRef::Let(at) = self.bound(node)?
                {
                    stack.push(self.let_root(at)?);
                }
            }
        }
        Ok(true)
    }
}
