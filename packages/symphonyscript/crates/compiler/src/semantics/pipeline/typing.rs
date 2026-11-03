use super::core::{Core, Stage};
use super::decl_ref::DeclRef;
use super::declined::Declined;
use super::fact::Fact;
use super::leaf::Leaf;
use crate::diagnostics::Code;
use crate::semantics::Internal;
use crate::semantics::counted_walk::CountedWalk;
use crate::semantics::ir::flat::Op;

const OPERAND: &str = "operators and conversions apply to single values, not arrays";
const SINGLE_OPERANDS: &str =
    "only `+` takes arrays, which it joins; every other operator takes single values";
const JOIN_SINGLE: &str = "`+` joins two arrays, and one of these is a single value";
const RAGGED: &str = "the elements of an array are arrays of one length, or all single values";

const NEW: u8 = 0;
const OPEN: u8 = 1;
const DONE: u8 = 2;

impl Core<'_> {
    pub fn operands(&self, node: u32) -> Result<Vec<u32>, Internal> {
        let (op, a, b, _) = self.ir.node(node)?;
        Ok(match op {
            Op::Neg => vec![a],
            Op::Add | Op::Sub | Op::Mul | Op::Index => vec![a, b],
            Op::Array => (a..a + b)
                .map(|at| self.kid(at))
                .collect::<Result<_, _>>()?,
            Op::Int
            | Op::Float
            | Op::Bool
            | Op::Name
            | Op::Str
            | Op::Variant
            | Op::To
            | Op::Convert
            | Op::Field
            | Op::IndexFromEnd
            | Op::Slice
            | Op::Call
            | Op::NamedCall
            | Op::MethodCall
            | Op::NamedMethodCall
            | Op::Past
            | Op::Stepped
            | Op::Not
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
            | Op::If
            | Op::Match
            | Op::For
            | Op::TyScalar
            | Op::TyName
            | Op::TyArray
            | Op::TyVar
            | Op::Bad => Vec::new(),
        })
    }

    pub fn postorder(&self, root: u32) -> Result<Vec<u32>, Internal> {
        let mut order = Vec::new();
        let mut stack = vec![(root, false)];
        while let Some((node, visited)) = stack.pop() {
            if visited {
                order.push(node);
                continue;
            }
            stack.push((node, true));
            for operand in self.operands(node)?.into_iter().rev() {
                stack.push((operand, false));
            }
        }
        Ok(order)
    }

    pub fn let_root(&self, at: u32) -> Result<u32, Internal> {
        Ok(self
            .ir
            .lets
            .get(at as usize)
            .ok_or(Internal::missed("IR let"))?
            .value)
    }

    fn read_lets(&self, root: u32) -> Result<Vec<u32>, Internal> {
        let mut reads = Vec::new();
        for node in self.postorder(root)? {
            if let DeclRef::Let(at) = self.bound(node)? {
                reads.push(at);
            }
        }
        Ok(reads)
    }

    pub fn bound(&self, node: u32) -> Result<DeclRef, Internal> {
        let local = self.local(node)?;
        Ok(self.bind.get(local).copied().unwrap_or(DeclRef::None))
    }

    pub fn fact(&self, node: u32) -> Result<Fact, Internal> {
        let local = self.local(node)?;
        self.facts
            .get(local)
            .cloned()
            .flatten()
            .ok_or(Internal::missed("new-core fact"))
    }

    pub fn ensure_typed(&mut self, root: u32) -> Stage<Fact> {
        let mut stack: Vec<(u32, bool)> = Vec::new();
        for at in self.read_lets(root)?.into_iter().rev() {
            stack.push((at, false));
        }
        while let Some((at, expanded)) = stack.pop() {
            let cyclic = self.cyclic.get(at as usize).copied().unwrap_or(false);
            let state = self.let_state.get(at as usize).copied().unwrap_or(DONE);
            if cyclic || state == DONE {
                continue;
            }
            if expanded {
                if let Err(declined) = self.type_root(self.let_root(at)?)? {
                    return Ok(Err(declined));
                }
                if let Some(slot) = self.let_state.get_mut(at as usize) {
                    *slot = DONE;
                }
                continue;
            }
            if state == OPEN {
                return Err(Internal::missed("an open declaration outside every cycle"));
            }
            if let Some(slot) = self.let_state.get_mut(at as usize) {
                *slot = OPEN;
            }
            stack.push((at, true));
            for read in self.read_lets(self.let_root(at)?)?.into_iter().rev() {
                if self.let_state.get(read as usize) == Some(&NEW) {
                    stack.push((read, false));
                }
            }
        }
        if let Err(declined) = self.type_root(root)? {
            return Ok(Err(declined));
        }
        Ok(Ok(self.fact(root)?))
    }

    fn type_root(&mut self, root: u32) -> Stage<()> {
        if self
            .facts
            .get(self.local(root)?)
            .is_some_and(Option::is_some)
        {
            return Ok(Ok(()));
        }
        for node in self.postorder(root)? {
            CountedWalk::SigNode.step();
            let fact = match self.node_fact(node)? {
                Ok(fact) => fact,
                Err(declined) => return Ok(Err(declined)),
            };
            let local = self.local(node)?;
            if let Some(slot) = self.facts.get_mut(local) {
                *slot = Some(fact);
            }
        }
        Ok(Ok(()))
    }

    fn node_fact(&mut self, node: u32) -> Stage<Fact> {
        let (op, a, b, _) = self.ir.node(node)?;
        Ok(Ok(match op {
            Op::Int => Fact::scalar(Leaf::Int),
            Op::Float => Fact::scalar(Leaf::Float),
            Op::Bool => Fact::scalar(Leaf::Bool),
            Op::Name => match self.bound(node)? {
                DeclRef::Let(at) => {
                    if self.cyclic.get(at as usize).copied().unwrap_or(false) {
                        Fact::ANY
                    } else {
                        self.fact(self.let_root(at)?)?
                    }
                }
                DeclRef::Duplicate | DeclRef::None => Fact::ANY,
                DeclRef::Instance(_) => return Ok(Err(Declined)),
            },
            Op::Neg => {
                let operand = self.fact(a)?;
                match operand.leaf {
                    Leaf::Any => return Ok(Err(Declined)),
                    Leaf::Error => Fact::ERROR,
                    _ if !operand.dims.is_empty() => {
                        self.draw_at(Code::ArrayOperand, node, OPERAND)?;
                        Fact::ERROR
                    }
                    Leaf::Bool => {
                        self.draw_at(Code::OperandType, node, "unary `-` takes a number")?;
                        Fact::ERROR
                    }
                    Leaf::Int | Leaf::Float => operand,
                }
            }
            Op::Add | Op::Sub | Op::Mul => return self.arith_fact(node, op, a, b),
            Op::Array => return self.array_fact(node, a, b),
            Op::Index => {
                let base = self.fact(a)?;
                let index = self.fact(b)?;
                if base.leaf == Leaf::Any || index.leaf == Leaf::Any || !index.dims.is_empty() {
                    return Ok(Err(Declined));
                }
                if matches!(index.leaf, Leaf::Float | Leaf::Bool) {
                    self.draw_at(Code::IndexType, b, "an index is an integer")?;
                }
                if base.leaf == Leaf::Error {
                    return Ok(Ok(Fact::ERROR));
                }
                if base.dims.is_empty() {
                    self.draw_at(Code::NotAnArray, a, "only an array is indexed")?;
                    return Ok(Ok(Fact::ERROR));
                }
                Fact {
                    leaf: base.leaf,
                    dims: base.dims.get(1..).unwrap_or_default().to_vec(),
                }
            }
            Op::Str
            | Op::Variant
            | Op::To
            | Op::Convert
            | Op::Field
            | Op::IndexFromEnd
            | Op::Slice
            | Op::Call
            | Op::NamedCall
            | Op::MethodCall
            | Op::NamedMethodCall
            | Op::Past
            | Op::Stepped
            | Op::Not
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
            | Op::If
            | Op::Match
            | Op::For
            | Op::TyScalar
            | Op::TyName
            | Op::TyArray
            | Op::TyVar
            | Op::Bad => return Ok(Err(Declined)),
        }))
    }

    fn arith_fact(&mut self, node: u32, op: Op, a: u32, b: u32) -> Stage<Fact> {
        let left = self.fact(a)?;
        let right = self.fact(b)?;
        if left.leaf == Leaf::Any || right.leaf == Leaf::Any {
            return Ok(Err(Declined));
        }
        if left.leaf == Leaf::Error || right.leaf == Leaf::Error {
            return Ok(Ok(Fact::ERROR));
        }
        let arrays = (!left.dims.is_empty(), !right.dims.is_empty());
        if arrays != (false, false) {
            if op != Op::Add {
                self.draw_at(Code::ArrayOperand, node, SINGLE_OPERANDS)?;
                return Ok(Ok(Fact::ERROR));
            }
            if arrays != (true, true) {
                if left.leaf == Leaf::Bool || right.leaf == Leaf::Bool {
                    return Ok(Err(Declined));
                }
                self.draw_at(Code::ArrayOperand, node, JOIN_SINGLE)?;
                return Ok(Ok(Fact::ERROR));
            }
            let (Some(left_len), Some(right_len)) = (left.dims.first(), right.dims.first()) else {
                return Err(Internal::missed("array length"));
            };
            if left.dims.get(1..) != right.dims.get(1..)
                || (left.leaf == Leaf::Bool) != (right.leaf == Leaf::Bool)
                || *left_len == 0
                || *right_len == 0
            {
                return Ok(Err(Declined));
            }
            let mut dims = left.dims.clone();
            if let Some(first) = dims.first_mut() {
                *first = left_len.saturating_add(*right_len);
            }
            return Ok(Ok(Fact {
                leaf: numeric(left.leaf, right.leaf),
                dims,
            }));
        }
        if left.leaf == Leaf::Bool || right.leaf == Leaf::Bool {
            let message = if op == Op::Add {
                "`+` takes two numbers or two strings"
            } else {
                "this operator takes numbers"
            };
            self.draw_at(Code::OperandType, node, message)?;
            return Ok(Ok(Fact::ERROR));
        }
        Ok(Ok(Fact::scalar(numeric(left.leaf, right.leaf))))
    }

    fn array_fact(&mut self, node: u32, start: u32, count: u32) -> Stage<Fact> {
        if count == 0 {
            return Ok(Err(Declined));
        }
        let mut facts = Vec::new();
        for at in start..start + count {
            facts.push(self.fact(self.kid(at)?)?);
        }
        if facts.iter().any(|fact| fact.leaf == Leaf::Any) {
            return Ok(Err(Declined));
        }
        if facts.iter().any(|fact| fact.leaf == Leaf::Error) {
            return Ok(Ok(Fact::ERROR));
        }
        let first = facts.first().cloned().ok_or(Internal::missed("element"))?;
        let mut leaf = first.leaf;
        for fact in facts.iter().skip(1) {
            if (fact.leaf == Leaf::Bool) != (leaf == Leaf::Bool) {
                self.draw_at(
                    Code::TypeMismatch,
                    node,
                    "the elements of the array hold values of different types",
                )?;
                return Ok(Ok(Fact::ERROR));
            }
            leaf = numeric(leaf, fact.leaf);
        }
        if facts.iter().any(|fact| fact.dims != first.dims) {
            self.draw_at(Code::RaggedArray, node, RAGGED)?;
            return Ok(Ok(Fact::ERROR));
        }
        let mut dims = vec![count];
        dims.extend(first.dims.iter().copied());
        Ok(Ok(Fact { leaf, dims }))
    }
}

fn numeric(a: Leaf, b: Leaf) -> Leaf {
    match (a, b) {
        (Leaf::Bool, Leaf::Bool) => Leaf::Bool,
        (Leaf::Int, Leaf::Int) => Leaf::Int,
        _ => Leaf::Float,
    }
}
