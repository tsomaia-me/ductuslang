use std::collections::HashSet;
use std::rc::Rc;

use symphonyscript_core::FamilyId;

use super::core::Core;
use super::decl_ref::DeclRef;
use super::leaf::Leaf;
use crate::semantics::Internal;
use crate::semantics::counted_walk::CountedWalk;
use crate::semantics::ir::flat::{NONE, Op};

#[derive(Default)]
pub(crate) struct UnitReach {
    pub scaled: HashSet<u32>,
    pub entered: HashSet<u32>,
    pub params: Vec<u32>,
}

#[derive(Clone)]
pub(crate) struct FrameUnit {
    pub family: FamilyId,
    pub unit: usize,
    pub reach: Rc<UnitReach>,
}

impl Core<'_> {
    pub fn taker(&mut self, node: u32, family: FamilyId, key: u32) {
        if self.taker_set.insert((node, family, key)) {
            self.takers.push((node, family, key));
        }
    }

    fn is_taker(&self, node: u32, family: FamilyId) -> Result<bool, Internal> {
        let key = self.in_body(node)?.map_or(NONE, |(inst, _)| inst);
        Ok(self.taker_set.contains(&(node, family, key))
            || self.taker_set.contains(&(node, family, NONE)))
    }

    pub fn frame_unit(
        &mut self,
        root: u32,
        inst: u32,
        family: FamilyId,
        unit: usize,
    ) -> Result<FrameUnit, Internal> {
        let reach = self.unit_reach(root, inst, family)?;
        Ok(FrameUnit {
            family,
            unit,
            reach,
        })
    }

    pub fn unit_reach(
        &mut self,
        root: u32,
        inst: u32,
        family: FamilyId,
    ) -> Result<Rc<UnitReach>, Internal> {
        let outer = self.inst;
        let mut pending = vec![(root, inst)];
        let mut outcome = Ok(());
        while let Some((at, body)) = pending.last().copied() {
            if self.unit_reaches.contains_key(&(at, body, family)) {
                pending.pop();
                continue;
            }
            self.inst = body;
            match self.reach_walk(at, family) {
                Ok(Ok(reach)) => {
                    self.unit_reaches.insert((at, body, family), Rc::new(reach));
                    pending.pop();
                }
                Ok(Err(missing)) => pending.extend(missing),
                Err(internal) => {
                    outcome = Err(internal);
                    break;
                }
            }
        }
        self.inst = outer;
        outcome?;
        self.unit_reaches
            .get(&(root, inst, family))
            .cloned()
            .ok_or(Internal::missed("a unit reach"))
    }

    fn reach_walk(
        &mut self,
        root: u32,
        family: FamilyId,
    ) -> Result<Result<UnitReach, Vec<(u32, u32)>>, Internal> {
        let mut reach = UnitReach::default();
        let mut missing = Vec::new();
        let mut seen = HashSet::new();
        let mut stack = vec![(root, false)];
        while let Some((node, base)) = stack.pop() {
            CountedWalk::CoreFamilies.step();
            if !seen.insert((node, base)) || !self.has_fact(node)? {
                continue;
            }
            let fact = self.fact(node)?;
            if matches!(fact.leaf, Leaf::Placeholder | Leaf::Error) {
                continue;
            }
            match self.family(node) {
                None => {
                    if matches!(fact.leaf, Leaf::Int | Leaf::Float | Leaf::Empty)
                        && (node == root || self.is_taker(node, family)?)
                    {
                        if self.accumulator_read(node)? {
                            continue;
                        }
                        let op = self.ir.node(node)?.0;
                        if matches!(op, Op::Call | Op::NamedCall)
                            && self.body_of_call(node).is_some()
                        {
                            reach.entered.insert(node);
                        } else {
                            reach.scaled.insert(node);
                        }
                    }
                    continue;
                }
                Some(own) if own != family => continue,
                Some(_) => {}
            }
            let (op, a, _, _) = self.ir.node(node)?;
            match op {
                Op::Name => match self.bound(node)? {
                    DeclRef::Let(at) => {
                        let local = self
                            .ir
                            .lets
                            .get(at as usize)
                            .is_some_and(|rec| rec.owner != NONE);
                        if self.cyclic.get(at as usize).copied().unwrap_or(false) {
                            continue;
                        }
                        if !local {
                            reach.entered.insert(node);
                        }
                        if local || base {
                            stack.push((self.let_root(at)?, base));
                        }
                    }
                    DeclRef::Param(position) => {
                        if self.binding_owner(node)? < self.functions() {
                            reach.params.push(position);
                        }
                    }
                    DeclRef::Cell(_)
                    | DeclRef::Instance(_)
                    | DeclRef::Fn(_)
                    | DeclRef::Pattern(_)
                    | DeclRef::Duplicate
                    | DeclRef::None => {}
                },
                Op::Call | Op::NamedCall => {
                    let Some(body) = self.body_of_call(node) else {
                        continue;
                    };
                    reach.entered.insert(node);
                    let value = self.body_value(body)?;
                    let Some(inner) = self.unit_reaches.get(&(value, body, family)).cloned() else {
                        missing.push((value, body));
                        continue;
                    };
                    let arguments = self.arguments(node)?;
                    for position in &inner.params {
                        if let Some(argument) = arguments.get(*position as usize) {
                            stack.push((*argument, false));
                        }
                    }
                }
                Op::For => {
                    if self.body_of_call(node).is_some() {
                        reach.entered.insert(node);
                    }
                    stack.push((self.fold_rec(node)?.start, false));
                }
                Op::Index | Op::IndexFromEnd | Op::Slice => stack.push((a, true)),
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
                | Op::NamedMethodCall => {
                    for operand in self.operands(node)? {
                        stack.push((operand, base && op == Op::Array));
                    }
                }
            }
        }
        if !missing.is_empty() {
            return Ok(Err(missing));
        }
        reach.params.sort_unstable();
        reach.params.dedup();
        Ok(Ok(reach))
    }
}
