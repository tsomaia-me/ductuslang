use std::collections::{BTreeMap, HashSet, VecDeque};

use symphonyscript_core::FamilyId;

use super::core::{Core, Stage};
use super::families::FeedFound;
use crate::semantics::Internal;
use crate::semantics::counted_walk::CountedWalk;
use crate::semantics::ir::flat::NONE;

pub(crate) struct UnitConflict {
    pub node: u32,
    pub first: u32,
    pub second: u32,
    pub chain: Vec<u32>,
}

pub(crate) type AttributeFeed = (u32, FamilyId, usize, u32, u32);

impl Core<'_> {
    pub fn scale_conflicted(&self, node: u32) -> Result<bool, Internal> {
        if self.unit_conflicted.is_empty() {
            return Ok(false);
        }
        let key = self.in_body(node)?.map_or(NONE, |(inst, _)| inst);
        let outermost = self.eval_chain.first().copied().unwrap_or(NONE);
        Ok(self.unit_conflicted.contains(&(node, key, NONE))
            || (key != NONE && self.unit_conflicted.contains(&(node, key, outermost))))
    }

    pub fn first_two_units(&self, list: &[(usize, u32)]) -> Result<Option<(u32, u32)>, Internal> {
        let mut keyed = Vec::with_capacity(list.len());
        for (unit, root) in list {
            keyed.push((*unit, self.call_order(*root)?, *root));
        }
        keyed.sort_unstable();
        keyed.dedup_by_key(|(unit, _, _)| *unit);
        Ok(match keyed.as_slice() {
            [(_, _, first), (_, _, second), ..] => Some((*first, *second)),
            _ => None,
        })
    }

    pub fn find_unit_conflicts(&mut self, feeds: Vec<AttributeFeed>) -> Stage<()> {
        let mut units: BTreeMap<FamilyId, HashSet<usize>> = BTreeMap::new();
        for (_, family, unit, _, _) in &feeds {
            units.entry(*family).or_default().insert(*unit);
        }
        let mixed: Vec<FamilyId> = units
            .into_iter()
            .filter(|(_, units)| units.len() > 1)
            .map(|(family, _)| family)
            .collect();
        if mixed.is_empty() {
            return Ok(Ok(()));
        }
        let feeds: Vec<AttributeFeed> = feeds
            .into_iter()
            .filter(|(_, family, _, _, _)| mixed.contains(family))
            .collect();
        for (root, _, _, env, wire) in &feeds {
            let saved = self.enter_env(*env);
            self.wire_now = *wire;
            let typed = self.ensure_typed(*root);
            self.wire_now = NONE;
            self.leave_env(saved);
            if let Err(declined) = typed? {
                return Ok(Err(declined));
            }
        }
        let outer = self.inst;
        for family in mixed {
            self.family_conflicts(family, &feeds)?;
        }
        self.inst = outer;
        Ok(Ok(()))
    }

    fn family_conflicts(
        &mut self,
        family: FamilyId,
        feeds: &[AttributeFeed],
    ) -> Result<(), Internal> {
        let mut found = FeedFound::default();
        let mut summaries: super::column::PairColumn<Vec<u32>> = Default::default();
        for body in self.callee_first_bodies() {
            let mut params = Vec::new();
            let start = (self.body_value(body)?, NONE, body);
            self.feed_walk(start, family, &summaries, &mut found, &mut params)?;
            params.sort_unstable();
            params.dedup();
            summaries.insert((body, super::column::dense(family.index())), params);
        }
        let mut taker_units: super::column::PairColumn<Vec<(usize, u32)>> = Default::default();
        let mut outer_units: BTreeMap<(u32, u32), Vec<(usize, u32)>> = BTreeMap::new();
        for (root, fed, unit, env, wire) in feeds {
            if *fed != family {
                continue;
            }
            let inst = self
                .envs
                .get(*env as usize)
                .map_or(NONE, |record| record.inst);
            let mut own = FeedFound::default();
            let mut params = Vec::new();
            self.wire_now = *wire;
            let walked = self.feed_walk(
                (*root, *env, inst),
                family,
                &summaries,
                &mut own,
                &mut params,
            );
            self.wire_now = NONE;
            walked?;
            for (node, _, key) in own.reached.iter() {
                taker_units
                    .entry((node, key))
                    .or_default()
                    .push((*unit, *root));
            }
            for (caller, call, _) in own.fed.iter() {
                if !self.function_inst(caller) {
                    outer_units
                        .entry((caller, call))
                        .or_default()
                        .push((*unit, *root));
                }
            }
        }
        let mut by_key: super::column::Column<Vec<u32>> = Default::default();
        let takers: Vec<(u32, FamilyId, u32)> = self.takers.clone();
        for (node, taken, key) in takers {
            if taken != family {
                continue;
            }
            if key != NONE {
                by_key.entry(key).or_default().push(node);
            }
            if key != NONE && self.function_inst(key) {
                continue;
            }
            if let Some(list) = taker_units.get(&(node, key))
                && self.first_two_units(list)?.is_some()
            {
                self.unit_conflicted.insert((node, key, NONE));
            }
        }
        if by_key.is_empty() {
            return Ok(());
        }
        let typed = |inst: u32, typed_as: &super::column::Column<u32>| {
            typed_as.get(&inst).copied().unwrap_or(inst)
        };
        let mut down: super::column::PairColumn<u32> = Default::default();
        for ((caller, call), body) in self.call_body.iter() {
            down.insert(
                (typed(caller, &self.typed_as), call),
                typed(*body, &self.typed_as),
            );
        }
        let mut fed_in: super::column::Column<Vec<(u32, u32, u32, u32)>> = Default::default();
        for (caller, call, fed) in found.fed.iter() {
            if fed == super::column::dense(family.index()) {
                let order = self.call_order(call)?;
                fed_in
                    .entry(caller)
                    .or_default()
                    .push((order.0, order.1, order.2, call));
            }
        }
        for list in fed_in.values_mut() {
            list.sort_unstable();
            list.dedup();
        }
        let mut folds_in: super::column::Column<Vec<u32>> = Default::default();
        for (caller, body) in found.folds.iter() {
            folds_in.entry(caller).or_default().push(body);
        }
        for list in folds_in.values_mut() {
            list.sort_unstable();
        }
        for ((caller, outermost), list) in outer_units {
            let Some((first, second)) = self.first_two_units(&list)? else {
                continue;
            };
            let Some(start) = down.get(&(caller, outermost)).copied() else {
                continue;
            };
            let mut seen = HashSet::from([start]);
            let mut queue = VecDeque::from([(start, vec![outermost])]);
            while let Some((body, chain)) = queue.pop_front() {
                CountedWalk::CoreFamilies.step();
                for node in by_key.get(&body).map(Vec::as_slice).unwrap_or_default() {
                    if found
                        .reached
                        .contains(&(*node, super::column::dense(family.index()), body))
                        && self.unit_conflicted.insert((*node, body, outermost))
                    {
                        self.unit_conflicts.push(UnitConflict {
                            node: *node,
                            first,
                            second,
                            chain: chain.clone(),
                        });
                    }
                }
                for inner in folds_in.get(&body).map(Vec::as_slice).unwrap_or_default() {
                    if seen.insert(*inner) {
                        queue.push_back((*inner, chain.clone()));
                    }
                }
                for (_, _, _, call) in fed_in.get(&body).map(Vec::as_slice).unwrap_or_default() {
                    if let Some(inner) = down.get(&(body, *call)).copied()
                        && seen.insert(inner)
                    {
                        let mut longer = vec![*call];
                        longer.extend(chain.iter().copied());
                        queue.push_back((inner, longer));
                    }
                }
            }
        }
        Ok(())
    }
}
