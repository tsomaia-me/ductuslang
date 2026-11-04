use super::annotation::{Refusal, annotated_fact};
use super::core::{Core, Stage};
use super::decl_ref::DeclRef;
use super::declined::Declined;
use super::fact::Fact;
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

    pub fn cell_read_fact(&self, at: u32) -> Result<Fact, Internal> {
        if let Some(ty) = self.cell_annotation_ty(at)? {
            return Ok(Fact::numeric(annotated_fact(ty).leaf, None, Some(ty)));
        }
        let mut fact = self.fact(self.cell_root(at)?)?;
        if !self.derived(at) && fact.fixed.is_none() && fact.read.is_none() {
            fact.read = match fact.leaf {
                Leaf::Int | Leaf::Empty => Some(EvalTy::I64),
                Leaf::Float | Leaf::Bool | Leaf::Placeholder | Leaf::Error => None,
            };
        }
        Ok(fact)
    }

    pub fn derived(&self, at: u32) -> bool {
        self.ir
            .cells
            .get(at as usize)
            .is_some_and(|rec| rec.rule != NONE)
    }

    pub fn node_reactive(&mut self, node: u32) -> Result<bool, Internal> {
        let (op, _, _, _) = self.ir.node(node)?;
        if op == Op::Name {
            return Ok(match self.bound(node)? {
                DeclRef::Cell(_) => true,
                DeclRef::Let(at) => self.let_reads_cell(at)?,
                DeclRef::Param(position) => self.param_fact(node, position)?.1,
                DeclRef::None | DeclRef::Instance(_) | DeclRef::Duplicate | DeclRef::Fn(_) => false,
            });
        }
        if matches!(op, Op::Call | Op::NamedCall) {
            return self.call_reactive(node);
        }
        if op == Op::If {
            let (_, condition, then, otherwise) = self.ir.node(node)?;
            let test = self.fact(condition)?;
            if test.leaf == Leaf::Bool && test.dims.is_empty() && !self.is_reactive(condition)? {
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
        if self.has_fact(root)? {
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
            for node in self.deep_postorder(self.let_root(current)?)? {
                let (op, _, _, _) = self.ir.node(node)?;
                if op != Op::Name {
                    continue;
                }
                match self.bound(node)? {
                    DeclRef::Cell(_) => found = true,
                    DeclRef::Let(other) => stack.push(other),
                    DeclRef::None
                    | DeclRef::Instance(_)
                    | DeclRef::Duplicate
                    | DeclRef::Param(_)
                    | DeclRef::Fn(_) => {}
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
                    DeclRef::None
                    | DeclRef::Instance(_)
                    | DeclRef::Duplicate
                    | DeclRef::Param(_)
                    | DeclRef::Fn(_) => false,
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
            if let Some(ty) = self.cell_annotation_ty(at)? {
                if let Err(declined) = self.check_cell_annotation(at, ty)? {
                    return Ok(Err(declined));
                }
                continue;
            }
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
            if self.faulty_declaration(top)? || !self.let_reads_cell(at)? {
                continue;
            }
            let root = self.let_root(at)?;
            if let Err(declined) = self.ensure_typed(root)? {
                return Ok(Err(declined));
            }
            if !self.is_reactive(root)? {
                continue;
            }
            let expected = match self.annotation_ty(at)? {
                Some(ty)
                    if self.annotation_dims(at)?.is_empty()
                        && self.reads_only_annotated_cells(at)?
                        && self.annotation_refusal(at, ty)? == Refusal::Kept =>
                {
                    Some(ty)
                }
                Some(_) | None => None,
            };
            if let Err(declined) = self.evaluate_own_in(root, expected)? {
                return Ok(Err(declined));
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
    pub fn reads_float_cell(&mut self, root: u32) -> Result<bool, Internal> {
        let outer = self.inst;
        let found = self.reads_float_cell_in(root);
        self.inst = outer;
        found
    }

    fn local_or_param(&self, node: u32) -> Result<bool, Internal> {
        Ok(match self.bound(node)? {
            DeclRef::Param(_) => true,
            DeclRef::Let(at) => self
                .ir
                .lets
                .get(at as usize)
                .is_some_and(|rec| rec.owner != NONE),
            DeclRef::None
            | DeclRef::Cell(_)
            | DeclRef::Instance(_)
            | DeclRef::Duplicate
            | DeclRef::Fn(_) => false,
        })
    }

    fn reads_float_cell_in(&mut self, root: u32) -> Result<bool, Internal> {
        let mut node = root;
        loop {
            let (op, a, b, _) = self.ir.node(node)?;
            if matches!(op, Op::Call | Op::NamedCall)
                && let Some(body) = self.body_of_call(node)
            {
                self.inst = body;
                node = self.body_value(body)?;
                continue;
            }
            if op == Op::Name && self.local_or_param(node)? {
                return self.is_reactive(node);
            }
            if matches!(op, Op::Index | Op::IndexFromEnd) {
                let index = self.fact(b)?;
                if index.leaf != Leaf::Int || !index.dims.is_empty() || self.is_reactive(b)? {
                    return Ok(false);
                }
                node = a;
                continue;
            }
            if op == Op::Slice {
                node = a;
                continue;
            }
            return Ok(op == Op::Name && matches!(self.bound(node)?, DeclRef::Cell(_)));
        }
    }
}

impl Core<'_> {
    fn reads_only_annotated_cells(&self, at: u32) -> Result<bool, Internal> {
        let mut seen = vec![false; self.ir.lets.len()];
        let mut stack = vec![at];
        while let Some(current) = stack.pop() {
            match seen.get_mut(current as usize) {
                Some(slot) if !*slot => *slot = true,
                Some(_) | None => continue,
            }
            for node in self.deep_postorder(self.let_root(current)?)? {
                if self.ir.node(node)?.0 != Op::Name {
                    continue;
                }
                match self.bound(node)? {
                    DeclRef::Cell(cell) if self.cell_annotation_ty(cell)?.is_none() => {
                        return Ok(false);
                    }
                    DeclRef::Let(other) => stack.push(other),
                    DeclRef::Cell(_)
                    | DeclRef::None
                    | DeclRef::Instance(_)
                    | DeclRef::Duplicate
                    | DeclRef::Param(_)
                    | DeclRef::Fn(_) => {}
                }
            }
        }
        Ok(true)
    }
}
