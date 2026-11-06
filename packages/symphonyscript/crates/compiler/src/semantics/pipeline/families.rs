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

#[derive(Default)]
pub(super) struct FeedFound {
    pub reached: std::collections::HashSet<(u32, FamilyId, u32)>,
    pub fed: std::collections::HashSet<(u32, u32, FamilyId)>,
    pub folds: std::collections::HashSet<(u32, u32)>,
}

struct Link {
    length: usize,
    call: u32,
    callee: u32,
    unfed_below: bool,
}

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
                    self.taker(*operand, family, key);
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
        if let Some(unit) = unit {
            self.unit_feeds
                .push((root, unit.family, unit.default_unit, self.feed_env));
        }
        if !self.units_seen {
            return Ok(());
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

    pub(super) fn function_inst(&self, inst: u32) -> bool {
        self.bodies
            .get(inst as usize)
            .is_some_and(|body| body.function < self.functions())
    }

    fn fold_family(&mut self, node: u32) -> Stage<Option<FamilyId>> {
        let rec = self.fold_rec(node)?;
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
                    self.taker(value, family, key);
                }
                Some(family)
            }
            (None, Some(family)) => {
                if numeric(&start) && !open(&start) {
                    self.taker(rec.start, family, NONE);
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

    pub(super) fn feed_walk(
        &mut self,
        start: (u32, u32, u32),
        family: FamilyId,
        summaries: &std::collections::HashMap<(u32, FamilyId), Vec<u32>>,
        found: &mut FeedFound,
        params: &mut Vec<u32>,
    ) -> Result<(), Internal> {
        let mut walked: std::collections::HashSet<(u32, u32, u32)> =
            std::collections::HashSet::new();
        let mut stack = vec![start];
        while let Some((node, env, inst)) = stack.pop() {
            CountedWalk::CoreFamilies.step();
            self.inst = inst;
            let key = self.in_body(node)?.map_or(NONE, |(inst, _)| inst);
            let site = if key == NONE { NONE } else { env };
            if !walked.insert((node, key, site)) {
                continue;
            }
            found.reached.insert((node, family, key));
            let fact = self.fact(node)?;
            if !open(&fact) && self.family(node) != Some(family) {
                continue;
            }
            let op = self.ir.node(node)?.0;
            if let (Op::Name, DeclRef::Param(position)) = (op, self.bound(node)?) {
                if self.binding_owner(node)? < self.functions() {
                    params.push(position);
                } else if let Some((argument, caller)) = self.argument_of(node, position, env)?
                    && !self.default_nodes.contains_key(&argument)
                {
                    let caller_inst = self
                        .envs
                        .get(caller as usize)
                        .map_or(NONE, |record| record.inst);
                    stack.push((argument, caller, caller_inst));
                } else if let Some((iterable, outer_env)) = self.loop_iterable(node, env)? {
                    let outer_inst = self
                        .envs
                        .get(outer_env as usize)
                        .map_or(NONE, |record| record.inst);
                    stack.push((iterable, outer_env, outer_inst));
                }
                continue;
            }
            if matches!(op, Op::Call | Op::NamedCall)
                && let Some(body) = self.body_of_call(node)
            {
                let caller = self.inst_key(node)?;
                let caller = self.typed_as.get(&caller).copied().unwrap_or(caller);
                found.fed.insert((caller, node, family));
                let arguments = self.arguments(node)?;
                for position in summaries
                    .get(&(body, family))
                    .map(Vec::as_slice)
                    .unwrap_or_default()
                {
                    if let Some(argument) = arguments.get(*position as usize).copied() {
                        stack.push((argument, env, inst));
                    }
                }
                continue;
            }
            if op == Op::For
                && let Some(body) = self.body_of_call(node)
            {
                let caller = self.inst_key(node)?;
                let caller = self.typed_as.get(&caller).copied().unwrap_or(caller);
                let typed = self.typed_as.get(&body).copied().unwrap_or(body);
                found.folds.insert((caller, typed));
                stack.push((self.body_value(body)?, env, body));
            }
            for next in self.carried(node)? {
                stack.push((next, env, inst));
            }
        }
        Ok(())
    }

    pub(super) fn callee_first_bodies(&self) -> Vec<u32> {
        let mut nested: std::collections::HashMap<u32, Vec<u32>> = std::collections::HashMap::new();
        let mut bodies: Vec<u32> = Vec::new();
        for ((caller, _), body) in &self.call_body {
            if self.function_inst(*body) {
                nested.entry(*caller).or_default().push(*body);
                bodies.push(*body);
            }
        }
        bodies.sort_unstable();
        bodies.dedup();
        for list in nested.values_mut() {
            list.sort_unstable();
            list.dedup();
        }
        let mut order = Vec::new();
        let mut done = std::collections::HashSet::new();
        for root in bodies {
            let mut stack = vec![(root, false)];
            while let Some((body, expanded)) = stack.pop() {
                if expanded {
                    order.push(body);
                    continue;
                }
                if !done.insert(body) {
                    continue;
                }
                stack.push((body, true));
                let key = self.typed_as.get(&body).copied().unwrap_or(body);
                for inner in nested.get(&key).map(Vec::as_slice).unwrap_or_default() {
                    if !done.contains(inner) {
                        stack.push((*inner, false));
                    }
                }
            }
        }
        order
    }

    fn units_by_family(&self) -> Result<Vec<(FamilyId, bool, bool)>, Internal> {
        let mut units: Vec<(FamilyId, usize)> = self
            .unit_feeds
            .iter()
            .map(|(_, family, unit, _)| (*family, *unit))
            .collect();
        units.sort_unstable_by_key(|(family, unit)| (family.index(), *unit));
        units.dedup();
        let mut by_family: Vec<(FamilyId, bool, bool)> = Vec::new();
        for (family, unit) in units {
            let scaled = self.unit_scale(family, unit)? != 1.0;
            match by_family.last_mut() {
                Some((last, many, any)) if *last == family => {
                    *many = true;
                    *any |= scaled;
                }
                Some(_) | None => by_family.push((family, false, scaled)),
            }
        }
        Ok(by_family)
    }

    pub fn plain_in_family(&mut self) -> Stage<()> {
        if self.scaled_accumulator {
            return Ok(Err(Declined(Reason::UnitAttribute)));
        }
        let by_family = self.units_by_family()?;
        let mixed: Vec<FamilyId> = by_family
            .iter()
            .filter(|(_, many, _)| *many)
            .map(|(family, _, _)| *family)
            .collect();
        if self.takers.is_empty() && mixed.is_empty() {
            return Ok(Ok(()));
        }
        let outer = self.inst;
        let feeds = std::mem::take(&mut self.unit_feeds);
        let mut families: Vec<FamilyId> = feeds.iter().map(|(_, family, _, _)| *family).collect();
        families.sort_unstable_by_key(|family| family.index());
        families.dedup();
        let mut found = FeedFound::default();
        let mut summaries: std::collections::HashMap<(u32, FamilyId), Vec<u32>> =
            std::collections::HashMap::new();
        let order = self.callee_first_bodies();
        for family in &families {
            for body in &order {
                let mut params = Vec::new();
                let start = (self.body_value(*body)?, NONE, *body);
                self.feed_walk(start, *family, &summaries, &mut found, &mut params)?;
                params.sort_unstable();
                params.dedup();
                summaries.insert((*body, *family), params);
            }
        }
        let mut top = FeedFound::default();
        let mut taker_units: std::collections::HashMap<(u32, u32), Vec<(usize, u32)>> =
            std::collections::HashMap::new();
        let mut cell_units: std::collections::BTreeMap<u32, Vec<(usize, u32)>> =
            std::collections::BTreeMap::new();
        for (root, family, unit, env) in feeds {
            let inst = self
                .envs
                .get(env as usize)
                .map_or(NONE, |record| record.inst);
            let mut params = Vec::new();
            self.feed_walk((root, env, inst), family, &summaries, &mut top, &mut params)?;
            if !mixed.contains(&family) {
                continue;
            }
            self.inst = inst;
            let held = self.fact(root)?;
            if open(&held) || !numeric(&held) {
                continue;
            }
            let mut own = FeedFound::default();
            self.feed_walk((root, env, inst), family, &summaries, &mut own, &mut params)?;
            for (node, _, key) in own.reached {
                taker_units
                    .entry((node, key))
                    .or_default()
                    .push((unit, root));
                let fact = self.fact(node)?;
                if key == NONE
                    && self.ir.node(node)?.0 == Op::Name
                    && let DeclRef::Cell(cell) = self.bound(node)?
                    && self.family(node).is_none()
                    && numeric(&fact)
                    && !open(&fact)
                {
                    cell_units.entry(cell).or_default().push((unit, root));
                }
            }
        }
        let mut conflicted = std::collections::HashSet::new();
        let units_of = |list: &mut Vec<(usize, u32)>| -> Result<Vec<(usize, u32)>, Internal> {
            let mut keyed = Vec::with_capacity(list.len());
            for (unit, root) in list.iter() {
                keyed.push((*unit, self.call_order(*root)?, *root));
            }
            keyed.sort_unstable();
            keyed.dedup_by_key(|(unit, _, _)| *unit);
            Ok(keyed
                .into_iter()
                .map(|(unit, _, root)| (unit, root))
                .collect())
        };
        let mut conflicts = Vec::new();
        let mut cell_conflicts = Vec::new();
        for (node, family, key) in &self.takers {
            if let Some(list) = taker_units.get_mut(&(*node, *key))
                && mixed.contains(family)
                && let [(_, first), (_, second), ..] = units_of(list)?.as_slice()
            {
                conflicts.push((*node, *first, *second));
                if let DeclRef::Cell(cell) = self.bound(*node)? {
                    conflicted.insert(cell);
                }
            }
        }
        for (cell, list) in &mut cell_units {
            if !conflicted.contains(cell)
                && let [(_, first), (_, second), ..] = units_of(list)?.as_slice()
            {
                cell_conflicts.push((*cell, *first, *second));
            }
        }
        conflicts.sort_unstable();
        conflicts.dedup();
        for (node, first, second) in &conflicts {
            let span = self.node_span(*node)?;
            let related = vec![
                (self.node_span(*first)?, "one attribute".to_string()),
                (self.node_span(*second)?, "the other attribute".to_string()),
            ];
            self.draw_related(
                Code::ConflictingUnits,
                span,
                related,
                "this plain value feeds two attributes of one unit family with different default units",
            );
        }
        for conflict in std::mem::take(&mut self.unit_conflicts) {
            let span = self.node_span(conflict.node)?;
            let mut related = vec![
                (self.node_span(conflict.first)?, "one attribute".to_string()),
                (
                    self.node_span(conflict.second)?,
                    "the other attribute".to_string(),
                ),
            ];
            for call in &conflict.chain {
                related.push((self.node_span(*call)?, "in this call".to_string()));
            }
            self.draw_related(
                Code::ConflictingUnits,
                span,
                related,
                "this plain value feeds two attributes of one unit family with different default units",
            );
        }
        for (cell, first, second) in cell_conflicts {
            let (name, _) = self.cell_name(cell)?;
            let span = self.node_span(first)?;
            let related = vec![(self.node_span(second)?, "the other attribute".to_string())];
            self.draw_related(
                Code::ConflictingUnits,
                span,
                related,
                format!(
                    "the plain cell `{name}` is read at two attributes of one unit family with different default units"
                ),
            );
        }
        let conflicting: std::collections::HashSet<u32> =
            conflicts.iter().map(|(node, _, _)| *node).collect();
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
            for list in sites.values_mut() {
                list.sort_unstable();
                list.dedup();
            }
        }
        let mut fed_memo: std::collections::HashMap<(u32, FamilyId), bool> =
            std::collections::HashMap::new();
        for (node, family, key) in takers {
            if held.contains(&(node, key)) || conflicting.contains(&node) {
                continue;
            }
            let name = self.family_name(family)?;
            let summary = format!(
                "this plain value takes the unit family `{name}`, and its family value reaches no attribute of that family to give its unit"
            );
            let chains = if key == NONE || !self.function_inst(key) {
                if top.reached.contains(&(node, family, key))
                    || found.reached.contains(&(node, family, key))
                {
                    Vec::new()
                } else {
                    vec![Vec::new()]
                }
            } else {
                let inside = found.reached.contains(&(node, family, key));
                if inside && self.fully_fed(key, family, &sites, &found, &top, &mut fed_memo) {
                    Vec::new()
                } else {
                    self.unreached_chains(key, family, inside, &sites, &found, &top)?
                }
            };
            for chain in chains {
                if chain.last().is_some_and(|outermost| {
                    self.unit_conflicted.contains(&(node, key, *outermost))
                }) {
                    continue;
                }
                let span = self.node_span(node)?;
                let mut related = Vec::new();
                for call in &chain {
                    related.push((self.node_span(*call)?, "in this call".to_string()));
                }
                self.draw_related(Code::PlainInFamily, span, related, summary.clone());
            }
        }
        Ok(Ok(()))
    }

    pub(super) fn call_order(&self, call: u32) -> Result<(u32, u32, u32), Internal> {
        let (lo, hi) = self.ir.span(call)?;
        Ok((self.file_of(call), lo, hi))
    }

    fn inner_first(
        &self,
        best: &std::collections::HashMap<(u32, bool), Link>,
        from: (u32, bool),
    ) -> Vec<u32> {
        let mut calls = Vec::new();
        let mut at = from;
        while let Some(link) = best.get(&at) {
            calls.push(link.call);
            at = (link.callee, link.unfed_below);
        }
        calls.reverse();
        calls
    }

    fn better(
        &self,
        best: &std::collections::HashMap<(u32, bool), Link>,
        candidate: (usize, u32, (u32, bool)),
        current: Option<&Link>,
    ) -> Result<bool, Internal> {
        let Some(current) = current else {
            return Ok(true);
        };
        let (length, call, below) = candidate;
        if length != current.length {
            return Ok(length > current.length);
        }
        let mut mine = self.inner_first(best, below);
        mine.push(call);
        let mut theirs = self.inner_first(best, (current.callee, current.unfed_below));
        theirs.push(current.call);
        for (left, right) in mine.iter().zip(&theirs) {
            let (left, right) = (self.call_order(*left)?, self.call_order(*right)?);
            if left != right {
                return Ok(left < right);
            }
        }
        Ok(false)
    }

    fn unreached_chains(
        &self,
        key: u32,
        family: FamilyId,
        inside: bool,
        sites: &std::collections::HashMap<u32, Vec<(u32, u32)>>,
        found: &FeedFound,
        top: &FeedFound,
    ) -> Result<Vec<Vec<u32>>, Internal> {
        let mut reaching = vec![key];
        let mut seen = std::collections::HashSet::from([key]);
        let mut cursor = 0;
        while let Some(at) = reaching.get(cursor).copied() {
            cursor += 1;
            for (caller, _) in sites.get(&at).map(Vec::as_slice).unwrap_or_default() {
                if self.function_inst(*caller) && seen.insert(*caller) {
                    reaching.push(*caller);
                }
            }
        }
        let mut waiting: std::collections::HashMap<u32, usize> = std::collections::HashMap::new();
        for at in &reaching {
            for (caller, _) in sites.get(at).map(Vec::as_slice).unwrap_or_default() {
                if seen.contains(caller) {
                    *waiting.entry(*caller).or_default() += 1;
                }
            }
        }
        let mut best: std::collections::HashMap<(u32, bool), Link> =
            std::collections::HashMap::new();
        let mut outermost: std::collections::BTreeMap<(u32, u32), Link> =
            std::collections::BTreeMap::new();
        let mut ready = vec![key];
        while let Some(callee) = ready.pop() {
            let depths: Vec<Option<usize>> = [false, true]
                .into_iter()
                .map(|mode| {
                    if callee == key {
                        (!mode || !inside).then_some(0)
                    } else {
                        best.get(&(callee, mode)).map(|link| link.length)
                    }
                })
                .collect();
            for (caller, call) in sites.get(&callee).map(Vec::as_slice).unwrap_or_default() {
                CountedWalk::CoreFamilies.step();
                let function = self.function_inst(*caller);
                let unfed_here = !found.fed.contains(&(*caller, *call, family))
                    && !top.fed.contains(&(*caller, *call, family));
                for mode in [false, true] {
                    if !function && !mode {
                        continue;
                    }
                    let below_mode = mode && !unfed_here;
                    let Some(length) = depths.get(usize::from(below_mode)).copied().flatten()
                    else {
                        continue;
                    };
                    let link = Link {
                        length: length + 1,
                        call: *call,
                        callee,
                        unfed_below: below_mode,
                    };
                    let candidate = (length + 1, *call, (callee, below_mode));
                    if function {
                        if self.better(&best, candidate, best.get(&(*caller, mode)))? {
                            best.insert((*caller, mode), link);
                        }
                    } else {
                        let slot = (*caller, *call);
                        if self.better(&best, candidate, outermost.get(&slot))? {
                            outermost.insert(slot, link);
                        }
                    }
                }
                if function && let Some(count) = waiting.get_mut(caller) {
                    *count = count.saturating_sub(1);
                    if *count == 0 {
                        ready.push(*caller);
                    }
                }
            }
        }
        let mut chains = Vec::new();
        for link in outermost.values() {
            let mut chain = self.inner_first(&best, (link.callee, link.unfed_below));
            chain.push(link.call);
            chains.push(chain);
        }
        Ok(chains)
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

impl Core<'_> {
    pub fn default_readable(&mut self, root: u32, fractional: bool) -> Stage<bool> {
        if let Err(declined) = self.ensure_typed(root)? {
            return Ok(Err(declined));
        }
        if fractional {
            return Ok(Ok(false));
        }
        for node in self.postorder(root)? {
            let readable = match self.ir.node(node)?.0 {
                Op::Name => matches!(
                    self.bound(node)?,
                    DeclRef::Cell(_) | DeclRef::Let(_) | DeclRef::Param(_)
                ),
                Op::Int
                | Op::Float
                | Op::Bool
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
                | Op::If
                | Op::Array
                | Op::Index
                | Op::IndexFromEnd
                | Op::Convert
                | Op::Call
                | Op::NamedCall
                | Op::Past
                | Op::Slice
                | Op::For => true,
                Op::Str
                | Op::Variant
                | Op::To
                | Op::Field
                | Op::MethodCall
                | Op::NamedMethodCall
                | Op::Stepped
                | Op::Is
                | Op::IsNot
                | Op::Match
                | Op::TyScalar
                | Op::TyName
                | Op::TyArray
                | Op::TyVar
                | Op::Bad => false,
            };
            if !readable {
                return Ok(Ok(false));
            }
        }
        Ok(Ok(true))
    }
}

impl Core<'_> {
    pub fn in_default_unit(
        &mut self,
        node: u32,
        value: super::core_value::CoreValue,
        family: FamilyId,
        unit: usize,
    ) -> Result<super::core_value::CoreValue, Internal> {
        use crate::semantics::operators::{Value, ty_name};
        let scale = self.unit_scale(family, unit)?;
        let Some((root, _, _, ty)) = self.default_scale else {
            return Ok(value);
        };
        let scaled = match value {
            Value::Int(held) if scale.fract() == 0.0 && scale.abs() < 1e38 => {
                match held.checked_mul(scale as i128) {
                    Some(scaled) if scaled != i128::MIN => Value::Int(scaled),
                    Some(_) | None => {
                        self.draw_at(
                            Code::IntegerOverflow,
                            node,
                            "this value's magnitude reaches 2¹²⁷",
                        )?;
                        return Ok(Value::Error);
                    }
                }
            }
            Value::F32(held) => Value::F32((f64::from(held) * scale) as f32),
            Value::F64(held) => Value::F64(held * scale),
            other => other,
        };
        let fits = match &scaled {
            Value::F32(held) => held.is_finite(),
            Value::F64(held) => held.is_finite(),
            _ => true,
        };
        let literal = matches!(self.ir.node(node)?.0, Op::Int | Op::Float);
        if node != root && literal && !fits {
            self.draw_at(
                Code::DoesNotFit,
                node,
                format!("this value does not fit `{}`", ty_name(ty)),
            )?;
            return Ok(Value::Error);
        }
        Ok(scaled)
    }
}

impl Core<'_> {
    pub fn fixed_root_outside(
        &self,
        node: u32,
        value: &super::core_value::CoreValue,
    ) -> Result<bool, Internal> {
        let Some((root, _, _, _)) = self.default_scale else {
            return Ok(false);
        };
        if node != root {
            return Ok(false);
        }
        let range = self
            .fact(node)?
            .fixed
            .and_then(crate::semantics::operators::int_range);
        Ok(match (value, range) {
            (crate::semantics::operators::Value::Int(held), Some(range)) => !range.contains(held),
            _ => false,
        })
    }
}

impl Core<'_> {
    pub fn accumulator_read(&self, node: u32) -> Result<bool, Internal> {
        if self.ir.node(node)?.0 != Op::Name {
            return Ok(false);
        }
        let DeclRef::Param(position) = self.bound(node)? else {
            return Ok(false);
        };
        let owner = self.binding_owner(node)?;
        if !self.is_fold(owner) {
            return Ok(false);
        }
        let rec = self
            .ir
            .fors
            .get((owner - self.functions()) as usize)
            .ok_or(Internal::missed("IR for"))?;
        Ok(position == rec.names.1 - rec.names.0)
    }
}

impl Core<'_> {
    fn fully_fed(
        &self,
        body: u32,
        family: FamilyId,
        sites: &std::collections::HashMap<u32, Vec<(u32, u32)>>,
        found: &FeedFound,
        top: &FeedFound,
        memo: &mut std::collections::HashMap<(u32, FamilyId), bool>,
    ) -> bool {
        let mut stack = vec![(body, false)];
        while let Some((at, expanded)) = stack.pop() {
            if memo.contains_key(&(at, family)) {
                continue;
            }
            let list = sites.get(&at).map(Vec::as_slice).unwrap_or_default();
            if !expanded {
                stack.push((at, true));
                for (caller, _) in list {
                    if self.function_inst(*caller) && !memo.contains_key(&(*caller, family)) {
                        stack.push((*caller, false));
                    }
                }
                continue;
            }
            CountedWalk::CoreFamilies.step();
            let fed = !list.is_empty()
                && list.iter().all(|(caller, call)| {
                    let here = found.fed.contains(&(*caller, *call, family))
                        || top.fed.contains(&(*caller, *call, family));
                    here && (!self.function_inst(*caller)
                        || memo.get(&(*caller, family)).copied().unwrap_or(false))
                });
            memo.insert((at, family), fed);
        }
        memo.get(&(body, family)).copied().unwrap_or(false)
    }
}
