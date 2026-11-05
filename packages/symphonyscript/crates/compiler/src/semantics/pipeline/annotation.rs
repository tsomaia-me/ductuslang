use super::core::{Core, Stage};
use super::core_value::CoreValue;
use super::declined::Declined;
use super::fact::Fact;
use super::folds::ANY_LENGTH;
use super::leaf::Leaf;
use super::reason::Reason;
use super::values::evaluated_in;
use super::width::{is_fixed_float, target_ty, widens};
use crate::diagnostics::Code;
use crate::semantics::Internal;
use crate::semantics::ir::flat::{NONE, Op};
use crate::semantics::operators::{EvalTy, Value, int_range, ty_name};
use symphonyscript_core::Span;

pub(crate) const SHAPE: &str = "the value's shape differs from its annotated type";
pub(crate) const TYPE: &str =
    "the value does not have the annotated type; a conversion states the rounding";

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Refusal {
    Kept,
    Silent,
    Shape,
    Type,
}

pub(crate) fn annotated_fact(ty: EvalTy) -> Fact {
    let leaf = if is_fixed_float(ty) {
        Leaf::Float
    } else if ty == EvalTy::Bool {
        Leaf::Bool
    } else {
        Leaf::Int
    };
    Fact::fixed(leaf, Some(ty))
}

impl Core<'_> {
    pub fn annotation_ty(&self, at: u32) -> Result<Option<EvalTy>, Internal> {
        let rec = self
            .ir
            .lets
            .get(at as usize)
            .ok_or(Internal::missed("IR let"))?;
        self.binder_ty(rec.binder)
    }

    pub fn cell_annotation_ty(&self, at: u32) -> Result<Option<EvalTy>, Internal> {
        let rec = self
            .ir
            .cells
            .get(at as usize)
            .ok_or(Internal::missed("IR cell"))?;
        self.binder_ty(rec.binder)
    }

    pub fn check_cell_annotation(&mut self, at: u32, ty: EvalTy) -> Stage<()> {
        let root = self.cell_root(at)?;
        let fact = match self.ensure_typed(root)? {
            Ok(fact) => fact,
            Err(declined) => return Ok(Err(declined)),
        };
        if let Some(capacity) = self.cell_capacity_annotation(at)? {
            return self.check_variable_init(root, &fact, (ty, capacity));
        }
        if fact.dims.is_empty() || matches!(fact.leaf, Leaf::Placeholder | Leaf::Error) {
            let name = self.ir.binder(self.cell_binder(at)?)?.name;
            let entry = self.ir.name(name).ok_or(Internal::missed("IR name"))?;
            let text = self.ir.symbols.text(entry.sym).to_string();
            return self.bind_value(root, &text, ty);
        }
        let summary = if self.derived(at) {
            "the rule's shape differs from the cell's type"
        } else {
            "the init's shape differs from the cell's type"
        };
        self.draw_at(Code::TypeMismatch, root, summary)?;
        self.own_check(root)
    }

    fn check_variable_init(
        &mut self,
        root: u32,
        fact: &Fact,
        (ty, capacity): (EvalTy, u32),
    ) -> Stage<()> {
        if matches!(fact.leaf, Leaf::Placeholder | Leaf::Error) {
            return self.own_check(root);
        }
        let Some((length, inner)) = fact.dims.split_first() else {
            self.draw_at(
                Code::TypeMismatch,
                root,
                "a variable-array cell's init is an array",
            )?;
            return self.own_check(root);
        };
        if !inner.is_empty() && *length > 0 {
            self.draw_at(
                Code::TypeMismatch,
                root,
                "the init's rows differ from the cell's type",
            )?;
            return self.own_check(root);
        }
        if *length > capacity {
            let help = if capacity == 1 {
                String::from("slice the value to its newest element with `[^1..]`")
            } else {
                format!("slice the value to its newest {capacity} elements with `[^{capacity}..]`")
            };
            let summary = format!(
                "the initializer holds {length} elements, more than the capacity {capacity}"
            );
            self.draw_helped_at(Code::CapacityExceeded, root, summary, help)?;
            return self.own_check(root);
        }
        let element = Fact {
            dims: Vec::new(),
            ..fact.clone()
        };
        match refusal(&element, ty) {
            Refusal::Type => {
                self.draw_refused_leaves(root, ty)?;
                self.own_check(root)
            }
            Refusal::Silent | Refusal::Shape => self.own_check(root),
            Refusal::Kept => self.evaluate(root, ty).map(|found| found.map(|_| ())),
        }
    }

    fn cell_binder(&self, at: u32) -> Result<u32, Internal> {
        self.ir
            .cells
            .get(at as usize)
            .map(|rec| rec.binder)
            .ok_or(Internal::missed("IR cell"))
    }

    pub fn binder_ty(&self, binder: u32) -> Result<Option<EvalTy>, Internal> {
        let node = self.ir.binder(binder)?.annotation;
        if node == NONE {
            return Ok(None);
        }
        let (mut op, mut code, _, _) = self.ir.node(node)?;
        while matches!(op, Op::TyArray | Op::TyVar) {
            (op, code, _, _) = self.ir.node(code)?;
        }
        if op != Op::TyScalar {
            return Err(Internal::missed(
                "an annotation the coverage table leaves out",
            ));
        }
        target_ty(code)
            .map(Some)
            .ok_or(Internal::missed("an annotated type"))
    }

    pub fn binder_dims(&self, binder: u32) -> Result<Vec<u32>, Internal> {
        let mut dims = Vec::new();
        let mut node = self.ir.binder(binder)?.annotation;
        while node != NONE {
            let (op, inner, length, _) = self.ir.node(node)?;
            match op {
                Op::TyArray => dims.push(length),
                Op::TyVar => dims.push(ANY_LENGTH),
                Op::Int
                | Op::Float
                | Op::Bool
                | Op::Str
                | Op::Variant
                | Op::Name
                | Op::To
                | Op::Field
                | Op::Call
                | Op::NamedCall
                | Op::MethodCall
                | Op::NamedMethodCall
                | Op::Past
                | Op::Stepped
                | Op::If
                | Op::Match
                | Op::For
                | Op::Array
                | Op::Index
                | Op::IndexFromEnd
                | Op::Slice
                | Op::Neg
                | Op::Not
                | Op::Add
                | Op::Sub
                | Op::Mul
                | Op::Div
                | Op::IntDiv
                | Op::Rem
                | Op::Pow
                | Op::And
                | Op::Or
                | Op::Lt
                | Op::Le
                | Op::Gt
                | Op::Ge
                | Op::Is
                | Op::IsNot
                | Op::Convert
                | Op::TyScalar
                | Op::TyName
                | Op::Bad => break,
            }
            node = inner;
        }
        Ok(dims)
    }

    pub fn cell_capacity_annotation(&self, at: u32) -> Result<Option<u32>, Internal> {
        let binder = self.cell_binder(at)?;
        let node = self.ir.binder(binder)?.annotation;
        if node == NONE {
            return Ok(None);
        }
        Ok(match self.ir.node(node)? {
            (Op::TyVar, _, capacity, _) => Some(capacity),
            (
                Op::Int
                | Op::Float
                | Op::Bool
                | Op::Str
                | Op::Variant
                | Op::Name
                | Op::To
                | Op::Field
                | Op::Call
                | Op::NamedCall
                | Op::MethodCall
                | Op::NamedMethodCall
                | Op::Past
                | Op::Stepped
                | Op::If
                | Op::Match
                | Op::For
                | Op::Array
                | Op::Index
                | Op::IndexFromEnd
                | Op::Slice
                | Op::Neg
                | Op::Not
                | Op::Add
                | Op::Sub
                | Op::Mul
                | Op::Div
                | Op::IntDiv
                | Op::Rem
                | Op::Pow
                | Op::And
                | Op::Or
                | Op::Lt
                | Op::Le
                | Op::Gt
                | Op::Ge
                | Op::Is
                | Op::IsNot
                | Op::Convert
                | Op::TyScalar
                | Op::TyName
                | Op::TyArray
                | Op::Bad,
                _,
                _,
                _,
            ) => None,
        })
    }

    pub fn annotation_dims(&self, at: u32) -> Result<Vec<u32>, Internal> {
        let rec = self
            .ir
            .lets
            .get(at as usize)
            .ok_or(Internal::missed("IR let"))?;
        self.binder_dims(rec.binder)
    }

    pub fn let_fact(&self, at: u32) -> Result<Fact, Internal> {
        match self.annotation_ty(at)? {
            Some(ty) => {
                let mut fact = annotated_fact(ty);
                fact.dims = self.annotation_dims(at)?;
                Ok(fact)
            }
            None => self.fact(self.let_root(at)?),
        }
    }

    pub fn annotation_refusal(&self, at: u32, ty: EvalTy) -> Result<Refusal, Internal> {
        let fact = self.fact(self.let_root(at)?)?;
        let dims = self.annotation_dims(at)?;
        if dims.is_empty() || matches!(fact.leaf, Leaf::Placeholder | Leaf::Error) {
            return Ok(refusal(&fact, ty));
        }
        if fact.dims != dims {
            return Ok(Refusal::Shape);
        }
        Ok(refusal(
            &Fact {
                dims: Vec::new(),
                ..fact
            },
            ty,
        ))
    }

    fn draw_refused_elements(&mut self, root: u32, ty: EvalTy) -> Result<(), Internal> {
        let mut stack = vec![root];
        while let Some(node) = stack.pop() {
            let fact = self.fact(node)?;
            if self.ir.node(node)?.0 == Op::Array && !fact.dims.is_empty() {
                stack.extend(self.operands(node)?.into_iter().rev());
                continue;
            }
            if refusal(
                &Fact {
                    dims: Vec::new(),
                    ..fact
                },
                ty,
            ) == Refusal::Type
            {
                self.draw_at(Code::TypeMismatch, node, TYPE)?;
            }
        }
        Ok(())
    }

    fn draw_refused_leaves(&mut self, root: u32, ty: EvalTy) -> Result<(), Internal> {
        let mut stack = vec![root];
        while let Some(node) = stack.pop() {
            let fact = self.fact(node)?;
            if self.ir.node(node)?.0 == Op::Array && !fact.dims.is_empty() {
                stack.extend(self.operands(node)?.into_iter().rev());
                continue;
            }
            let leaf = Fact {
                dims: Vec::new(),
                ..fact
            };
            if let Some((code, summary)) = self.leaf_refusal(node, &leaf, ty)? {
                self.draw_at(code, node, summary)?;
            }
        }
        Ok(())
    }

    fn let_span(&self, at: u32) -> Result<Span, Internal> {
        let rec = self
            .ir
            .lets
            .get(at as usize)
            .ok_or(Internal::missed("IR let"))?;
        Ok(self.span(self.file_of(rec.value), rec.lo, rec.hi))
    }

    pub fn check_annotation(&mut self, at: u32) -> Stage<()> {
        let Some(ty) = self.annotation_ty(at)? else {
            return Ok(Ok(()));
        };
        let root = self.let_root(at)?;
        for node in self.postorder(root)? {
            if matches!(self.ir.node(node)?.0, Op::Call | Op::NamedCall) {
                return Ok(Err(Declined(Reason::AnnotatedCall)));
            }
        }
        let chain = std::mem::take(&mut self.eval_chain);
        let outer = std::mem::replace(&mut self.inst, NONE);
        let outcome = self.annotation_check(at, root, ty);
        self.inst = outer;
        self.eval_chain = chain;
        outcome
    }

    fn annotation_check(&mut self, at: u32, root: u32, ty: EvalTy) -> Stage<()> {
        match self.annotation_refusal(at, ty)? {
            Refusal::Shape => {
                let span = self.let_span(at)?;
                self.draw(Code::TypeMismatch, span, SHAPE);
                self.own_check(root)
            }
            Refusal::Type if self.annotation_dims(at)?.is_empty() => {
                self.draw_at(Code::TypeMismatch, root, TYPE)?;
                self.own_check(root)
            }
            Refusal::Type => {
                self.draw_refused_elements(root, ty)?;
                self.own_check(root)
            }
            Refusal::Silent => self.own_check(root),
            Refusal::Kept => Ok(Ok(())),
        }
    }

    pub fn operand_fits(&mut self, at: u32) -> Stage<()> {
        let Some(ty) = self.annotation_ty(at)? else {
            return Ok(Ok(()));
        };
        if (int_range(ty).is_none() && !is_fixed_float(ty))
            || !self.annotation_dims(at)?.is_empty()
            || self.fitting
            || !self.let_reads_cell(at)?
            || self.annotation_refusal(at, ty)? != Refusal::Kept
        {
            return Ok(Ok(()));
        }
        let root = self.let_root(at)?;
        let (drawn, stop) = (self.diagnostics.len(), self.stop.clone());
        let chain = std::mem::take(&mut self.eval_chain);
        let outer = std::mem::replace(&mut self.inst, NONE);
        self.fitting = true;
        let outcome = self.evaluate(root, evaluated_in(ty));
        self.fitting = false;
        self.inst = outer;
        self.eval_chain = chain;
        self.stop = stop;
        for diagnostic in self.diagnostics.split_off(drawn) {
            let fit = diagnostic.code == Code::DoesNotFit
                && !self
                    .diagnostics
                    .iter()
                    .any(|seen| seen.code == diagnostic.code && seen.span == diagnostic.span);
            if fit {
                self.diagnostics.push(diagnostic);
            }
        }
        outcome.map(|outcome| outcome.map(|_| ()))
    }

    pub fn annotation_fit(
        &mut self,
        at: u32,
        ty: EvalTy,
        value: CoreValue,
    ) -> Result<CoreValue, Internal> {
        let (Value::Int(integer), Some(range)) = (&value, int_range(ty)) else {
            return Ok(value);
        };
        if range.contains(integer) {
            return Ok(value);
        }
        let root = self.let_root(at)?;
        let summary = format!("this value does not fit `{}`", ty_name(ty));
        let chain = std::mem::take(&mut self.eval_chain);
        let outcome = if matches!(self.ir.node(root)?.0, Op::Int | Op::Name) {
            self.draw_at(Code::DoesNotFit, root, summary)
        } else {
            self.let_span(at)
                .map(|span| self.draw(Code::DoesNotFit, span, summary))
        };
        self.eval_chain = chain;
        outcome.map(|()| Value::Error)
    }
}

pub(crate) fn refusal(fact: &Fact, ty: EvalTy) -> Refusal {
    match fact.leaf {
        Leaf::Placeholder | Leaf::Error => Refusal::Silent,
        _ if !fact.dims.is_empty() => Refusal::Shape,
        Leaf::Bool if ty == EvalTy::Bool => Refusal::Kept,
        Leaf::Bool => Refusal::Type,
        _ if ty == EvalTy::Bool => Refusal::Type,
        Leaf::Float if int_range(ty).is_some() => Refusal::Type,
        _ if fact.fixed.is_some_and(|own| !widens(own, ty)) => Refusal::Type,
        Leaf::Empty | Leaf::Int | Leaf::Float => Refusal::Kept,
    }
}
