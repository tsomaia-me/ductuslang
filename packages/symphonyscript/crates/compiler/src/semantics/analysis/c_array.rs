use std::cell::{Cell, OnceCell};
use std::fmt;
use std::rc::Rc;

use super::c_val::CVal;
use super::fast_hash::FastSet;
use super::lengths_of::lengths_of;
use crate::diagnostics::Code;
use crate::semantics::analysis::resolve::LENGTH_LIMIT;

thread_local! {
    static BUILT_ELEMENTS: Cell<u64> = const { Cell::new(0) };
    static VISITED_ARRAYS: Cell<u64> = const { Cell::new(0) };
}

fn count_built(elements: usize) {
    let elements = u64::try_from(elements).unwrap_or(u64::MAX);
    BUILT_ELEMENTS.with(|built| built.set(built.get().saturating_add(elements)));
}

#[derive(Clone)]
pub(crate) struct CArray(Rc<Repr>);

enum Repr {
    Items(Vec<CVal>),
    Join(Joined),
    Slice(Sliced),
}

struct Sliced {
    base: CArray,
    start: usize,
    length: usize,
    items: OnceCell<Vec<CVal>>,
}

struct Joined {
    left: CArray,
    right: CArray,
    length: usize,
    row: Option<Vec<u32>>,
    items: OnceCell<Vec<CVal>>,
}

impl CArray {
    pub fn count_visit() {
        VISITED_ARRAYS.with(|visited| visited.set(visited.get().saturating_add(1)));
    }

    #[cfg(test)]
    pub fn work() -> (u64, u64) {
        (
            BUILT_ELEMENTS.with(Cell::get),
            VISITED_ARRAYS.with(Cell::get),
        )
    }

    pub fn new(items: Vec<CVal>) -> CArray {
        count_built(items.len());
        CArray(Rc::new(Repr::Items(items)))
    }

    fn vacant() -> CArray {
        CArray(Rc::new(Repr::Items(Vec::new())))
    }

    pub fn join(left: &CArray, right: &CArray) -> Result<CArray, Code> {
        let row = match (left.row(), right.row()) {
            (Some(left_row), Some(right_row)) if left_row != right_row => {
                return Err(Code::RaggedArray);
            }
            (Some(row), _) | (None, Some(row)) => Some(row),
            (None, None) => None,
        };
        let length = left.len().saturating_add(right.len());
        if u32::try_from(length).map_or(true, |length| length >= LENGTH_LIMIT) {
            return Err(Code::LengthLimit);
        }
        Ok(CArray(Rc::new(Repr::Join(Joined {
            left: left.clone(),
            right: right.clone(),
            length,
            row,
            items: OnceCell::new(),
        }))))
    }

    pub fn slice(base: &CArray, start: usize, end: usize) -> CArray {
        let start = start.min(base.len());
        let length = end.min(base.len()).saturating_sub(start);
        if start == 0 && length == base.len() {
            return base.clone();
        }
        CArray(Rc::new(Repr::Slice(Sliced {
            base: base.clone(),
            start,
            length,
            items: OnceCell::new(),
        })))
    }

    pub fn sliced(&self) -> Option<(&CArray, usize, usize)> {
        match &*self.0 {
            Repr::Slice(sliced) => Some((&sliced.base, sliced.start, sliced.length)),
            Repr::Items(_) | Repr::Join(_) => None,
        }
    }

    pub fn identity(&self) -> *const () {
        Rc::as_ptr(&self.0).cast()
    }

    pub fn operands(&self) -> Option<(&CArray, &CArray)> {
        match &*self.0 {
            Repr::Items(_) | Repr::Slice(_) => None,
            Repr::Join(joined) => Some((&joined.left, &joined.right)),
        }
    }

    pub fn len(&self) -> usize {
        match &*self.0 {
            Repr::Items(items) => items.len(),
            Repr::Join(joined) => joined.length,
            Repr::Slice(sliced) => sliced.length,
        }
    }

    fn row(&self) -> Option<Vec<u32>> {
        match &*self.0 {
            Repr::Items(items) => items.first().map(lengths_of),
            Repr::Join(joined) => joined.row.clone(),
            Repr::Slice(sliced) if sliced.length == 0 => None,
            Repr::Slice(sliced) => sliced.base.row(),
        }
    }

    pub fn get(&self, index: usize) -> Option<&CVal> {
        let mut array = self;
        let mut index = index;
        loop {
            match &*array.0 {
                Repr::Items(items) => return items.get(index),
                Repr::Join(joined) => {
                    if let Some(items) = joined.items.get() {
                        return items.get(index);
                    }
                    if index < joined.left.len() {
                        array = &joined.left;
                    } else {
                        index -= joined.left.len();
                        array = &joined.right;
                    }
                }
                Repr::Slice(sliced) => {
                    if let Some(items) = sliced.items.get() {
                        return items.get(index);
                    }
                    if index >= sliced.length {
                        return None;
                    }
                    index += sliced.start;
                    array = &sliced.base;
                }
            }
        }
    }

    pub fn range(&self, start: usize, end: usize) -> Vec<CVal> {
        let mut out = Vec::with_capacity(end.saturating_sub(start));
        let mut stack: Vec<(&CArray, usize, usize)> = vec![(self, start, end)];
        while let Some((array, start, end)) = stack.pop() {
            crate::semantics::counted_walk::CountedWalk::CArrayRange.step();
            if start >= end {
                continue;
            }
            match &*array.0 {
                Repr::Items(items) => {
                    out.extend(items.get(start..end).unwrap_or_default().iter().cloned())
                }
                Repr::Join(joined) => match joined.items.get() {
                    Some(items) => {
                        out.extend(items.get(start..end).unwrap_or_default().iter().cloned());
                    }
                    None => {
                        let split = joined.left.len();
                        stack.push((
                            &joined.right,
                            start.saturating_sub(split),
                            end.saturating_sub(split),
                        ));
                        stack.push((&joined.left, start.min(split), end.min(split)));
                    }
                },
                Repr::Slice(sliced) => match sliced.items.get() {
                    Some(items) => {
                        out.extend(items.get(start..end).unwrap_or_default().iter().cloned());
                    }
                    None => {
                        let end = end.min(sliced.length);
                        stack.push((
                            &sliced.base,
                            sliced.start.saturating_add(start),
                            sliced.start.saturating_add(end),
                        ));
                    }
                },
            }
        }
        count_built(out.len());
        out
    }

    pub fn items(&self) -> &[CVal] {
        match &*self.0 {
            Repr::Items(items) => items,
            Repr::Join(joined) => joined.items.get_or_init(|| self.range(0, joined.length)),
            Repr::Slice(sliced) => sliced.items.get_or_init(|| self.range(0, sliced.length)),
        }
    }

    pub fn lengths(&self) -> Vec<u32> {
        let mut lengths = self.row().unwrap_or_default();
        lengths.push(u32::try_from(self.len()).unwrap_or(u32::MAX));
        lengths
    }

    pub fn leaves(&self) -> Vec<CVal> {
        let mut found = Vec::new();
        let mut stack: Vec<(&CArray, usize)> = vec![(self, 0)];
        while let Some((array, index)) = stack.pop() {
            crate::semantics::counted_walk::CountedWalk::CArrayLeaves.step();
            let Some(item) = array.items().get(index) else {
                continue;
            };
            stack.push((array, index + 1));
            match item {
                CVal::Array(inner) => stack.push((inner, 0)),
                leaf => found.push(leaf.clone()),
            }
        }
        found
    }

    pub fn all_leaves(&self, mut holds: impl FnMut(&CVal) -> bool) -> bool {
        let mut seen: FastSet<(*const Repr, usize, usize)> = FastSet::default();
        let mut stack: Vec<(&CArray, usize, usize)> = vec![(self, 0, self.len())];
        while let Some((array, start, end)) = stack.pop() {
            crate::semantics::counted_walk::CountedWalk::CArrayAllLeaves.step();
            if start >= end || !seen.insert((Rc::as_ptr(&array.0), start, end)) {
                continue;
            }
            CArray::count_visit();
            let items = match &*array.0 {
                Repr::Items(items) => items.get(start..end).unwrap_or_default(),
                Repr::Join(joined) => {
                    let split = joined.left.len();
                    stack.push((
                        &joined.right,
                        start.saturating_sub(split),
                        end.saturating_sub(split),
                    ));
                    stack.push((&joined.left, start.min(split), end.min(split)));
                    continue;
                }
                Repr::Slice(sliced) => {
                    let end = end.min(sliced.length);
                    stack.push((
                        &sliced.base,
                        sliced.start.saturating_add(start),
                        sliced.start.saturating_add(end),
                    ));
                    continue;
                }
            };
            for item in items {
                match item {
                    CVal::Array(inner) => stack.push((inner, 0, inner.len())),
                    leaf => {
                        if !holds(leaf) {
                            return false;
                        }
                    }
                }
            }
        }
        true
    }
}

impl PartialEq for CArray {
    fn eq(&self, other: &Self) -> bool {
        let mut pairs: Vec<(&CArray, &CArray)> = vec![(self, other)];
        while let Some((left, right)) = pairs.pop() {
            crate::semantics::counted_walk::CountedWalk::CArrayEq.step();
            if Rc::ptr_eq(&left.0, &right.0) {
                continue;
            }
            if left.len() != right.len() {
                return false;
            }
            for (a, b) in left.items().iter().zip(right.items().iter()) {
                match (a, b) {
                    (CVal::Array(a), CVal::Array(b)) => pairs.push((a, b)),
                    (a, b) => {
                        if a != b {
                            return false;
                        }
                    }
                }
            }
        }
        true
    }
}

impl fmt::Debug for CArray {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut out = String::from("[");
        let mut stack: Vec<(&CArray, usize)> = vec![(self, 0)];
        while let Some((array, index)) = stack.pop() {
            crate::semantics::counted_walk::CountedWalk::CArrayFmt.step();
            let Some(item) = array.items().get(index) else {
                out.push(']');
                continue;
            };
            stack.push((array, index + 1));
            if index > 0 {
                out.push_str(", ");
            }
            match item {
                CVal::Array(inner) => {
                    out.push('[');
                    stack.push((inner, 0));
                }
                leaf => out.push_str(&format!("{leaf:?}")),
            }
        }
        f.write_str(&out)
    }
}

impl Drop for CArray {
    fn drop(&mut self) {
        let mut items = Vec::new();
        let mut arrays = Vec::new();
        take_parts(&mut self.0, &mut items, &mut arrays);
        loop {
            if let Some(mut array) = arrays.pop() {
                take_parts(&mut array.0, &mut items, &mut arrays);
                continue;
            }
            match items.pop() {
                Some(CVal::Array(array)) => arrays.push(array),
                Some(_) => {}
                None => break,
            }
        }
    }
}

fn take_parts(repr: &mut Rc<Repr>, items: &mut Vec<CVal>, arrays: &mut Vec<CArray>) {
    let Some(repr) = Rc::get_mut(repr) else {
        return;
    };
    match repr {
        Repr::Items(own) => items.append(own),
        Repr::Join(joined) => {
            if let Some(own) = joined.items.get_mut() {
                items.append(own);
            }
            arrays.push(std::mem::replace(&mut joined.left, CArray::vacant()));
            arrays.push(std::mem::replace(&mut joined.right, CArray::vacant()));
        }
        Repr::Slice(sliced) => {
            if let Some(own) = sliced.items.get_mut() {
                items.append(own);
            }
            arrays.push(std::mem::replace(&mut sliced.base, CArray::vacant()));
        }
    }
}
