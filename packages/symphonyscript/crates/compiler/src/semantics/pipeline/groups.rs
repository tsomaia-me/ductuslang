use std::collections::{HashMap, HashSet};

use super::core::{Core, Stage};
use super::cycles::strongly_connected;
use super::decl_ref::DeclRef;
use super::declined::Declined;
use super::fact::Fact;
use super::folds::ANY_LENGTH;
use super::leaf::Leaf;
use super::reason::Reason;
use crate::diagnostics::{Code, Diagnostic};
use crate::semantics::Internal;
use crate::semantics::ir::CellWord;
use crate::semantics::ir::flat::{NONE, Op};

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Source {
    Let(u32),
    Fn(u32),
    Root(u32),
}

#[derive(Clone, Default)]
struct Direct {
    cells: Vec<u32>,
    history: Vec<(u32, u32)>,
    sources: Vec<Source>,
}

#[derive(Clone, Default, Debug)]
pub(crate) struct CellReads {
    pub present: Vec<u32>,
    pub history: Vec<(u32, u32)>,
}

const SETTLE_ROUNDS: usize = 64;

impl Core<'_> {
    fn direct_reads(&self, roots: &[u32]) -> Result<Direct, Internal> {
        let mut found = Direct::default();
        for root in roots {
            for node in self.deep_postorder(*root)? {
                let (op, a, _, _) = self.ir.node(node)?;
                if matches!(op, Op::Call | Op::NamedCall)
                    && let Some(function) = self.callee(node)?
                {
                    found.sources.push(Source::Fn(function));
                }
                if op == Op::Field
                    && let Some(root) = self.read_root(node)?
                {
                    found.sources.push(Source::Root(root));
                    continue;
                }
                if matches!(op, Op::Past | Op::Stepped) {
                    if let DeclRef::Cell(other) = self.bound(a)? {
                        found.history.push((other, node));
                    }
                    continue;
                }
                if op != Op::Name || self.is_history_target(node) {
                    continue;
                }
                match self.bound(node)? {
                    DeclRef::Cell(other) => found.cells.push(other),
                    DeclRef::Let(other) => found.sources.push(Source::Let(other)),
                    DeclRef::None
                    | DeclRef::Instance(_)
                    | DeclRef::Duplicate
                    | DeclRef::Param(_)
                    | DeclRef::Fn(_)
                    | DeclRef::Pattern(_) => {}
                }
            }
        }
        Ok(found)
    }

    fn cells_read(
        &self,
        root: u32,
        memo: &mut HashMap<Source, Direct>,
    ) -> Result<CellReads, Internal> {
        let direct = self.direct_reads(&[root])?;
        let mut reads = CellReads {
            present: direct.cells,
            history: direct.history,
        };
        let mut pending = direct.sources;
        let mut seen = HashSet::new();
        while let Some(source) = pending.pop() {
            if !seen.insert(source) {
                continue;
            }
            let found = match memo.entry(source) {
                std::collections::hash_map::Entry::Occupied(entry) => entry.into_mut(),
                std::collections::hash_map::Entry::Vacant(entry) => {
                    let roots = match source {
                        Source::Let(at) => vec![self.let_root(at)?],
                        Source::Fn(at) => self.function_roots(at)?,
                        Source::Root(root) => vec![root],
                    };
                    entry.insert(self.direct_reads(&roots)?)
                }
            };
            reads.present.extend(found.cells.iter().copied());
            reads.history.extend(found.history.iter().copied());
            pending.extend(found.sources.iter().copied());
        }
        Ok(reads)
    }

    fn cell_order(&mut self) -> Stage<Vec<u32>> {
        let count = u32::try_from(self.ir.cells.len()).unwrap_or(0);
        let mut all = Vec::with_capacity(count as usize);
        let mut memo = HashMap::new();
        for at in 0..count {
            all.push(if !self.is_cyclic_cell(at) {
                let mut reads = self.cells_read(self.cell_root(at)?, &mut memo)?;
                reads.present.retain(|other| !self.is_cyclic_cell(*other));
                reads
                    .history
                    .retain(|(other, _)| !self.is_cyclic_cell(*other));
                reads
            } else {
                CellReads::default()
            });
        }
        let edges: Vec<Vec<u32>> = all
            .iter()
            .map(|reads| {
                let mut edges = reads.present.clone();
                edges.extend(reads.history.iter().map(|(target, _)| *target));
                edges
            })
            .collect();
        let mut order = Vec::with_capacity(count as usize);
        let mut group_of = vec![NONE; count as usize];
        let mut groups = Vec::new();
        for component in strongly_connected(&edges) {
            let single = component.len() == 1;
            let first = component.first().copied().unwrap_or(NONE);
            let own = all
                .get(first as usize)
                .is_some_and(|reads| reads.history.iter().any(|(target, _)| *target == first));
            if single && !own {
                if all
                    .get(first as usize)
                    .is_some_and(|reads| reads.present.contains(&first))
                {
                    return Ok(Err(Declined(Reason::CellGroup)));
                }
                order.push(first);
                continue;
            }
            let Some(members) = present_order(&component, &all) else {
                return Ok(Err(Declined(Reason::CellGroup)));
            };
            let group = u32::try_from(groups.len()).unwrap_or(NONE);
            for member in &members {
                if let Some(slot) = group_of.get_mut(*member as usize) {
                    *slot = group;
                }
            }
            order.extend(members.iter().copied());
            groups.push(members);
        }
        for (group, members) in groups.iter().enumerate() {
            for member in members {
                let root = self.cell_root(*member)?;
                let rule: HashSet<u32> = self.postorder(root)?.into_iter().collect();
                let reads = all.get(*member as usize).cloned().unwrap_or_default();
                for (target, node) in reads.history {
                    if group_of.get(target as usize).copied() != u32::try_from(group).ok() {
                        continue;
                    }
                    if !rule.contains(&node) {
                        return Ok(Err(Declined(Reason::CellGroup)));
                    }
                    self.group_reads.insert(node, (target, *member));
                    self.group_history.entry(target).or_default().push(node);
                }
            }
        }
        self.cell_reads = all;
        self.cell_group = group_of;
        self.groups = groups;
        Ok(Ok(order))
    }

    pub fn group_read(&self, node: u32) -> Result<Option<Fact>, Internal> {
        let Some((target, reader)) = self.group_reads.get(&node).copied() else {
            return Ok(None);
        };
        let settled = match self.group_kinds.get(&target) {
            Some(fact) => match self.open_member(target, fact) {
                Some(init) => Some(self.fact(init)?),
                None => Some(fact.clone()),
            },
            None => None,
        }
        .filter(|fact| fact.leaf != Leaf::Empty);
        let seed = self.group_seed(node, target)?;
        let mut found = match (seed, settled) {
            (Some(seed), Some(settled))
                if matches!(settled.leaf, Leaf::Placeholder | Leaf::Error) =>
            {
                seed
            }
            (Some(seed), Some(settled)) if target != reader => {
                match super::typing::unify(seed.leaf, settled.leaf) {
                    Some(leaf) if leaf == seed.leaf && leaf != settled.leaf => seed,
                    Some(_) | None => settled,
                }
            }
            (Some(seed), _) => seed,
            (None, Some(settled)) => settled,
            (None, None) => Fact::scalar(Leaf::Empty),
        };
        if self.group_variable.contains(&target)
            && let Some(first) = found.dims.first_mut()
        {
            *first = ANY_LENGTH;
        }
        Ok(Some(found))
    }

    fn open_reads(&self, members: &[u32]) -> Result<bool, Internal> {
        for member in members {
            for node in self.group_history.get(member).into_iter().flatten() {
                if self.group_seed(*node, *member)?.is_none() {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    pub fn open_member(&self, at: u32, fact: &Fact) -> Option<u32> {
        if !self.group_open.contains(&at) || !open_kind(fact) {
            return None;
        }
        self.ir
            .cells
            .get(at as usize)
            .map(|rec| rec.init)
            .filter(|init| *init != NONE)
    }

    fn seedless(&self, member: u32) -> Result<bool, Internal> {
        for node in self.group_history.get(&member).into_iter().flatten() {
            if self.group_seed(*node, member)?.is_none() {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn holds_variable(&self, member: u32, fact: &Fact) -> Result<bool, Internal> {
        let Some(length) = fact.dims.first().copied() else {
            return Ok(false);
        };
        if matches!(fact.leaf, Leaf::Placeholder | Leaf::Error) {
            return Ok(false);
        }
        if length == ANY_LENGTH {
            return Ok(true);
        }
        for node in self.group_history.get(&member).into_iter().flatten() {
            if let Some(seed) = self.group_seed(*node, member)?
                && seed.dims.first() != Some(&length)
            {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn group_seed(&self, node: u32, target: u32) -> Result<Option<Fact>, Internal> {
        let (_, _, _, fallback) = self.ir.node(node)?;
        if fallback != NONE && self.has_fact(fallback)? {
            let held = self.fact(fallback)?;
            if !matches!(held.leaf, Leaf::Placeholder | Leaf::Error) {
                return Ok(Some(held));
            }
        }
        let init = self
            .ir
            .cells
            .get(target as usize)
            .map_or(NONE, |rec| rec.init);
        if init != NONE && self.has_fact(init)? {
            let held = self.fact(init)?;
            if !self.is_reactive(init)? && !matches!(held.leaf, Leaf::Placeholder | Leaf::Error) {
                return Ok(Some(held));
            }
        }
        Ok(None)
    }

    pub fn is_own_history(&self, node: u32) -> bool {
        self.group_reads
            .get(&node)
            .is_some_and(|(target, reader)| target == reader)
    }

    fn clear_rule(&mut self, root: u32) -> Result<(), Internal> {
        for node in self.postorder(root)? {
            let local = self.local(node)?;
            if let Some(slot) = self.facts.get_mut(local) {
                *slot = None;
            }
            if let Some(slot) = self.kept.get_mut(local) {
                *slot = false;
            }
            if let Some(slot) = self.reactive.get_mut(local) {
                *slot = false;
            }
        }
        Ok(())
    }

    fn group_readers(&self, members: &[u32]) -> HashMap<u32, Vec<usize>> {
        let inside: HashSet<u32> = members.iter().copied().collect();
        let mut readers: HashMap<u32, Vec<usize>> = HashMap::new();
        for (position, member) in members.iter().enumerate() {
            let Some(reads) = self.cell_reads.get(*member as usize) else {
                continue;
            };
            let targets = reads
                .present
                .iter()
                .copied()
                .chain(reads.history.iter().map(|(target, _)| *target));
            for target in targets {
                if inside.contains(&target) {
                    let list = readers.entry(target).or_default();
                    if !list.contains(&position) {
                        list.push(position);
                    }
                }
            }
        }
        readers
    }

    fn type_member(&mut self, member: u32) -> Stage<bool> {
        let root = self.cell_root(member)?;
        self.clear_rule(root)?;
        let fact = match self.ensure_typed(root)? {
            Ok(fact) => fact,
            Err(declined) => return Ok(Err(declined)),
        };
        let resized = self
            .group_kinds
            .get(&member)
            .is_some_and(|before| resized(before, &fact))
            && self.seedless(member)?;
        let variable =
            (resized || self.holds_variable(member, &fact)?) && self.group_variable.insert(member);
        let changed = variable || self.group_kinds.get(&member) != Some(&fact);
        self.group_kinds.insert(member, fact);
        Ok(Ok(changed))
    }

    fn settle_group(&mut self, members: &[u32]) -> Stage<()> {
        for member in members {
            let init = self
                .ir
                .cells
                .get(*member as usize)
                .map_or(NONE, |rec| rec.init);
            if init != NONE
                && let Err(declined) = self.ensure_typed(init)?
            {
                return Ok(Err(declined));
            }
        }
        let readers = self.group_readers(members);
        let mark = self.diagnostics.len();
        let top = self.top_drawn.len();
        let warned = self.warned.clone();
        let mut queue: std::collections::BTreeSet<usize> = (0..members.len()).collect();
        let mut budget = members.len().saturating_mul(SETTLE_ROUNDS);
        loop {
            while let Some(position) = queue.pop_first() {
                if budget == 0 {
                    return Ok(Err(Declined(Reason::CellGroup)));
                }
                budget -= 1;
                let member = members.get(position).copied().unwrap_or(NONE);
                match self.type_member(member)? {
                    Ok(true) => queue.extend(readers.get(&member).into_iter().flatten().copied()),
                    Ok(false) => {}
                    Err(declined) => return Ok(Err(declined)),
                }
            }
            if !self.open_reads(members)? {
                break;
            }
            for (position, member) in members.iter().enumerate() {
                let init = self
                    .ir
                    .cells
                    .get(*member as usize)
                    .map_or(NONE, |rec| rec.init);
                let open = self.group_kinds.get(member).is_some_and(open_kind);
                if open
                    && init != NONE
                    && !self.is_reactive(init)?
                    && self.group_open.insert(*member)
                {
                    queue.insert(position);
                    queue.extend(readers.get(member).into_iter().flatten().copied());
                }
            }
            if queue.is_empty() {
                break;
            }
        }
        self.diagnostics.truncate(mark);
        self.top_drawn.truncate(top);
        self.warned = warned;
        for member in members {
            if let Err(declined) = self.type_member(*member)? {
                return Ok(Err(declined));
            }
        }
        Ok(self.settled_group(members))
    }

    fn check_group(&mut self, members: &[u32]) -> Stage<()> {
        let mark = self.diagnostics.len();
        let warned = self.warned.clone();
        let mut queue: std::collections::BTreeSet<usize> = (0..members.len()).collect();
        let mut budget = members.len().saturating_mul(SETTLE_ROUNDS);
        let at_of: HashMap<u32, usize> = members
            .iter()
            .enumerate()
            .map(|(at, member)| (*member, at))
            .collect();
        self.read_log.clear();
        while let Some(position) = queue.pop_last() {
            if budget == 0 {
                return Ok(Err(Declined(Reason::CellGroup)));
            }
            budget -= 1;
            let member = members.get(position).copied().unwrap_or(NONE);
            if let Err(declined) = self.check_cell(member)? {
                return Ok(Err(declined));
            }
            for read in std::mem::take(&mut self.read_log) {
                if let Some(at) = at_of.get(&read) {
                    queue.insert(*at);
                }
            }
        }
        self.diagnostics.truncate(mark);
        self.warned = warned;
        for member in members.iter().rev() {
            if let Err(declined) = self.check_cell(*member)? {
                return Ok(Err(declined));
            }
        }
        self.read_log.clear();
        Ok(Ok(()))
    }

    pub fn type_cells(&mut self) -> Stage<()> {
        let order = match self.cell_order()? {
            Ok(order) => order,
            Err(declined) => return Ok(Err(declined)),
        };
        let mut settled = HashSet::new();
        for at in order.iter().copied() {
            if self.is_cyclic_cell(at) {
                continue;
            }
            let group = self.cell_group.get(at as usize).copied().unwrap_or(NONE);
            if group != NONE {
                if settled.insert(group) {
                    let members = self.groups.get(group as usize).cloned().unwrap_or_default();
                    if let Err(declined) = self.settle_group(&members)? {
                        return Ok(Err(declined));
                    }
                }
                continue;
            }
            let root = self.cell_root(at)?;
            if let Err(declined) = self.ensure_typed(root)? {
                return Ok(Err(declined));
            }
            let init = self.ir.cells.get(at as usize).map_or(NONE, |rec| rec.init);
            if init != NONE
                && init != root
                && let Err(declined) = self.ensure_typed(init)?
            {
                return Ok(Err(declined));
            }
        }
        self.cell_order = order;
        Ok(Ok(()))
    }

    fn check_cell(&mut self, at: u32) -> Stage<()> {
        if self.reactive_host_init(at)? {
            return Ok(Ok(()));
        }
        let root = self.cell_root(at)?;
        if let Some(ty) = self.cell_annotation_ty(at)? {
            if let Err(declined) = self.check_cell_annotation(at, ty)? {
                return Ok(Err(declined));
            }
            return self.check_derived_init(at);
        }
        if let Err(declined) = self.check_own_cell(at, root)? {
            return Ok(Err(declined));
        }
        if let Err(declined) = self.run_rule_reads(at)? {
            return Ok(Err(declined));
        }
        self.check_derived_init(at)
    }

    pub fn check_cells(&mut self) -> Stage<()> {
        let mut checked = HashSet::new();
        for at in self.cell_order.clone().into_iter().rev() {
            if self.is_cyclic_cell(at) {
                continue;
            }
            let group = self.cell_group.get(at as usize).copied().unwrap_or(NONE);
            if group == NONE {
                if let Err(declined) = self.check_cell(at)? {
                    return Ok(Err(declined));
                }
                continue;
            }
            if !checked.insert(group) {
                continue;
            }
            let members = self.groups.get(group as usize).cloned().unwrap_or_default();
            if let Err(declined) = self.check_group(&members)? {
                return Ok(Err(declined));
            }
        }
        self.unstarted_cycles()?;
        self.unrun_cells()?;
        Ok(Ok(()))
    }
}

impl Core<'_> {
    fn cell_text(&self, at: u32) -> Result<(String, CellWord, u32), Internal> {
        let rec = self
            .ir
            .cells
            .get(at as usize)
            .ok_or(Internal::missed("IR cell"))?;
        let name = self.ir.binder(rec.binder)?.name;
        let entry = self.ir.name(name).ok_or(Internal::missed("IR name"))?;
        Ok((self.ir.symbols.text(entry.sym).to_string(), rec.word, name))
    }

    fn group_unrun(&self, members: &[u32]) -> bool {
        let mut own = false;
        for member in members {
            if !self.derived(*member) {
                return false;
            }
            let faulty = self
                .cell_root(*member)
                .and_then(|root| self.fact(root))
                .map_or(true, |fact| {
                    matches!(fact.leaf, Leaf::Placeholder | Leaf::Error)
                });
            if faulty {
                return false;
            }
            let Some(reads) = self.cell_reads.get(*member as usize) else {
                return false;
            };
            if reads.present.iter().any(|read| !members.contains(read)) {
                return false;
            }
            for (target, _) in &reads.history {
                if !members.contains(target) {
                    return false;
                }
                own = true;
            }
        }
        own
    }

    fn unrun_cells(&mut self) -> Result<(), Internal> {
        let groups = self.groups.clone();
        for members in groups {
            if !self.group_unrun(&members) {
                continue;
            }
            let Some(first) = members.iter().min().copied() else {
                continue;
            };
            let (text, _, name) = self.cell_text(first)?;
            let file = self.file_of(self.cell_root(first)?);
            let span = self.name_span(file, name)?;
            let summary = if members.len() == 1 {
                format!(
                    "`{text}` has no clock and reads no cell but its own history, so nothing runs it"
                )
            } else {
                format!(
                    "`{text}` lies on a cycle of cells with no clock that reads no cell outside itself, so nothing runs it"
                )
            };
            let help = format!(
                "give `{text}` a clock, as in `state {text}(0) on tick = …`, or read a cell that steps in its rule"
            );
            let diagnostic = Diagnostic::new(Code::UnrunCell, span, summary).with_help(help);
            self.push_drawn(diagnostic);
        }
        Ok(())
    }

    fn unstarted_cycles(&mut self) -> Result<(), Internal> {
        let count = self.cell_reads.len();
        let mut edges: Vec<Vec<u32>> = vec![Vec::new(); count];
        let mut reads = Vec::new();
        for (at, out) in edges.iter_mut().enumerate() {
            let reader = u32::try_from(at).unwrap_or(NONE);
            let has_init = self.ir.cells.get(at).is_some_and(|rec| rec.init != NONE);
            if has_init || !self.derived(reader) || self.is_cyclic_cell(reader) {
                continue;
            }
            let Some(found) = self.cell_reads.get(at) else {
                continue;
            };
            out.extend(found.present.iter().copied());
            for (target, node) in &found.history {
                let (op, _, _, fallback) = self.ir.node(*node)?;
                if op == Op::Past && fallback == NONE {
                    out.push(*target);
                    reads.push((reader, *target, *node));
                }
            }
        }
        let mut component_of = vec![NONE; count];
        for (component, members) in strongly_connected(&edges).into_iter().enumerate() {
            for member in members {
                if let Some(slot) = component_of.get_mut(member as usize) {
                    *slot = u32::try_from(component).unwrap_or(NONE);
                }
            }
        }
        let mut earliest: HashMap<u32, ((u32, u32), u32, u32)> = HashMap::new();
        for (reader, target, node) in reads {
            let (Some(from), Some(to)) = (
                component_of.get(reader as usize).copied(),
                component_of.get(target as usize).copied(),
            ) else {
                continue;
            };
            let target_init = self
                .ir
                .cells
                .get(target as usize)
                .is_some_and(|rec| rec.init != NONE);
            if from != to || target_init {
                continue;
            }
            let key = (self.file_of(node), self.ir.span(node)?.0);
            let slot = earliest.entry(from).or_insert((key, target, node));
            if key < slot.0 {
                *slot = (key, target, node);
            }
        }
        let mut found: Vec<((u32, u32), u32, u32)> = earliest.into_values().collect();
        found.sort_unstable();
        for (_, target, node) in found {
            let (text, word, _) = self.cell_text(target)?;
            let word = match word {
                CellWord::State => "state",
                CellWord::Stream => "stream",
                CellWord::Channel => "channel",
            };
            let fallback = self.fallback_text(target)?;
            let group = self
                .cell_group
                .get(target as usize)
                .copied()
                .unwrap_or(NONE);
            let unrun = self
                .groups
                .get(group as usize)
                .is_some_and(|members| self.group_unrun(members));
            let start = format!(
                "give `{text}` an init, as in `{word} {text}({fallback}) = …`, or this read a fallback, as in `{text}.past(1, {fallback})`"
            );
            let help = if unrun {
                format!("read a cell outside the cycle, and {start}")
            } else {
                start
            };
            let span = self.node_span(node)?;
            let diagnostic = Diagnostic::new(
                Code::HistoryFallback,
                span,
                format!(
                    "this read of `{text}` lies on a cycle of history reads with its reader, and `{text}` has no init and the read no fallback, so it has no value at load"
                ),
            )
            .with_help(help);
            self.push_drawn(diagnostic);
        }
        Ok(())
    }

    fn fallback_text(&self, at: u32) -> Result<String, Internal> {
        let root = self.cell_root(at)?;
        let fact = if self.has_fact(root)? {
            self.fact(root)?
        } else {
            Fact::PLACEHOLDER
        };
        Ok(if !fact.dims.is_empty() {
            String::from("[]")
        } else if fact.leaf == Leaf::Bool {
            String::from("false")
        } else {
            String::from("0")
        })
    }
}

fn resized(before: &Fact, after: &Fact) -> bool {
    let fixed = |fact: &Fact| {
        (!matches!(fact.leaf, Leaf::Placeholder | Leaf::Error))
            .then(|| fact.dims.first().copied())
            .flatten()
            .filter(|length| *length != ANY_LENGTH)
    };
    matches!((fixed(before), fixed(after)), (Some(a), Some(b)) if a != b)
}

fn open_kind(fact: &Fact) -> bool {
    matches!(fact.leaf, Leaf::Placeholder | Leaf::Error)
        || (fact.leaf == Leaf::Empty && fact.dims.is_empty())
}

fn present_order(component: &[u32], all: &[CellReads]) -> Option<Vec<u32>> {
    let mut sorted = component.to_vec();
    sorted.sort_unstable();
    let mut state: HashMap<u32, u8> = HashMap::new();
    let mut order = Vec::with_capacity(sorted.len());
    for first in sorted.iter().copied() {
        if state.contains_key(&first) {
            continue;
        }
        state.insert(first, 1);
        let mut stack = vec![(first, 0usize)];
        while let Some((at, next)) = stack.pop() {
            let read = all
                .get(at as usize)
                .and_then(|reads| reads.present.get(next))
                .copied();
            let Some(read) = read else {
                state.insert(at, 2);
                order.push(at);
                continue;
            };
            stack.push((at, next + 1));
            if !component.contains(&read) {
                continue;
            }
            match state.get(&read) {
                None => {
                    state.insert(read, 1);
                    stack.push((read, 0));
                }
                Some(1) => return None,
                Some(_) => {}
            }
        }
    }
    Some(order)
}

impl Core<'_> {
    fn settled_group(&self, members: &[u32]) -> Result<(), Declined> {
        for member in members {
            if self
                .group_kinds
                .get(member)
                .is_none_or(|fact| fact.leaf == Leaf::Error)
            {
                return Err(Declined(Reason::CellGroup));
            }
        }
        let placeholder = members.iter().any(|member| {
            self.group_kinds
                .get(member)
                .is_some_and(|fact| fact.leaf == Leaf::Placeholder)
        });
        let nodes = members
            .iter()
            .filter_map(|member| self.cell_reads.get(*member as usize))
            .flat_map(|reads| reads.history.iter().map(|(_, node)| *node))
            .filter(|node| self.group_reads.contains_key(node));
        for node in nodes {
            let fallback = self
                .ir
                .node(node)
                .map_or(NONE, |(_, _, _, fallback)| fallback);
            let open = self.group_reads.get(&node).is_some_and(|(target, _)| {
                self.ir
                    .cells
                    .get(*target as usize)
                    .is_some_and(|rec| rec.init == NONE)
            });
            if placeholder && fallback == NONE && open {
                return Err(Declined(Reason::CellGroup));
            }
        }
        Ok(())
    }
}
