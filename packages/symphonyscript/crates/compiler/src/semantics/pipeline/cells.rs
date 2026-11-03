use super::core::{Core, Stage};
use super::decl_ref::DeclRef;
use super::declined::Declined;
use super::leaf::Leaf;
use super::reason::Reason;
use super::values::flexible;
use crate::diagnostics::Code;
use crate::semantics::Internal;
use crate::semantics::ir::flat::{NONE, Op, TopRec};
use crate::semantics::operators::{EvalTy, Value};

impl Core<'_> {
    pub fn cell_root(&self, at: u32) -> Result<u32, Internal> {
        let rec = self
            .ir
            .cells
            .get(at as usize)
            .ok_or(Internal::missed("IR cell"))?;
        Ok(if rec.rule == NONE { rec.init } else { rec.rule })
    }

    fn derived(&self, at: u32) -> bool {
        self.ir
            .cells
            .get(at as usize)
            .is_some_and(|rec| rec.rule != NONE)
    }

    pub fn is_reactive(&self, node: u32) -> Result<bool, Internal> {
        Ok(self
            .reactive
            .get(self.local(node)?)
            .copied()
            .unwrap_or(false))
    }

    pub fn node_reactive(&mut self, node: u32) -> Result<bool, Internal> {
        let (op, _, _, _) = self.ir.node(node)?;
        if op == Op::Name {
            return Ok(match self.bound(node)? {
                DeclRef::Cell(_) => true,
                DeclRef::Let(at) => self.let_reads_cell(at)?,
                DeclRef::None | DeclRef::Instance(_) | DeclRef::Duplicate => false,
            });
        }
        if op == Op::If {
            let (_, condition, then, otherwise) = self.ir.node(node)?;
            if !self.is_reactive(condition)? {
                let taken = match self.evaluate(condition, EvalTy::Bool)? {
                    Ok(Value::Bool(true)) => Some(then),
                    Ok(Value::Bool(false)) => Some(otherwise),
                    Ok(_) | Err(_) => None,
                };
                if let Some(arm) = taken {
                    return self.is_reactive(arm);
                }
            }
        }
        for operand in self.operands(node)? {
            if self.is_reactive(operand)? {
                return Ok(true);
            }
        }
        Ok(false)
    }

    pub fn let_reads_cell(&mut self, at: u32) -> Result<bool, Internal> {
        let root = self.let_root(at)?;
        if self
            .facts
            .get(self.local(root)?)
            .is_some_and(Option::is_some)
        {
            return self.is_reactive(root);
        }
        if let Some(Some(known)) = self.reads_cell.get(at as usize) {
            return Ok(*known);
        }
        let mut seen = vec![false; self.ir.lets.len()];
        let mut stack = vec![at];
        let mut found = false;
        while let Some(current) = stack.pop() {
            if let Some(Some(known)) = self.reads_cell.get(current as usize) {
                found = found || *known;
                continue;
            }
            match seen.get_mut(current as usize) {
                Some(slot) if !*slot => *slot = true,
                Some(_) | None => continue,
            }
            for node in self.postorder(self.let_root(current)?)? {
                let (op, _, _, _) = self.ir.node(node)?;
                if op != Op::Name {
                    continue;
                }
                match self.bound(node)? {
                    DeclRef::Cell(_) => found = true,
                    DeclRef::Let(other) => stack.push(other),
                    DeclRef::None | DeclRef::Instance(_) | DeclRef::Duplicate => {}
                }
            }
        }
        if let Some(slot) = self.reads_cell.get_mut(at as usize) {
            *slot = Some(found);
        }
        Ok(found)
    }

    pub fn type_cells(&mut self) -> Stage<()> {
        let count = u32::try_from(self.ir.cells.len()).unwrap_or(0);
        let mut order: Vec<u32> = (0..count).filter(|at| !self.derived(*at)).collect();
        order.extend((0..count).filter(|at| self.derived(*at)));
        for at in order {
            let root = self.cell_root(at)?;
            for node in self.postorder(root)? {
                let (op, _, _, _) = self.ir.node(node)?;
                if op != Op::Name {
                    continue;
                }
                let grouped = match self.bound(node)? {
                    DeclRef::Cell(other) => !self.derived(at) || self.derived(other),
                    DeclRef::Let(other) => self.let_reads_cell(other)?,
                    DeclRef::None | DeclRef::Instance(_) | DeclRef::Duplicate => false,
                };
                if grouped {
                    return Ok(Err(Declined(Reason::CellGroup)));
                }
            }
            if let Err(declined) = self.ensure_typed(root)? {
                return Ok(Err(declined));
            }
        }
        Ok(Ok(()))
    }

    pub fn check_cells(&mut self) -> Stage<()> {
        for at in 0..u32::try_from(self.ir.cells.len()).unwrap_or(0) {
            let root = self.cell_root(at)?;
            let checked = match self.own_ty(root)? {
                Some(own) if self.derived(at) && !self.is_kept(root)? => {
                    self.evaluate(root, flexible(own))?.map(|_| ())
                }
                Some(_) | None => self.evaluate_own(root)?,
            };
            if let Err(declined) = checked {
                return Ok(Err(declined));
            }
        }
        Ok(Ok(()))
    }

    pub fn check_reactive_lets(&mut self) -> Stage<()> {
        for top in self.ir.top.clone() {
            let TopRec::Let(at) = top else {
                continue;
            };
            if !self.let_reads_cell(at)? {
                continue;
            }
            let root = self.let_root(at)?;
            if let Err(declined) = self.ensure_typed(root)? {
                return Ok(Err(declined));
            }
            if !self.is_reactive(root)? {
                continue;
            }
            if let Err(declined) = self.evaluate_own(root)? {
                return Ok(Err(declined));
            }
            if matches!(self.fact(root)?.leaf, Leaf::Placeholder | Leaf::Error) {
                continue;
            }
            let rec = self
                .ir
                .lets
                .get(at as usize)
                .ok_or(Internal::missed("IR let"))?;
            let name = self.ir.binder(rec.binder)?.name;
            let span =
                self.name_span(self.let_file.get(at as usize).copied().unwrap_or(0), name)?;
            let text = self.name_text(name)?;
            self.draw(
                Code::ReactiveLet,
                span,
                format!("`{text}` is a `let`, a compile-time value, and its value reads a cell"),
            );
        }
        Ok(Ok(()))
    }

    pub fn check_integer_cells(&mut self) -> Result<(), Internal> {
        for at in 0..u32::try_from(self.ir.cells.len()).unwrap_or(0) {
            if self.derived(at) || self.cell_uses.get(at as usize) != Some(&[false, true]) {
                continue;
            }
            let fact = self.fact(self.cell_root(at)?)?;
            if !matches!(fact.leaf, Leaf::Int | Leaf::Empty)
                || !fact.dims.is_empty()
                || fact.fixed.is_some()
            {
                continue;
            }
            let rec = self
                .ir
                .cells
                .get(at as usize)
                .ok_or(Internal::missed("IR cell"))?;
            let name = self.ir.binder(rec.binder)?.name;
            let file = self.file_of(rec.init);
            let span = self.name_span(file, name)?;
            let text = self.name_text(name)?;
            self.draw(
                Code::IntegerCell,
                span,
                format!(
                    "`{text}` is an integer host-fed cell whose every use expects a float, so its host cannot write a fraction; a float init such as `1.0` makes it a float cell"
                ),
            );
        }
        Ok(())
    }

    fn name_text(&self, name: u32) -> Result<String, Internal> {
        let entry = self.ir.name(name).ok_or(Internal::missed("IR name"))?;
        Ok(self.ir.symbols.text(entry.sym).to_string())
    }
}

impl Core<'_> {
    pub fn reads_float_cell(&self, root: u32) -> Result<bool, Internal> {
        let mut node = root;
        loop {
            let (op, a, _, _) = self.ir.node(node)?;
            if matches!(op, Op::Index | Op::IndexFromEnd | Op::Slice) {
                node = a;
                continue;
            }
            return Ok(op == Op::Name && matches!(self.bound(node)?, DeclRef::Cell(_)));
        }
    }
}
