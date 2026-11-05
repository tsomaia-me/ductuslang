use super::core::Core;
use super::decl_ref::DeclRef;
use crate::diagnostics::{Code, Diagnostic};
use crate::semantics::Internal;
use crate::semantics::ir::flat::{ForRec, Op};
use crate::semantics::operators::{EvalTy, Value};

const DEPTH: usize = 256;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct Extremes {
    pub min: Option<i128>,
    pub max: Option<i128>,
}

impl Extremes {
    const UNBOUNDED: Extremes = Extremes {
        min: None,
        max: None,
    };

    fn exactly(value: i128) -> Extremes {
        Extremes {
            min: Some(value),
            max: Some(value),
        }
    }

    fn sum(self, other: Extremes) -> Extremes {
        let add = |a: Option<i128>, b: Option<i128>| a.zip(b).map(|(a, b)| a.saturating_add(b));
        Extremes {
            min: add(self.min, other.min),
            max: add(self.max, other.max),
        }
    }

    fn difference(self, other: Extremes) -> Extremes {
        let sub = |a: Option<i128>, b: Option<i128>| a.zip(b).map(|(a, b)| a.saturating_sub(b));
        Extremes {
            min: sub(self.min, other.max),
            max: sub(self.max, other.min),
        }
    }
}

impl Core<'_> {
    pub fn bound_extremes(&mut self, node: u32) -> Result<Extremes, Internal> {
        self.extremes_at(node, 0)
    }

    fn extremes_at(&mut self, node: u32, depth: usize) -> Result<Extremes, Internal> {
        if depth > DEPTH {
            return Ok(Extremes::UNBOUNDED);
        }
        if !self.is_reactive(node)? {
            let chain = std::mem::take(&mut self.eval_chain);
            let (drawn, stop) = (self.diagnostics.len(), self.stop.clone());
            let value = self.evaluate(node, EvalTy::Exact)?;
            self.diagnostics.truncate(drawn);
            self.stop = stop;
            self.eval_chain = chain;
            return Ok(match value {
                Ok(Value::Int(value)) => Extremes::exactly(value),
                Ok(_) | Err(_) => Extremes::UNBOUNDED,
            });
        }
        let (op, a, b, _) = self.ir.node(node)?;
        Ok(match op {
            Op::Name => match self.bound(node)? {
                DeclRef::Let(at) if self.let_reads_cell(at)? => Extremes::exactly(0),
                DeclRef::Let(at) => {
                    let root = self.let_root(at)?;
                    self.extremes_at(root, depth + 1)?
                }
                DeclRef::Cell(_)
                | DeclRef::Param(_)
                | DeclRef::None
                | DeclRef::Instance(_)
                | DeclRef::Duplicate
                | DeclRef::Fn(_)
                | DeclRef::Pattern(_) => Extremes::UNBOUNDED,
            },
            Op::Add => {
                let left = self.extremes_at(a, depth + 1)?;
                left.sum(self.extremes_at(b, depth + 1)?)
            }
            Op::Sub => {
                let left = self.extremes_at(a, depth + 1)?;
                left.difference(self.extremes_at(b, depth + 1)?)
            }
            Op::Int
            | Op::Float
            | Op::Bool
            | Op::Str
            | Op::Variant
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
            | Op::TyVar
            | Op::Bad => Extremes::UNBOUNDED,
        })
    }

    pub fn reactive_range_bounds(&mut self, rec: &ForRec) -> Result<(), Internal> {
        for (bound, upper) in [(rec.iterable.0, false), (rec.iterable.1, true)] {
            if !self.is_reactive(bound)? {
                continue;
            }
            let extremes = self.bound_extremes(bound)?;
            let (side, example, summary) = if upper {
                (
                    extremes.max,
                    "0..min(n, 16)",
                    "this upper bound of the range reads a cell, and its form gives it no maximum",
                )
            } else {
                (
                    extremes.min,
                    "max(n, 0)..16",
                    "this lower bound of the range reads a cell, and its form gives it no minimum",
                )
            };
            if side.is_some() {
                continue;
            }
            let span = self.node_span(bound)?;
            let diagnostic = Diagnostic::new(Code::RangeBound, span, summary)
                .with_help(format!("bound it by its form, as in `{example}`"));
            let diagnostic = self.chained(diagnostic);
            self.push_drawn(diagnostic);
        }
        Ok(())
    }
}
