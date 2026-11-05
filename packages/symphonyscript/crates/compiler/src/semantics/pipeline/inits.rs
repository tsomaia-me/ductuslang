use super::core::{Core, Stage};
use super::fact::Fact;
use super::leaf::Leaf;
use crate::diagnostics::Code;
use crate::semantics::Internal;
use crate::semantics::ir::flat::NONE;
use crate::semantics::operators::EvalTy;

const REACTIVE_INIT: &str = "a cell's init is a compile-time value and reads no cell";

const INIT_SHAPE: &str = "a cell's init has the cell's type, and this one's shape differs";

impl Core<'_> {
    pub fn reactive_host_init(&mut self, at: u32) -> Result<bool, Internal> {
        let Some(rec) = self.ir.cells.get(at as usize).copied() else {
            return Err(Internal::missed("IR cell"));
        };
        if rec.rule != NONE || rec.init == NONE || !self.is_reactive(rec.init)? {
            return Ok(false);
        }
        if matches!(self.fact(rec.init)?.leaf, Leaf::Placeholder | Leaf::Error) {
            return Ok(true);
        }
        self.draw_at(Code::ReactiveInitializer, rec.init, REACTIVE_INIT)?;
        Ok(true)
    }

    pub fn check_derived_init(&mut self, at: u32) -> Stage<()> {
        let Some(rec) = self.ir.cells.get(at as usize).copied() else {
            return Err(Internal::missed("IR cell"));
        };
        if rec.rule == NONE || rec.init == NONE {
            return Ok(Ok(()));
        }
        let init = rec.init;
        let held = self.fact(init)?;
        if matches!(held.leaf, Leaf::Placeholder | Leaf::Error) && !held.node {
            return self.own_check(init);
        }
        if self.is_reactive(init)? {
            self.draw_at(Code::ReactiveInitializer, init, REACTIVE_INIT)?;
            return Ok(Ok(()));
        }
        if held.node {
            self.draw_at(
                Code::StateValue,
                init,
                "a cell never holds a node reference",
            )?;
            return Ok(Ok(()));
        }
        let cell = self.cell_read_fact(at)?;
        if matches!(cell.leaf, Leaf::Placeholder | Leaf::Error) {
            return self.own_check(init);
        }
        if !super::folds::dims_agree(&held.dims, &cell.dims) {
            self.draw_at(Code::TypeMismatch, init, INIT_SHAPE)?;
            return self.own_check(init);
        }
        if !cell.dims.is_empty() {
            return self.own_check(init);
        }
        let ty = match self.cell_annotation_ty(at)? {
            Some(ty) => ty,
            None => self.read_ty(at, &cell),
        };
        self.bind_value(init, "", ty)
    }

    pub fn read_ty(&self, at: u32, cell: &Fact) -> EvalTy {
        let reads = self
            .rule_reads
            .get(at as usize)
            .copied()
            .unwrap_or_default();
        let widest = |classes: &[usize]| {
            classes
                .iter()
                .rev()
                .find_map(|class| reads.get(*class).copied().flatten())
                .map(|(ty, _)| ty)
        };
        let float = widest(&[1, 2]);
        let integer = widest(&[4, 5, 6]);
        match cell.leaf {
            Leaf::Bool => EvalTy::Bool,
            Leaf::Enum(Some(id)) => EvalTy::Enum(id),
            Leaf::Float => float.or(cell.own()).unwrap_or(EvalTy::F64),
            Leaf::Int | Leaf::Empty | Leaf::Placeholder | Leaf::Error | Leaf::Enum(None) => {
                integer.or(float).or(cell.own()).unwrap_or(EvalTy::I64)
            }
        }
    }
}
