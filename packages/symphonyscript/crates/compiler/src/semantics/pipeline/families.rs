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

type Feed = (u32, u32, u32, Vec<(u32, u32, u32)>);

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
                    let key = self.in_body(*operand)?.map_or(NONE, |(inst, _)| inst);
                    self.takers.push((*operand, family, key));
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
                Op::Index | Op::IndexFromEnd => {
                    let position = if op == Op::Index {
                        "an index"
                    } else {
                        "a `^` offset"
                    };
                    self.unit_position(b, position)?;
                    self.family(a)
                }
                Op::Past => self.family(a),
                Op::Stepped => None,
                Op::Slice => {
                    for at in [b, b + 1] {
                        let bound = self.kid(at)?;
                        if bound != NONE {
                            self.unit_position(bound, "a slice bound")?;
                        }
                    }
                    self.family(a)
                }
                Op::For => match self.fold_family(node)? {
                    Ok(family) => family,
                    Err(declined) => return Ok(Err(declined)),
                },
                Op::Bool
                | Op::Not
                | Op::And
                | Op::Or
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
                    if kids
                        .iter()
                        .any(|kid| self.family(*kid).is_some() && !self.family_faults.contains(kid))
                    {
                        return Ok(Err(Declined(Reason::UnitFlow)));
                    }
                    None
                }
            }
        };
        fact.family = found;
        Ok(Ok(()))
    }

    pub fn unit_position(&mut self, node: u32, position: &str) -> Result<(), Internal> {
        if self.family(node).is_none() || self.family_faults.contains(&node) {
            return Ok(());
        }
        self.family_faults.insert(node);
        self.draw_at(
            Code::UnitOperation,
            node,
            format!("a unit value is not {position}"),
        )
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
        if !self.units_seen {
            return Ok(());
        }
        if let Some(unit) = unit {
            self.unit_feeds.push((root, unit.family, self.feed_env));
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

    fn call_chains(
        &self,
        inst: u32,
        sites: &std::collections::HashMap<u32, Vec<(u32, u32)>>,
    ) -> Result<Vec<Vec<u32>>, Internal> {
        let function = |inst: u32| {
            self.bodies
                .get(inst as usize)
                .is_some_and(|body| body.function < self.functions())
        };
        if inst == NONE || !function(inst) {
            return Ok(vec![Vec::new()]);
        }
        let mut chains = Vec::new();
        let mut stack: Vec<(u32, Vec<u32>)> = vec![(inst, Vec::new())];
        while let Some((at, inner)) = stack.pop() {
            CountedWalk::CoreFamilies.step();
            for (caller, call) in sites.get(&at).map(Vec::as_slice).unwrap_or_default() {
                let mut chain = vec![*call];
                chain.extend(inner.iter().copied());
                if function(*caller) {
                    stack.push((*caller, chain));
                } else {
                    chains.push(chain);
                }
            }
        }
        chains.sort_unstable();
        Ok(chains)
    }

    fn function_inst(&self, inst: u32) -> bool {
        self.bodies
            .get(inst as usize)
            .is_some_and(|body| body.function < self.functions())
    }

    fn fold_family(&mut self, node: u32) -> Stage<Option<FamilyId>> {
        let rec = self.fold_rec(node)?;
        let mut iterable = vec![rec.iterable.0];
        if rec.range {
            iterable.push(rec.iterable.1);
        }
        for at in iterable {
            if self.family(at).is_some() && !self.family_faults.contains(&at) {
                return Ok(Err(Declined(Reason::UnitFlow)));
            }
        }
        let Some(body) = self.body_of_call(node) else {
            return Ok(Ok(self.family(rec.start)));
        };
        let value = self.body_value(body)?;
        let outer = self.inst;
        self.inst = body;
        let found = (self.family(value), self.fact(value));
        let key = self.in_body(value)?.map_or(NONE, |(inst, _)| inst);
        self.inst = outer;
        let (inner, fact) = (found.0, found.1?);
        let start = self.fact(rec.start)?;
        Ok(Ok(match (self.family(rec.start), inner) {
            (Some(left), Some(right)) if left != right => {
                self.draw_at(
                    Code::MixedFamilies,
                    node,
                    "the start and the body of this `for` belong to different unit families, which never convert",
                )?;
                None
            }
            (Some(family), Some(_)) => Some(family),
            (Some(family), None) => {
                if numeric(&fact) && !open(&fact) {
                    self.takers.push((value, family, key));
                }
                Some(family)
            }
            (None, Some(family)) => {
                if numeric(&start) && !open(&start) {
                    self.takers.push((rec.start, family, NONE));
                }
                Some(family)
            }
            (None, None) => None,
        }))
    }

    fn loop_iterable(&self, node: u32, env: u32) -> Result<Option<(u32, u32)>, Internal> {
        let owner = self.binding_owner(node)?;
        if self.loop_at(owner).is_none() {
            return Ok(None);
        }
        let head = self.loop_rec(owner)?.head;
        if head.range {
            return Ok(None);
        }
        let mut at = env;
        while let Some(record) = self.envs.get(at as usize) {
            if self
                .bodies
                .get(record.inst as usize)
                .is_some_and(|body| body.function == owner)
            {
                return Ok(Some((head.iterable.0, record.parent)));
            }
            at = record.parent;
        }
        Ok(None)
    }

    pub fn plain_in_family(&mut self) -> Result<(), Internal> {
        if self.takers.is_empty() {
            return Ok(());
        }
        let mut reached: std::collections::HashSet<(u32, FamilyId, u32, Vec<u32>)> =
            std::collections::HashSet::new();
        let mut walked: std::collections::HashSet<(u32, FamilyId, u32, u32, Vec<u32>)> =
            std::collections::HashSet::new();
        let outer = self.inst;
        for (root, family, env) in std::mem::take(&mut self.unit_feeds) {
            let inst = self
                .envs
                .get(env as usize)
                .map_or(NONE, |record| record.inst);
            let mut stack: Vec<Feed> = vec![(root, env, inst, Vec::new())];
            while let Some((node, env, inst, frames)) = stack.pop() {
                CountedWalk::CoreFamilies.step();
                self.inst = inst;
                let key = self.in_body(node)?.map_or(NONE, |(inst, _)| inst);
                let chain: Vec<u32> = if key == NONE || !self.function_inst(key) {
                    Vec::new()
                } else {
                    frames.iter().map(|(call, _, _)| *call).collect()
                };
                let site = if key == NONE { NONE } else { env };
                if !walked.insert((node, family, key, site, chain.clone())) {
                    continue;
                }
                reached.insert((node, family, key, chain));
                let fact = self.fact(node)?;
                if !open(&fact) && self.family(node) != Some(family) {
                    continue;
                }
                let op = self.ir.node(node)?.0;
                if let (Op::Name, DeclRef::Param(position)) = (op, self.bound(node)?) {
                    if self.binding_owner(node)? < self.functions() {
                        if let Some((call, caller_env, caller_inst)) = frames.last().copied()
                            && let Some(argument) =
                                self.arguments(call)?.get(position as usize).copied()
                        {
                            let mut outer_frames = frames.clone();
                            outer_frames.pop();
                            stack.push((argument, caller_env, caller_inst, outer_frames));
                        }
                    } else if let Some((argument, caller)) =
                        self.argument_of(node, position, env)?
                        && !self.default_nodes.contains_key(&argument)
                    {
                        let caller_inst = self
                            .envs
                            .get(caller as usize)
                            .map_or(NONE, |record| record.inst);
                        stack.push((argument, caller, caller_inst, frames.clone()));
                    } else if let Some((iterable, outer_env)) = self.loop_iterable(node, env)? {
                        let outer_inst = self
                            .envs
                            .get(outer_env as usize)
                            .map_or(NONE, |record| record.inst);
                        stack.push((iterable, outer_env, outer_inst, frames.clone()));
                    }
                    continue;
                }
                if matches!(op, Op::Call | Op::NamedCall)
                    && let Some(body) = self.body_of_call(node)
                {
                    let mut inner = frames.clone();
                    inner.push((node, env, inst));
                    stack.push((self.body_value(body)?, env, body, inner));
                    continue;
                }
                if op == Op::For
                    && let Some(body) = self.body_of_call(node)
                {
                    stack.push((self.body_value(body)?, env, body, frames.clone()));
                }
                for next in self.carried(node)? {
                    stack.push((next, env, inst, frames.clone()));
                }
            }
        }
        let mut held = std::collections::HashSet::new();
        let mut stack: Vec<(u32, u32)> = Vec::new();
        for (local, fact) in self.facts.iter().enumerate() {
            if fact.as_ref().is_some_and(open) {
                stack.push((self.ir.base + u32::try_from(local).unwrap_or(0), NONE));
            }
        }
        for ((inst, node), fact) in &self.body_facts {
            if open(fact) {
                stack.push((*node, *inst));
            }
        }
        while let Some((node, inst)) = stack.pop() {
            CountedWalk::CoreFamilies.step();
            self.inst = inst;
            let key = self.in_body(node)?.map_or(NONE, |(inst, _)| inst);
            if held.insert((node, key)) {
                for next in self.carried(node)? {
                    stack.push((next, inst));
                }
            }
        }
        self.inst = outer;
        let mut takers = std::mem::take(&mut self.takers);
        takers.sort_unstable_by_key(|(node, family, key)| (*node, family.index(), *key));
        takers.dedup();
        let mut sites: std::collections::HashMap<u32, Vec<(u32, u32)>> =
            std::collections::HashMap::new();
        if takers.iter().any(|(_, _, key)| *key != NONE) {
            for ((caller, call), body) in &self.call_body {
                let typed = |inst: &u32| self.typed_as.get(inst).copied().unwrap_or(*inst);
                sites
                    .entry(typed(body))
                    .or_default()
                    .push((typed(caller), *call));
            }
            for found in sites.values_mut() {
                found.sort_unstable();
            }
        }
        for (node, family, key) in takers {
            if held.contains(&(node, key)) {
                continue;
            }
            let name = self.family_name(family)?;
            let summary = format!(
                "this plain value takes the unit family `{name}`, and its family value reaches no attribute of that family to give its unit"
            );
            for chain in self.call_chains(key, &sites)? {
                if reached.contains(&(node, family, key, chain.clone())) {
                    continue;
                }
                let span = self.node_span(node)?;
                let mut related = Vec::new();
                for call in chain.iter().rev() {
                    related.push((self.node_span(*call)?, "in this call".to_string()));
                }
                self.draw_related(Code::PlainInFamily, span, related, summary.clone());
            }
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
