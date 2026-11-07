use super::core::{Core, Stage};
use super::decl_ref::DeclRef;
use super::declined::Declined;
use super::fact::Fact;
use super::leaf::Leaf;
use super::reason::Reason;
use super::typing::{JOIN_SINGLE, operand_message};
use crate::diagnostics::Code;
use crate::semantics::Internal;
use crate::semantics::ir::flat::{AttrForm, ItemRec, NONE, Op, binary_of};

const ELEMENTS: &str = "the elements of the array hold values of different types";

fn known(fact: &Fact) -> bool {
    fact.node || !matches!(fact.leaf, Leaf::Placeholder | Leaf::Error)
}

impl Core<'_> {
    pub fn mark_reference_roots(&mut self) -> Result<(), Internal> {
        let mut roots = Vec::new();
        for rec in &self.ir.lets {
            if self.ir.binder(rec.binder)?.annotation == NONE {
                roots.push(rec.value);
            }
        }
        for item in &self.ir.items {
            match item {
                ItemRec::Wire(wire) => roots.push(wire.destination),
                ItemRec::For(rec) if !rec.head.range => roots.push(rec.head.iterable.0),
                ItemRec::Placement(placement) if self.used_pattern(placement.kind).is_some() => {
                    roots.push(placement.default);
                    for entry in self
                        .ir
                        .attr_entries
                        .get(placement.attrs.0 as usize..placement.attrs.1 as usize)
                        .unwrap_or_default()
                    {
                        if let AttrForm::Value(value) = entry.form {
                            roots.push(value);
                        }
                    }
                }
                ItemRec::For(_)
                | ItemRec::Placement(_)
                | ItemRec::Bundle(_)
                | ItemRec::When(_)
                | ItemRec::Line(_)
                | ItemRec::Paren(_) => {}
            }
        }
        roots.sort_unstable();
        self.reference_roots = roots;
        Ok(())
    }

    pub fn holds_references(&self, root: u32) -> bool {
        self.reference_roots.binary_search(&root).is_ok()
    }

    pub fn reference_fact(&mut self, node: u32) -> Stage<Option<Fact>> {
        let (op, a, b, _) = self.ir.node(node)?;
        if op == Op::Field {
            return Ok(Ok(self
                .path_row(node)
                .is_some()
                .then(|| Fact::node_reference(Vec::new()))));
        }
        let operands = self.operands(node)?;
        let mut facts = Vec::with_capacity(operands.len());
        for operand in operands {
            facts.push(self.fact(operand)?);
        }
        if !facts.iter().any(|fact| fact.node) {
            return Ok(Ok(None));
        }
        let joined = facts.iter().any(|fact| !fact.node && !fact.dims.is_empty());
        if joined && binary_of(op).is_some() && op != Op::Add {
            return Ok(Ok(None));
        }
        if joined && op == Op::Add && facts.iter().any(|fact| fact.node && fact.dims.is_empty()) {
            self.draw_at(Code::ArrayOperand, node, JOIN_SINGLE)?;
            return Ok(Ok(Some(Fact::PLACEHOLDER)));
        }
        match op {
            Op::Array => self.reference_array(node, &facts),
            Op::If => self.reference_choice(node, a, &facts),
            Op::Index | Op::IndexFromEnd => self.reference_element(node, (a, b), &facts),
            Op::Int | Op::Float | Op::Bool | Op::Name => Ok(Err(Declined(Reason::NameOfInstance))),
            Op::Neg | Op::Not if facts.iter().any(|fact| !fact.dims.is_empty()) => {
                self.draw_at(Code::ArrayOperand, node, super::typing::OPERAND)?;
                Ok(Ok(Some(Fact::ERROR)))
            }
            Op::Neg => {
                self.draw_at(Code::OperandType, node, "unary `-` takes a number")?;
                Ok(Ok(Some(Fact::ERROR)))
            }
            Op::Not => {
                self.draw_at(Code::OperandType, node, "`not` takes a `bool`")?;
                Ok(Ok(Some(Fact::scalar(Leaf::Bool))))
            }
            Op::Convert => {
                self.draw_at(Code::ConversionType, node, "a conversion takes a number")?;
                if facts.iter().any(|fact| !fact.dims.is_empty()) {
                    return Ok(Ok(Some(Fact::ERROR)));
                }
                let (_, _, code, _) = self.ir.node(node)?;
                let converted = Self::converted_fact(code)?;
                Ok(Ok(Some(self.keep(node, converted)?)))
            }
            Op::Add | Op::Sub | Op::Mul | Op::Div | Op::IntDiv | Op::Rem | Op::Pow => {
                let binary = binary_of(op).ok_or(Internal::missed("operator"))?;
                self.draw_at(Code::OperandType, node, operand_message(binary))?;
                Ok(Ok(Some(Fact::ERROR)))
            }
            Op::And | Op::Or => {
                let refused = |fact: &Fact| {
                    fact.node
                        || (fact.dims.is_empty()
                            && !matches!(fact.leaf, Leaf::Bool | Leaf::Placeholder | Leaf::Error))
                };
                if facts.iter().all(refused) {
                    let binary = binary_of(op).ok_or(Internal::missed("operator"))?;
                    self.draw_at(Code::OperandType, node, operand_message(binary))?;
                    return Ok(Ok(Some(Fact::scalar(Leaf::Bool))));
                }
                for (operand, fact) in self.operands(node)?.into_iter().zip(facts.iter()) {
                    if fact.node && fact.dims.is_empty() {
                        self.draw_at_root(
                            Code::TypeMismatch,
                            operand,
                            "this value has the wrong type here; `bool` is expected",
                        )?;
                    }
                }
                Ok(Ok(Some(Fact::scalar(Leaf::Bool))))
            }
            Op::Lt | Op::Le | Op::Gt | Op::Ge | Op::Is | Op::IsNot => {
                let binary = binary_of(op).ok_or(Internal::missed("operator"))?;
                self.draw_at(Code::OperandType, node, operand_message(binary))?;
                Ok(Ok(Some(Fact::scalar(Leaf::Bool))))
            }
            Op::Call | Op::NamedCall if self.unknown_methods.contains(&node) => {
                let (_, callee, _, _) = self.ir.node(node)?;
                let name = self.ir.node(callee)?.1;
                let text = self.ir.name_text(name).unwrap_or_default().to_string();
                let entry = *self.ir.name(name).ok_or(Internal::missed("IR name"))?;
                let span = self.span(self.file_of(node), entry.lo, entry.hi);
                self.draw(
                    Code::UnknownName,
                    span,
                    format!("`{text}` names nothing in scope"),
                );
                Ok(Ok(Some(Fact::PLACEHOLDER)))
            }
            Op::Call if self.callee(node)?.is_some() => Ok(Ok(None)),
            Op::Slice
                if facts
                    .first()
                    .is_some_and(|base| base.node && !base.dims.is_empty())
                    && facts.iter().skip(1).all(|bound| !bound.node) =>
            {
                Ok(Ok(None))
            }
            Op::Slice | Op::For | Op::Call | Op::NamedCall => {
                Ok(Err(Declined(Reason::NameOfInstance)))
            }
            Op::TyScalar | Op::TyArray | Op::TyName | Op::TyVar | Op::To | Op::Field => {
                Ok(Err(Declined(Reason::NameOfInstance)))
            }
            Op::Past | Op::Stepped | Op::Variant | Op::Bad | Op::Str | Op::Match => {
                Ok(Err(Declined(Reason::NameOfInstance)))
            }
            Op::MethodCall | Op::NamedMethodCall => Ok(Err(Declined(Reason::NameOfInstance))),
        }
    }

    fn reference_array(&mut self, node: u32, facts: &[Fact]) -> Stage<Option<Fact>> {
        let count = u32::try_from(facts.len()).unwrap_or(u32::MAX);
        if facts.iter().any(|fact| !fact.dims.is_empty()) {
            return Ok(Err(Declined(Reason::NameOfInstance)));
        }
        if facts
            .iter()
            .any(|fact| !fact.node && fact.leaf == Leaf::Placeholder)
        {
            return Ok(Ok(Some(Fact::PLACEHOLDER)));
        }
        if facts.iter().all(|fact| fact.node) {
            return Ok(Ok(Some(Fact::node_reference(vec![count]))));
        }
        if facts.iter().all(known) {
            self.draw_at(Code::TypeMismatch, node, ELEMENTS)?;
        }
        Ok(Ok(Some(Fact {
            dims: vec![count],
            ..Fact::ERROR
        })))
    }

    fn reference_choice(
        &mut self,
        node: u32,
        condition: u32,
        facts: &[Fact],
    ) -> Stage<Option<Fact>> {
        let [test, first, second] = facts else {
            return Err(Internal::missed("an if's operands"));
        };
        if test.node || !first.dims.is_empty() || !second.dims.is_empty() {
            return Ok(Err(Declined(Reason::NameOfInstance)));
        }
        if !test.dims.is_empty() {
            self.draw_at(
                Code::ConditionType,
                node,
                "the condition of an `if` is a `bool`, not an array",
            )?;
        } else if !matches!(test.leaf, Leaf::Bool | Leaf::Placeholder | Leaf::Error) {
            self.draw_at_root(Code::ConditionType, condition, "the condition is a `bool`")?;
        }
        if [first, second]
            .iter()
            .any(|fact| !fact.node && fact.leaf == Leaf::Placeholder)
        {
            return Ok(Ok(Some(Fact::PLACEHOLDER)));
        }
        if !(first.node && second.node) {
            if known(first) && known(second) {
                self.draw_at(
                    Code::TypeMismatch,
                    node,
                    "the branches of an `if` hold values of different types",
                )?;
            }
            return Ok(Ok(Some(Fact::ERROR)));
        }
        let fact = Fact::node_reference(Vec::new());
        if !test.dims.is_empty() || test.leaf != Leaf::Bool {
            return Ok(Ok(Some(self.keep(node, fact)?)));
        }
        Ok(Ok(Some(fact)))
    }

    fn reference_element(
        &mut self,
        node: u32,
        (base, index): (u32, u32),
        facts: &[Fact],
    ) -> Stage<Option<Fact>> {
        let [array, position] = facts else {
            return Err(Internal::missed("an index's operands"));
        };
        if position.node || !array.node {
            return Ok(Err(Declined(Reason::NameOfInstance)));
        }
        let faulty = matches!(position.leaf, Leaf::Placeholder | Leaf::Error);
        if !position.dims.is_empty() && !faulty {
            self.draw_at(
                Code::IndexType,
                node,
                "an index is an integer, not an array",
            )?;
            return Ok(Ok(Some(Fact::PLACEHOLDER)));
        }
        if array.dims.is_empty() {
            if position.leaf == Leaf::Placeholder {
                return Ok(Err(Declined(Reason::NameOfInstance)));
            }
            self.draw_at(Code::NotAnArray, base, "only an array is indexed")?;
            return Ok(Ok(Some(Fact::PLACEHOLDER)));
        }
        let refused = !faulty && matches!(position.leaf, Leaf::Float | Leaf::Bool);
        if refused {
            self.draw_at_root(Code::IndexType, index, "an index is an integer")?;
        }
        if position.leaf == Leaf::Placeholder {
            return Ok(Ok(Some(Fact::PLACEHOLDER)));
        }
        let element = Fact::node_reference(array.dims.get(1..).unwrap_or_default().to_vec());
        if refused || faulty {
            return Ok(Ok(Some(self.keep(node, element)?)));
        }
        Ok(Ok(Some(element)))
    }

    pub fn reference_elements(&self, base: u32) -> Result<Option<Vec<u32>>, Internal> {
        let mut node = base;
        loop {
            let (op, a, b, _) = self.ir.node(node)?;
            if op == Op::Array && b > 0 {
                return (a..a + b)
                    .map(|at| self.kid(at))
                    .collect::<Result<Vec<u32>, Internal>>()
                    .map(Some);
            }
            let DeclRef::Let(at) = self.bound(node)? else {
                return Ok(None);
            };
            if op != Op::Name || self.cyclic.get(at as usize).copied().unwrap_or(true) {
                return Ok(None);
            }
            node = self.let_root(at)?;
        }
    }
}
