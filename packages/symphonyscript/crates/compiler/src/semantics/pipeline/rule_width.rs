use super::core::{Core, Stage};
use super::decl_ref::DeclRef;
use super::leaf::Leaf;
use super::width::widens;
use crate::diagnostics::Code;
use crate::semantics::Internal;
use crate::semantics::ir::flat::Op;
use crate::semantics::operators::{EvalTy, ty_name};

fn widened_through(op: Op) -> bool {
    matches!(
        op,
        Op::Neg
            | Op::Add
            | Op::Sub
            | Op::Mul
            | Op::Div
            | Op::IntDiv
            | Op::Rem
            | Op::Pow
            | Op::Array
    )
}

impl Core<'_> {
    pub fn refuse_wider_rule(&mut self, root: u32, width: EvalTy) -> Stage<()> {
        let mut refused = Vec::new();
        let mut stack = vec![root];
        while let Some(node) = stack.pop() {
            let Some(own) = self.fact(node)?.fixed.filter(|own| !widens(*own, width)) else {
                continue;
            };
            if self.adaptive_operand(node)? {
                stack.extend(self.operands(node)?.into_iter().rev());
                continue;
            }
            self.draw_at_root(
                Code::TypeMismatch,
                node,
                format!(
                    "a `{}` value does not widen into `{}`",
                    ty_name(own),
                    ty_name(width)
                ),
            )?;
            refused.push(node);
        }
        if refused.contains(&root) {
            return self.evaluate_own(root);
        }
        for node in refused {
            if let Err(declined) = self.evaluate_own(node)? {
                return Ok(Err(declined));
            }
        }
        Ok(self.evaluate(root, width)?.map(|_| ()))
    }

    pub fn adaptive_operand(&mut self, node: u32) -> Result<bool, Internal> {
        let (op, _, _, _) = self.ir.node(node)?;
        if !widened_through(op) {
            return Ok(false);
        }
        let mut stack = self.operands(node)?;
        while let Some(node) = stack.pop() {
            if !self.is_reactive(node)? {
                continue;
            }
            if self.fact(node)?.fixed.is_none() {
                return Ok(true);
            }
            let (op, _, _, _) = self.ir.node(node)?;
            if widened_through(op) {
                stack.extend(self.operands(node)?);
            }
        }
        Ok(false)
    }
}

impl Core<'_> {
    pub fn reactive_reads(&self, root: u32) -> Result<(bool, bool), Internal> {
        let (mut host, mut derived) = (false, false);
        for node in self.postorder(root)? {
            let (op, _, _, _) = self.ir.node(node)?;
            if !matches!(op, Op::Name | Op::Field) || !self.is_reactive(node)? {
                continue;
            }
            if op == Op::Field {
                host = true;
                continue;
            }
            match self.bound(node)? {
                DeclRef::Cell(at) if self.derived(at) => derived = true,
                DeclRef::Param(_) => {}
                DeclRef::Cell(_)
                | DeclRef::None
                | DeclRef::Let(_)
                | DeclRef::Instance(_)
                | DeclRef::Duplicate
                | DeclRef::Fn(_)
                | DeclRef::Pattern(_) => host = true,
            }
        }
        Ok((host, derived))
    }
}

impl Core<'_> {
    pub fn defer_compared_fit(&mut self, a: u32, b: u32, pair: EvalTy) -> Result<(), Internal> {
        if pair != EvalTy::F64 {
            return Ok(());
        }
        for (name, other) in [(a, b), (b, a)] {
            if self.ir.node(name)?.0 != Op::Name {
                continue;
            }
            let DeclRef::Cell(at) = self.bound(name)? else {
                continue;
            };
            if !self.derived(at) || self.cell_annotation_ty(at)?.is_some() {
                continue;
            }
            let (cell, literal) = (self.fact(name)?, self.fact(other)?);
            let flexible = literal.leaf == Leaf::Float
                && literal.own().is_none()
                && literal.dims.is_empty()
                && !self.is_reactive(other)?;
            if cell.leaf != Leaf::Float
                || cell.own().is_some()
                || !cell.dims.is_empty()
                || !flexible
                || self.own_float(other)? != EvalTy::F64
            {
                continue;
            }
            self.compared_reads.insert(name);
            if !self.compared_fits.contains(&(at, other)) {
                self.compared_fits.push((at, other));
            }
        }
        Ok(())
    }

    pub fn check_compared_fits(&mut self, at: u32) -> Result<(), Internal> {
        let wide = self
            .rule_reads
            .get(at as usize)
            .and_then(|reads| reads.get(2).copied().flatten())
            .is_some();
        if wide {
            return Ok(());
        }
        let literals: Vec<u32> = self
            .compared_fits
            .iter()
            .filter(|(cell, _)| *cell == at)
            .map(|(_, literal)| *literal)
            .collect();
        for literal in literals {
            self.draw_at_root(
                Code::DoesNotFit,
                literal,
                format!("this value does not fit `{}`", ty_name(EvalTy::F32)),
            )?;
        }
        Ok(())
    }
}

impl Core<'_> {
    pub fn placed_root(&self, root: u32) -> Result<bool, Internal> {
        if self
            .ir
            .cells
            .iter()
            .any(|rec| rec.init == root || rec.rule == root)
        {
            return Ok(false);
        }
        for at in 0..u32::try_from(self.ir.lets.len()).unwrap_or(0) {
            if self.let_root(at)? == root {
                return Ok(false);
            }
        }
        Ok(true)
    }
}
