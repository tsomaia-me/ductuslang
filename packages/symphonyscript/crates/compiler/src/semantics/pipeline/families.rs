use symphonyscript_core::FamilyId;

use super::core::{Core, Stage};
use super::decl_ref::DeclRef;
use super::declined::Declined;
use super::fact::Fact;
use super::leaf::Leaf;
use super::reason::Reason;
use crate::diagnostics::Code;
use crate::semantics::Internal;
use crate::semantics::counted_walk::CountedWalk;
use crate::semantics::ir::Suffix;
use crate::semantics::ir::flat::{NONE, Op};

fn open(fact: &Fact) -> bool {
    matches!(fact.leaf, Leaf::Placeholder | Leaf::Error)
}

fn numeric(fact: &Fact) -> bool {
    matches!(fact.leaf, Leaf::Int | Leaf::Float | Leaf::Empty)
}

impl Core<'_> {
    pub fn family(&self, node: u32) -> Option<FamilyId> {
        self.fact(node).ok().and_then(|fact| fact.family)
    }

    pub fn unit_scale(&self, family: FamilyId, unit: usize) -> Result<f64, Internal> {
        Ok(self
            .vocabulary
            .family(family)
            .and_then(|decl| decl.units.get(unit))
            .ok_or(Internal::missed("unit"))?
            .scale)
    }

    fn family_name(&self, family: FamilyId) -> Result<String, Internal> {
        Ok(self
            .vocabulary
            .family(family)
            .ok_or(Internal::missed("unit family"))?
            .name
            .clone())
    }

    fn one_family(
        &mut self,
        node: u32,
        operands: &[u32],
    ) -> Result<Option<Option<FamilyId>>, Internal> {
        let mut family: Option<FamilyId> = None;
        let mut facts = Vec::with_capacity(operands.len());
        for operand in operands {
            facts.push(self.fact(*operand)?);
        }
        for (operand, fact) in operands.iter().zip(&facts) {
            if open(fact) {
                continue;
            }
            match (family, self.family(*operand)) {
                (Some(existing), Some(other)) if existing != other => {
                    let names = format!(
                        "`{}` and `{}`",
                        self.family_name(existing)?,
                        self.family_name(other)?
                    );
                    self.draw_at(
                        Code::MixedFamilies,
                        node,
                        format!(
                            "these values belong to the unit families {names}, which never convert"
                        ),
                    )?;
                    return Ok(None);
                }
                (None, Some(other)) => family = Some(other),
                _ => {}
            }
        }
        if let Some(family) = family {
            for (operand, fact) in operands.iter().zip(&facts) {
                if self.family(*operand).is_none() && numeric(fact) && !open(fact) {
                    self.takers.push((*operand, family));
                }
            }
        }
        Ok(Some(family))
    }

    pub fn node_family(&mut self, node: u32, fact: &mut Fact) -> Stage<()> {
        if !self.units_seen {
            return Ok(Ok(()));
        }
        let (op, a, b, c) = self.ir.node(node)?;
        let kids = self.operands(node)?;
        let in_body = self.in_body(node)?.is_some();
        let takers = self.takers.len();
        let found = if open(fact) {
            None
        } else {
            match op {
                Op::Int | Op::Float => match self.literal_suffix(node)? {
                    Suffix::Unit { family, .. } => Some(family),
                    Suffix::None | Suffix::Type(_) => None,
                },
                Op::Name => match self.bound(node)? {
                    DeclRef::Let(at) => self.family(self.let_root(at)?),
                    DeclRef::Cell(at) if !self.is_cyclic_cell(at) => {
                        self.family(self.cell_root(at)?)
                    }
                    DeclRef::Param(_) => fact.family,
                    DeclRef::Cell(_)
                    | DeclRef::Instance(_)
                    | DeclRef::Fn(_)
                    | DeclRef::Pattern(_)
                    | DeclRef::Duplicate
                    | DeclRef::None => None,
                },
                Op::Neg | Op::Convert => self.family(a),
                Op::Add | Op::Sub | Op::Rem => match self.one_family(node, &[a, b])? {
                    Some(family) => family,
                    None => return self.family_error(node, fact),
                },
                Op::Lt | Op::Le | Op::Gt | Op::Ge | Op::Is | Op::IsNot => {
                    if numeric(&self.fact(a)?) && numeric(&self.fact(b)?) {
                        self.one_family(node, &[a, b])?;
                    }
                    None
                }
                Op::Mul => match (self.family(a), self.family(b)) {
                    (Some(_), Some(_)) => {
                        self.draw_at(
                            Code::UnitOperation,
                            node,
                            "two unit values are not multiplied",
                        )?;
                        return self.family_error(node, fact);
                    }
                    (family, None) | (None, family) => family,
                },
                Op::Pow => {
                    if self.family(a).is_some() || self.family(b).is_some() {
                        self.draw_at(Code::UnitOperation, node, "`**` takes no unit value")?;
                        return self.family_error(node, fact);
                    }
                    None
                }
                Op::Div | Op::IntDiv => match (self.family(a), self.family(b)) {
                    (Some(x), Some(y)) if x == y => None,
                    (Some(_), Some(_)) => {
                        self.draw_at(
                            Code::MixedFamilies,
                            node,
                            "these values belong to different unit families",
                        )?;
                        return self.family_error(node, fact);
                    }
                    (Some(family), None) => Some(family),
                    (None, Some(_)) => {
                        self.draw_at(
                            Code::UnitOperation,
                            node,
                            "a plain number is not divided by a unit value",
                        )?;
                        return self.family_error(node, fact);
                    }
                    (None, None) => None,
                },
                Op::If => match self.one_family(node, &[b, c])? {
                    Some(family) => family,
                    None => return self.family_error(node, fact),
                },
                Op::Array => match self.one_family(node, &kids)? {
                    Some(family) => family,
                    None => return self.family_error(node, fact),
                },
                Op::Call | Op::NamedCall => fact.family,
                Op::Index => {
                    if self.family(b).is_some() {
                        return Ok(Err(Declined(Reason::UnitFlow)));
                    }
                    self.family(a)
                }
                Op::Past => self.family(a),
                Op::Stepped => None,
                Op::Slice => {
                    for at in [b, b + 1] {
                        let bound = self.kid(at)?;
                        if bound != NONE && self.family(bound).is_some() {
                            return Ok(Err(Declined(Reason::UnitFlow)));
                        }
                    }
                    self.family(a)
                }
                Op::Bool
                | Op::Not
                | Op::And
                | Op::Or
                | Op::IndexFromEnd
                | Op::For
                | Op::Field
                | Op::To
                | Op::Variant
                | Op::Bad
                | Op::TyScalar
                | Op::TyArray
                | Op::TyName
                | Op::TyVar
                | Op::Str
                | Op::Match
                | Op::MethodCall
                | Op::NamedMethodCall => {
                    if kids.iter().any(|kid| self.family(*kid).is_some()) {
                        return Ok(Err(Declined(Reason::UnitFlow)));
                    }
                    None
                }
            }
        };
        fact.family = found;
        if in_body && self.takers.len() > takers {
            return Ok(Err(Declined(Reason::UnitFlow)));
        }
        Ok(Ok(()))
    }

    fn family_error(&mut self, node: u32, fact: &mut Fact) -> Stage<()> {
        self.family_faults.insert(node);
        *fact = Fact::ERROR;
        Ok(Ok(()))
    }

    pub fn bind_unit(
        &mut self,
        root: u32,
        unit: Option<symphonyscript_core::AttrUnit>,
    ) -> Result<(), Internal> {
        if let Some(unit) = unit {
            self.unit_feeds.push((root, unit.family));
        }
        let fact = self.fact(root)?;
        if open(&fact) || !numeric(&fact) {
            return Ok(());
        }
        match (self.family(root), unit) {
            (Some(_), None) => self.draw_at_root(
                Code::UnitMismatch,
                root,
                "a unit value is bound to an attribute of no unit family",
            ),
            (Some(family), Some(unit)) if family != unit.family => self.draw_at_root(
                Code::UnitMismatch,
                root,
                "a unit value is bound to an attribute of another unit family",
            ),
            (Some(_) | None, Some(_) | None) => Ok(()),
        }
    }

    fn carried(&self, node: u32) -> Result<Vec<u32>, Internal> {
        let (op, a, _, _) = self.ir.node(node)?;
        Ok(match op {
            Op::Name => match self.bound(node)? {
                DeclRef::Let(at) => vec![self.let_root(at)?],
                DeclRef::Cell(at) => vec![self.cell_root(at)?],
                DeclRef::Param(_)
                | DeclRef::Instance(_)
                | DeclRef::Fn(_)
                | DeclRef::Pattern(_)
                | DeclRef::Duplicate
                | DeclRef::None => Vec::new(),
            },
            Op::Index | Op::IndexFromEnd | Op::Slice => vec![a],
            Op::Call | Op::NamedCall => self.arguments(node)?,
            Op::Int
            | Op::Float
            | Op::Bool
            | Op::Neg
            | Op::Add
            | Op::Sub
            | Op::Mul
            | Op::Div
            | Op::IntDiv
            | Op::Rem
            | Op::Pow
            | Op::Not
            | Op::And
            | Op::Or
            | Op::Lt
            | Op::Le
            | Op::Gt
            | Op::Ge
            | Op::Is
            | Op::IsNot
            | Op::If
            | Op::Array
            | Op::Convert
            | Op::For
            | Op::Past
            | Op::Stepped
            | Op::Field
            | Op::To
            | Op::Variant
            | Op::Bad
            | Op::TyScalar
            | Op::TyArray
            | Op::TyName
            | Op::TyVar
            | Op::Str
            | Op::Match
            | Op::MethodCall
            | Op::NamedMethodCall => self.operands(node)?,
        })
    }

    pub fn plain_in_family(&mut self) -> Result<(), Internal> {
        if self.takers.is_empty() {
            return Ok(());
        }
        let mut reached: std::collections::HashSet<(u32, FamilyId)> =
            std::collections::HashSet::new();
        for (root, family) in std::mem::take(&mut self.unit_feeds) {
            let mut stack = vec![root];
            while let Some(node) = stack.pop() {
                CountedWalk::CoreFamilies.step();
                if !reached.insert((node, family)) {
                    continue;
                }
                let fact = self.fact(node)?;
                if open(&fact) || self.family(node) == Some(family) {
                    stack.extend(self.carried(node)?);
                }
            }
        }
        let mut held = std::collections::HashSet::new();
        let mut stack: Vec<u32> = Vec::new();
        for (local, fact) in self.facts.iter().enumerate() {
            if fact.as_ref().is_some_and(open) {
                stack.push(self.ir.base + u32::try_from(local).unwrap_or(0));
            }
        }
        while let Some(node) = stack.pop() {
            CountedWalk::CoreFamilies.step();
            if held.insert(node) {
                stack.extend(self.carried(node)?);
            }
        }
        let mut takers = std::mem::take(&mut self.takers);
        takers.sort_unstable_by_key(|(node, family)| (*node, family.index()));
        takers.dedup();
        for (node, family) in takers {
            if reached.contains(&(node, family)) || held.contains(&node) {
                continue;
            }
            let name = self.family_name(family)?;
            self.draw_at(
                Code::PlainInFamily,
                node,
                format!(
                    "this plain value takes the unit family `{name}`, and its family value reaches no attribute of that family to give its unit"
                ),
            )?;
        }
        Ok(())
    }
}

impl Core<'_> {
    pub fn unit_literal(
        &mut self,
        node: u32,
        ty: crate::semantics::operators::EvalTy,
    ) -> Result<Option<super::core_value::CoreValue>, Internal> {
        if !self.units_seen {
            return Ok(None);
        }
        let (op, a, _, _) = self.ir.node(node)?;
        let Suffix::Unit { family, unit } = self.literal_suffix(node)? else {
            return Ok(None);
        };
        let scale = self.unit_scale(family, unit)?;
        if op == Op::Int {
            let value = *self
                .ir
                .ints
                .get(a as usize)
                .ok_or(Internal::missed("IR integer"))?;
            if scale.fract() == 0.0 && scale.abs() < 1e38 {
                return match value.checked_mul(scale as i128) {
                    Some(scaled) if scaled != i128::MIN => {
                        self.integer_literal(node, scaled, ty).map(Some)
                    }
                    Some(_) | None => {
                        self.draw_at(
                            Code::IntegerOverflow,
                            node,
                            "this value's magnitude reaches 2¹²⁷",
                        )?;
                        Ok(Some(crate::semantics::operators::Value::Error))
                    }
                };
            }
            let literal = crate::semantics::ir::FloatLit::from_integer(value).scaled(scale)?;
            return self.float_literal(node, &literal, ty).map(Some);
        }
        let entry = *self
            .ir
            .floats
            .get(a as usize)
            .ok_or(Internal::missed("IR float"))?;
        let literal = crate::semantics::ir::FloatLit {
            negative: entry.negative,
            digits: self.ir.text(entry.digits).to_string(),
            exponent: entry.exponent,
        }
        .scaled(scale)?;
        self.float_literal(node, &literal, ty).map(Some)
    }
}
