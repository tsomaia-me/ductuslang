use super::core::{Core, Stage};
use super::decl_ref::DeclRef;
use super::fact::Fact;
use super::folds::ANY_LENGTH;
use super::leaf::Leaf;
use crate::diagnostics::{Code, Diagnostic};
use crate::semantics::Internal;
use crate::semantics::ir::flat::{FlatIr, NONE, Op};
use crate::semantics::operators::{EvalTy, Value};

const OFFSET: &str = "the offset of a history read is a positive integer known at compile time";

const OFFSET_RANGE: &str = "the offset of a history read is between 1 and 2³¹ − 1";

const FALLBACK_SHAPE: &str =
    "a history read's fallback has the cell's type, and this one's shape differs";

const LENGTH_LIMIT: i128 = 1 << 31;

pub(crate) fn history_targets(ir: &FlatIr) -> Vec<u8> {
    let mut marked = vec![0; ir.op.len()];
    for (local, op) in ir.op.iter().enumerate() {
        if !matches!(op, Op::Past | Op::Stepped) {
            continue;
        }
        let Some(target) = ir.a.get(local).and_then(|node| node.checked_sub(ir.base)) else {
            continue;
        };
        if let Some(slot) = marked.get_mut(target as usize) {
            *slot = if *op == Op::Past { 1 } else { 2 };
        }
    }
    marked
}

enum Target {
    Cell(u32),
    Let,
    Faulty,
}

impl Core<'_> {
    pub fn is_history_target(&self, node: u32) -> bool {
        self.local(node)
            .ok()
            .and_then(|local| self.history_target.get(local).copied())
            .is_some_and(|kind| kind != 0)
    }

    pub fn history_word(&self, node: u32) -> &'static str {
        let kind = self
            .local(node)
            .ok()
            .and_then(|local| self.history_target.get(local).copied());
        if kind == Some(2) { "step" } else { "history" }
    }

    pub fn history_target_fact(&self, node: u32) -> Result<Fact, Internal> {
        Ok(match self.bound(node)? {
            DeclRef::Cell(at) if self.cell_annotation_ty(at)?.is_some() => {
                self.cell_read_fact(at)?
            }
            DeclRef::Cell(at) if self.has_fact(self.cell_root(at)?)? => self.cell_read_fact(at)?,
            DeclRef::Cell(_)
            | DeclRef::None
            | DeclRef::Duplicate
            | DeclRef::Let(_)
            | DeclRef::Param(_)
            | DeclRef::Instance(_)
            | DeclRef::Fn(_)
            | DeclRef::Pattern(_) => Fact::PLACEHOLDER,
        })
    }

    pub fn history_fact(
        &mut self,
        node: u32,
        target: u32,
        offset: u32,
        fallback: u32,
    ) -> Stage<Fact> {
        let cell = match self.history_cell(target, "history")? {
            Ok(Target::Faulty) => return Ok(Ok(Fact::PLACEHOLDER)),
            Ok(cell) => cell,
            Err(declined) => return Ok(Err(declined)),
        };
        match self.offset_holds(offset)? {
            Ok(true) => {}
            Ok(false) => return Ok(Ok(Fact::PLACEHOLDER)),
            Err(declined) => return Ok(Err(declined)),
        }
        let at = match cell {
            Target::Cell(at) => at,
            Target::Let | Target::Faulty => {
                self.compile_time_target(node, "history")?;
                return Ok(Ok(Fact::PLACEHOLDER));
            }
        };
        let fact = match self.group_read(node)? {
            Some(fact) => fact,
            None => self.cell_read_fact(at)?,
        };
        if fallback == NONE {
            return Ok(Ok(fact));
        }
        let held = self.fact(fallback)?;
        if matches!(held.leaf, Leaf::Placeholder | Leaf::Error) {
            return Ok(Ok(Fact::PLACEHOLDER));
        }
        if matches!(fact.leaf, Leaf::Placeholder | Leaf::Error) {
            return Ok(Ok(fact));
        }
        if let Some(settled) = self.group_settled(node, at)
            && !fallback_fits(&held, &settled)
        {
            self.draw_at(Code::TypeMismatch, fallback, FALLBACK_SHAPE)?;
            if let Err(declined) = self.own_check(fallback)? {
                return Ok(Err(declined));
            }
            return Ok(Ok(settled));
        }
        if !fallback_fits(&held, &fact) {
            self.draw_at(Code::TypeMismatch, fallback, FALLBACK_SHAPE)?;
            if let Err(declined) = self.own_check(fallback)? {
                return Ok(Err(declined));
            }
            return Ok(Ok(fact));
        }
        if !fact.dims.is_empty() {
            if let Err(declined) = self.own_check(fallback)? {
                return Ok(Err(declined));
            }
            return Ok(Ok(fact));
        }
        let ty = match self.cell_annotation_ty(at)? {
            Some(ty) => ty,
            None => match fact.leaf {
                Leaf::Bool => EvalTy::Bool,
                Leaf::Float => fact.own().unwrap_or(EvalTy::F64),
                Leaf::Int | Leaf::Empty | Leaf::Placeholder | Leaf::Error => {
                    fact.own().unwrap_or(EvalTy::I64)
                }
            },
        };
        if let Err(declined) = self.bind_value(fallback, "", ty)? {
            return Ok(Err(declined));
        }
        Ok(Ok(fact))
    }

    pub fn stepped_fact(&mut self, node: u32, target: u32) -> Stage<Fact> {
        match self.history_cell(target, "step")? {
            Ok(Target::Cell(_)) => Ok(Ok(Fact::scalar(Leaf::Bool))),
            Ok(Target::Let) => {
                self.compile_time_target(node, "step")?;
                Ok(Ok(Fact::PLACEHOLDER))
            }
            Ok(Target::Faulty) => Ok(Ok(Fact::PLACEHOLDER)),
            Err(declined) => Ok(Err(declined)),
        }
    }

    fn compile_time_target(&mut self, node: u32, what: &str) -> Result<(), Internal> {
        self.draw_at(
            Code::HistoryTarget,
            node,
            format!(
                "this `let` is a compile-time value, which has no {what}; a history read names a `state`, a `stream` or a `channel`"
            ),
        )
    }

    fn history_cell(&mut self, target: u32, what: &str) -> Stage<Target> {
        let name = self.ir.node(target)?.1;
        let entry = self.ir.name(name).ok_or(Internal::missed("IR name"))?;
        let text = self.ir.symbols.text(entry.sym).to_string();
        let span = self.node_span(target)?;
        match self.bound(target)? {
            DeclRef::Cell(at) => return Ok(Ok(Target::Cell(at))),
            DeclRef::None | DeclRef::Duplicate => return Ok(Ok(Target::Faulty)),
            DeclRef::Let(at)
                if self
                    .ir
                    .lets
                    .get(at as usize)
                    .is_some_and(|rec| rec.owner == NONE) =>
            {
                if self.cyclic.get(at as usize).copied().unwrap_or(false) {
                    return Ok(Ok(Target::Faulty));
                }
                return Ok(Ok(Target::Let));
            }
            DeclRef::Param(_) if !self.placement_loop_name(target)? => {
                let diagnostic = Diagnostic::new(
                    Code::HistoryTarget,
                    span,
                    format!(
                        "`{text}` is a parameter without a marker, so it has no {what}; its argument is bound as a value"
                    ),
                )
                .with_help(format!(
                    "mark the parameter `cell` in its parameter list, as in `(cell {text})`"
                ));
                let diagnostic = self.chained(diagnostic);
                self.push_drawn(diagnostic);
            }
            DeclRef::Let(_)
            | DeclRef::Param(_)
            | DeclRef::Instance(_)
            | DeclRef::Fn(_)
            | DeclRef::Pattern(_) => self.not_a_cell(span, &text, what),
        }
        Ok(Ok(Target::Faulty))
    }

    fn offset_holds(&mut self, offset: u32) -> Stage<bool> {
        let fact = self.fact(offset)?;
        if matches!(fact.leaf, Leaf::Placeholder | Leaf::Error) {
            return Ok(Ok(false));
        }
        if fact.leaf != Leaf::Int || !fact.dims.is_empty() || self.is_reactive(offset)? {
            self.draw_at(Code::HistoryOffset, offset, OFFSET)?;
            return Ok(Ok(false));
        }
        let value = match self.evaluate(offset, EvalTy::Exact)? {
            Ok(value) => value,
            Err(declined) => return Ok(Err(declined)),
        };
        let Value::Int(value) = value else {
            return Ok(Ok(false));
        };
        if value < 1 {
            self.draw_at(Code::HistoryOffset, offset, OFFSET)?;
            return Ok(Ok(false));
        }
        if value >= LENGTH_LIMIT {
            self.draw_at(Code::HistoryOffset, offset, OFFSET_RANGE)?;
            return Ok(Ok(false));
        }
        Ok(Ok(true))
    }
}

impl Core<'_> {
    pub fn not_a_cell(&mut self, span: symphonyscript_core::Span, text: &str, what: &str) {
        self.draw(
            Code::HistoryTarget,
            span,
            format!(
                "`{text}` is not a cell, so it has no {what}; a history read names a `state`, a `stream` or a `channel`"
            ),
        );
    }

    fn placement_loop_name(&self, node: u32) -> Result<bool, Internal> {
        let owner = self.binding_owner(node)?;
        let loops = self.functions() + u32::try_from(self.ir.fors.len()).unwrap_or(0);
        Ok(owner >= loops && owner < self.patterns_start())
    }
}

fn fallback_fits(held: &Fact, cell: &Fact) -> bool {
    if held.leaf == Leaf::Empty && held.dims == [0] && !cell.dims.is_empty() {
        return true;
    }
    held.dims.len() == cell.dims.len()
        && held
            .dims
            .iter()
            .zip(&cell.dims)
            .all(|(held, cell)| held == cell || *cell == ANY_LENGTH)
}
