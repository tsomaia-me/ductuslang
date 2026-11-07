use std::ops::RangeInclusive;

use super::core::Core;
use super::core_value::CoreValue;
use crate::diagnostics::Code;
use crate::semantics::Internal;
use crate::semantics::counted_walk::CountedWalk;
use crate::semantics::operators::{EvalTy, Value, int_range};

pub(crate) type Index = super::column::PairColumn<Vec<(usize, u64)>>;

#[derive(Clone, Debug)]
pub(crate) struct InitRead {
    pub node: u32,
    pub span: u32,
    pub cell: u32,
    pub held: Vec<CoreValue>,
    pub ty: EvalTy,
    pub chain: u32,
    pub open: bool,
    pub pinned: bool,
    pub bound: bool,
    pub serial: u64,
}

fn listed(index: &mut Index, entry: &InitRead, at: usize) {
    index
        .entry((entry.node, entry.chain))
        .or_default()
        .push((at, entry.serial));
}

fn found(
    index: &mut Index,
    reads: &[InitRead],
    (node, chain): (u32, u32),
    fits: fn(&InitRead) -> bool,
) -> Option<usize> {
    let list = index.get_mut(&(node, chain))?;
    let live = |(at, serial): (usize, u64)| {
        reads.get(at).is_some_and(|entry| {
            entry.serial == serial
                && entry.open
                && entry.node == node
                && entry.chain == chain
                && fits(entry)
        })
    };
    while let Some(last) = list.last().copied() {
        CountedWalk::CoreInitRead.step();
        if live(last) {
            return Some(last.0);
        }
        list.pop();
    }
    None
}

impl Core<'_> {
    pub fn hold_init_read(&mut self, node: u32, cell: u32, held: CoreValue, ty: EvalTy) {
        let chain = self.eval_chain.id();
        self.push_init_read(InitRead {
            node,
            span: node,
            cell,
            held: vec![held],
            ty,
            chain,
            open: true,
            pinned: false,
            bound: false,
            serial: 0,
        });
    }

    fn push_init_read(&mut self, mut entry: InitRead) {
        self.init_serial += 1;
        entry.serial = self.init_serial;
        let at = self.init_reads.len();
        listed(&mut self.init_open, &entry, at);
        listed(&mut self.init_args, &entry, at);
        self.init_reads.push(entry);
    }

    fn relisted(&mut self, at: usize) {
        if let Some(entry) = self.init_reads.get(at) {
            listed(&mut self.init_open, entry, at);
            listed(&mut self.init_args, entry, at);
        }
    }

    fn dropped(&mut self, at: usize) {
        if let Some(entry) = self.init_reads.get_mut(at) {
            entry.open = false;
            entry.held.clear();
        }
    }

    fn open_init_read(&mut self, base: u32) -> Option<usize> {
        found(
            &mut self.init_open,
            &self.init_reads,
            (base, self.eval_chain.id()),
            |entry| !entry.bound,
        )
    }

    fn arrays(held: &[CoreValue]) -> Option<Vec<u32>> {
        held.iter()
            .map(|value| match value {
                Value::Array(array) => Some(*array),
                Value::Int(_)
                | Value::F32(_)
                | Value::F64(_)
                | Value::Bool(_)
                | Value::Enum(..)
                | Value::Str(_)
                | Value::Node(_)
                | Value::Error => None,
            })
            .collect()
    }

    fn held_at(&self, at: usize) -> Vec<CoreValue> {
        self.init_reads
            .get(at)
            .map(|entry| entry.held.clone())
            .unwrap_or_default()
    }

    pub fn step_init_read(
        &mut self,
        (read, base): (u32, u32),
        position: &CoreValue,
        from_end: bool,
    ) {
        let Some(at) = self.open_init_read(base) else {
            return;
        };
        let Some(arrays) = Self::arrays(&self.held_at(at)) else {
            self.close_init_read(at);
            return;
        };
        let stepped: Option<Vec<CoreValue>> = match position {
            Value::Int(position) => arrays
                .iter()
                .map(|array| {
                    let length = i128::from(self.store.len(*array));
                    let position = if from_end {
                        length.checked_sub(*position)
                    } else {
                        Some(*position)
                    };
                    position
                        .and_then(|position| usize::try_from(position).ok())
                        .and_then(|position| self.store.element(*array, position))
                })
                .collect(),
            Value::F32(_)
            | Value::F64(_)
            | Value::Bool(_)
            | Value::Enum(..)
            | Value::Str(_)
            | Value::Node(_)
            | Value::Array(_)
            | Value::Error => {
                let mut every = Vec::new();
                for array in &arrays {
                    let length = self.store.len(*array) as usize;
                    every.extend((0..length).filter_map(|at| self.store.element(*array, at)));
                }
                if let Some(entry) = self.init_reads.get_mut(at) {
                    entry.pinned = true;
                }
                Some(every)
            }
        };
        match stepped {
            Some(stepped) => self.move_init_read(at, read, stepped),
            None => {
                self.dropped(at);
            }
        }
    }

    pub fn slice_init_read(
        &mut self,
        (read, base): (u32, u32),
        lower: Option<(&CoreValue, bool)>,
        upper: Option<(&CoreValue, bool)>,
    ) {
        let Some(at) = self.open_init_read(base) else {
            return;
        };
        let known = |side: Option<(&CoreValue, bool)>| match side {
            None => Some(None),
            Some((Value::Int(value), from_end)) => Some(Some((*value, from_end))),
            Some(_) => None,
        };
        let (Some(arrays), Some(lower), Some(upper)) =
            (Self::arrays(&self.held_at(at)), known(lower), known(upper))
        else {
            self.close_init_read(at);
            return;
        };
        let mut sliced = Vec::with_capacity(arrays.len());
        for array in arrays {
            let length = i128::from(self.store.len(array));
            let place = |side: Option<(i128, bool)>, absent: i128| match side {
                None => absent,
                Some((value, true)) => length.saturating_sub(value),
                Some((value, false)) => value,
            };
            let (start, end) = (place(lower, 0), place(upper, length));
            let within = 0 <= start && start <= end && end <= length;
            let (true, Ok(start), Ok(end)) = (within, usize::try_from(start), usize::try_from(end))
            else {
                self.dropped(at);
                return;
            };
            sliced.push(Value::Array(self.store.slice(array, start, end)));
        }
        self.move_init_read(at, read, sliced);
    }

    pub fn pass_init_read(&mut self, read: u32, base: u32) {
        if let Some(at) = self.open_init_read(base) {
            let held = self.held_at(at);
            self.move_init_read(at, read, held);
        }
    }

    pub fn bind_init_read(&mut self, read: u32, argument: u32, ty: EvalTy) {
        if self.eval_chain.is_empty() {
            return;
        }
        let caller = self.eval_chain.caller_id();
        let Some(at) = found(
            &mut self.init_args,
            &self.init_reads,
            (argument, caller),
            |entry| !entry.pinned,
        ) else {
            return;
        };
        let held = self.held_at(at);
        if Self::arrays(&held).is_none() {
            return;
        }
        let Some(entry) = self.init_reads.get_mut(at) else {
            return;
        };
        entry.bound = true;
        let cell = entry.cell;
        let chain = self.eval_chain.id();
        self.push_init_read(InitRead {
            node: read,
            span: read,
            cell,
            held,
            ty,
            chain,
            open: true,
            pinned: false,
            bound: false,
            serial: 0,
        });
    }

    pub fn return_init_read(&mut self, call: u32, root: u32) {
        if self.eval_chain.is_empty() {
            return;
        }
        let caller = self.eval_chain.caller_id();
        let Some(at) = self.open_init_read(root) else {
            return;
        };
        if Self::arrays(&self.held_at(at)).is_none() {
            return;
        }
        if let Some(entry) = self.init_reads.get_mut(at)
            && !entry.pinned
        {
            entry.node = call;
            entry.span = call;
            entry.chain = caller;
            self.relisted(at);
        }
    }

    pub fn whole_init_read(&mut self) {
        if let Some(entry) = self.init_reads.last_mut() {
            entry.open = false;
        }
    }

    fn close_init_read(&mut self, at: usize) {
        if let Some(entry) = self.init_reads.get_mut(at) {
            entry.open = false;
        }
    }

    fn move_init_read(&mut self, at: usize, read: u32, held: Vec<CoreValue>) {
        if let Some(entry) = self.init_reads.get_mut(at) {
            entry.node = read;
            if !entry.pinned {
                entry.span = read;
            }
            entry.held = held;
        }
        self.relisted(at);
    }

    pub fn flush_init_reads(&mut self, from: usize) -> Result<(), Internal> {
        let reads = self.init_reads.split_off(from.min(self.init_reads.len()));
        if self.init_reads.is_empty() {
            self.init_open.clear();
            self.init_args.clear();
        }
        let chain = std::mem::take(&mut self.eval_chain);
        let mut outcome = Ok(());
        for read in reads {
            CountedWalk::CoreInitRead.step();
            if read.bound {
                continue;
            }
            let Some(range) = int_range(read.ty) else {
                continue;
            };
            if !read
                .held
                .iter()
                .any(|held| self.outside_range(held, &range))
            {
                continue;
            }
            self.eval_chain = self.chain_arena.chain_at(read.chain);
            outcome = self.draw_init_misfit(read.cell, read.span);
            if outcome.is_err() {
                break;
            }
        }
        self.eval_chain = chain;
        outcome
    }

    pub fn draw_init_misfit(&mut self, cell: u32, node: u32) -> Result<(), Internal> {
        let rec = self
            .ir
            .cells
            .get(cell as usize)
            .copied()
            .ok_or(Internal::missed("IR cell"))?;
        let name = self.ir.binder(rec.binder)?.name;
        let entry = self.ir.name(name).ok_or(Internal::missed("IR name"))?;
        let text = self.ir.symbols.text(entry.sym).to_string();
        self.draw_at(
            Code::DoesNotFit,
            node,
            format!("the initial value of `{text}` does not fit the type this use reads it in"),
        )
    }

    pub fn outside_range(&mut self, init: &CoreValue, range: &RangeInclusive<i128>) -> bool {
        let mut stack = vec![init.clone()];
        while let Some(value) = stack.pop() {
            match value {
                Value::Int(value) if !range.contains(&value) => return true,
                Value::Array(array) => {
                    let length = self.store.len(array) as usize;
                    stack.extend((0..length).filter_map(|at| self.store.element(array, at)));
                }
                Value::Int(_)
                | Value::F32(_)
                | Value::F64(_)
                | Value::Bool(_)
                | Value::Enum(..)
                | Value::Str(_)
                | Value::Node(_)
                | Value::Error => {}
            }
        }
        false
    }
}
