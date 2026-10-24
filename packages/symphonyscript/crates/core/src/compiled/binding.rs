use alloc::vec;
use alloc::vec::Vec;
use core::fmt;
use core::hash::{Hash, Hasher};
use core::mem;

use crate::ids::{CellId, InputSlot};
use crate::types::Value;

pub enum Binding {
    Const(Value),
    Input(InputSlot),
    Cell(CellId),
    Array(Vec<Binding>),
}

enum CloneStep<'a> {
    Visit(&'a Binding),
    Collect(usize),
}

enum DebugStep<'a> {
    Visit(&'a Binding),
    Text(&'static str),
}

impl Clone for Binding {
    fn clone(&self) -> Self {
        let mut steps = vec![CloneStep::Visit(self)];
        let mut done: Vec<Binding> = Vec::new();
        while let Some(step) = steps.pop() {
            match step {
                CloneStep::Visit(Binding::Const(value)) => done.push(Binding::Const(*value)),
                CloneStep::Visit(Binding::Input(slot)) => done.push(Binding::Input(*slot)),
                CloneStep::Visit(Binding::Cell(cell)) => done.push(Binding::Cell(*cell)),
                CloneStep::Visit(Binding::Array(items)) => {
                    steps.push(CloneStep::Collect(items.len()));
                    steps.extend(items.iter().rev().map(CloneStep::Visit));
                }
                CloneStep::Collect(count) => {
                    let items = done.split_off(done.len() - count);
                    done.push(Binding::Array(items));
                }
            }
        }
        done.pop().unwrap_or(Binding::Array(Vec::new()))
    }
}

impl PartialEq for Binding {
    fn eq(&self, other: &Self) -> bool {
        let mut pending = vec![(self, other)];
        while let Some(pair) = pending.pop() {
            match pair {
                (Binding::Const(a), Binding::Const(b)) if a == b => {}
                (Binding::Input(a), Binding::Input(b)) if a == b => {}
                (Binding::Cell(a), Binding::Cell(b)) if a == b => {}
                (Binding::Array(a), Binding::Array(b)) if a.len() == b.len() => {
                    pending.extend(a.iter().zip(b));
                }
                _ => return false,
            }
        }
        true
    }
}

impl Eq for Binding {}

impl Hash for Binding {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let mut pending = vec![self];
        while let Some(binding) = pending.pop() {
            mem::discriminant(binding).hash(state);
            match binding {
                Binding::Const(value) => value.hash(state),
                Binding::Input(slot) => slot.hash(state),
                Binding::Cell(cell) => cell.hash(state),
                Binding::Array(items) => {
                    items.len().hash(state);
                    pending.extend(items.iter().rev());
                }
            }
        }
    }
}

impl fmt::Debug for Binding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut steps = vec![DebugStep::Visit(self)];
        while let Some(step) = steps.pop() {
            match step {
                DebugStep::Text(text) => f.write_str(text)?,
                DebugStep::Visit(Binding::Const(value)) => write!(f, "Const({value:?})")?,
                DebugStep::Visit(Binding::Input(slot)) => write!(f, "Input({slot:?})")?,
                DebugStep::Visit(Binding::Cell(cell)) => write!(f, "Cell({cell:?})")?,
                DebugStep::Visit(Binding::Array(items)) => {
                    f.write_str("Array([")?;
                    steps.push(DebugStep::Text("])"));
                    for (position, item) in items.iter().enumerate().rev() {
                        steps.push(DebugStep::Visit(item));
                        if position > 0 {
                            steps.push(DebugStep::Text(", "));
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

impl Drop for Binding {
    fn drop(&mut self) {
        let Binding::Array(items) = self else {
            return;
        };
        let mut pending = mem::take(items);
        while let Some(mut binding) = pending.pop() {
            if let Binding::Array(inner) = &mut binding {
                pending.append(inner);
            }
        }
    }
}
