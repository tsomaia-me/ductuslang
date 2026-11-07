use std::collections::VecDeque;

use crate::semantics::ir::flat::NONE;

#[derive(Clone, Debug)]
pub(crate) struct Column<T> {
    base: u32,
    slots: Vec<Option<T>>,
    none: Option<T>,
    len: usize,
}

impl<T> Default for Column<T> {
    fn default() -> Self {
        Column {
            base: 0,
            slots: Vec::new(),
            none: None,
            len: 0,
        }
    }
}

impl<T> Column<T> {
    pub fn new() -> Self {
        Column::default()
    }

    fn at(&self, id: u32) -> Option<usize> {
        id.checked_sub(self.base).map(|local| local as usize)
    }

    fn slot(&self, id: u32) -> Option<&Option<T>> {
        if id == NONE {
            return Some(&self.none);
        }
        self.slots.get(self.at(id)?)
    }

    fn slot_mut(&mut self, id: u32) -> &mut Option<T> {
        if id == NONE {
            return &mut self.none;
        }
        if self.slots.is_empty() {
            self.base = id;
        } else if id < self.base {
            let wanted = (self.base - id).max(u32::try_from(self.slots.len()).unwrap_or(u32::MAX));
            let shift = wanted.min(self.base);
            let mut moved = Vec::with_capacity(self.slots.len() + shift as usize);
            moved.resize_with(shift as usize, || None);
            moved.append(&mut self.slots);
            self.slots = moved;
            self.base -= shift;
        }
        let at = (id - self.base) as usize;
        if self.slots.len() <= at {
            self.slots.resize_with(at + 1, || None);
        }
        match self.slots.get_mut(at) {
            Some(slot) => slot,
            None => &mut self.none,
        }
    }

    pub fn get(&self, id: &u32) -> Option<&T> {
        self.slot(*id)?.as_ref()
    }

    pub fn get_mut(&mut self, id: &u32) -> Option<&mut T> {
        if *id == NONE {
            return self.none.as_mut();
        }
        let at = self.at(*id)?;
        self.slots.get_mut(at)?.as_mut()
    }

    pub fn contains_key(&self, id: &u32) -> bool {
        self.get(id).is_some()
    }

    pub fn insert(&mut self, id: u32, value: T) -> Option<T> {
        let old = self.slot_mut(id).replace(value);
        if old.is_none() {
            self.len += 1;
        }
        old
    }

    pub fn remove(&mut self, id: &u32) -> Option<T> {
        if *id != NONE && self.at(*id).is_none_or(|at| at >= self.slots.len()) {
            return None;
        }
        let old = self.slot_mut(*id).take();
        if old.is_some() {
            self.len -= 1;
        }
        old
    }

    pub fn or_insert_with(&mut self, id: u32, value: impl FnOnce() -> T) -> &mut T {
        if self.get(&id).is_none() {
            self.len += 1;
        }
        self.slot_mut(id).get_or_insert_with(value)
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn iter(&self) -> impl Iterator<Item = (u32, &T)> {
        let base = self.base;
        self.slots
            .iter()
            .zip(0u32..)
            .filter_map(move |(slot, at)| slot.as_ref().map(|value| (base + at, value)))
            .chain(self.none.as_ref().map(|value| (NONE, value)))
    }

    pub fn keys(&self) -> impl Iterator<Item = u32> + '_ {
        self.iter().map(|(id, _)| id)
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct ColumnSet {
    held: Column<()>,
}

impl ColumnSet {
    pub fn new() -> Self {
        ColumnSet::default()
    }

    pub fn insert(&mut self, id: u32) -> bool {
        self.held.insert(id, ()).is_none()
    }

    pub fn contains(&self, id: &u32) -> bool {
        self.held.contains_key(id)
    }

    pub fn remove(&mut self, id: &u32) -> bool {
        self.held.remove(id).is_some()
    }

    pub fn is_empty(&self) -> bool {
        self.held.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = u32> + '_ {
        self.held.keys()
    }
}

#[derive(Clone, Debug)]
pub(crate) struct PairColumn<T> {
    rows: Column<VecDeque<(u32, T)>>,
    len: usize,
}

impl<T> Default for PairColumn<T> {
    fn default() -> Self {
        PairColumn {
            rows: Column::new(),
            len: 0,
        }
    }
}

impl<T> PairColumn<T> {
    pub fn get(&self, key: &(u32, u32)) -> Option<&T> {
        let row = self.rows.get(&key.0)?;
        let at = row
            .binary_search_by_key(&key.1, |(second, _)| *second)
            .ok()?;
        row.get(at).map(|(_, value)| value)
    }

    pub fn contains_key(&self, key: &(u32, u32)) -> bool {
        self.get(key).is_some()
    }

    pub fn insert(&mut self, key: (u32, u32), value: T) -> Option<T> {
        let row = self.rows.or_insert_with(key.0, VecDeque::new);
        match row.binary_search_by_key(&key.1, |(second, _)| *second) {
            Ok(at) => row
                .get_mut(at)
                .map(|(_, held)| std::mem::replace(held, value)),
            Err(at) => {
                row.insert(at, (key.1, value));
                self.len += 1;
                None
            }
        }
    }

    pub fn remove(&mut self, key: &(u32, u32)) -> Option<T> {
        let row = self.rows.get_mut(&key.0)?;
        let at = row
            .binary_search_by_key(&key.1, |(second, _)| *second)
            .ok()?;
        let (_, value) = row.remove(at)?;
        self.len -= 1;
        Some(value)
    }

    pub fn or_insert_with(&mut self, key: (u32, u32), value: impl FnOnce() -> T) -> &mut T {
        let len = &mut self.len;
        let row = self.rows.or_insert_with(key.0, VecDeque::new);
        let at = match row.binary_search_by_key(&key.1, |(second, _)| *second) {
            Ok(at) => at,
            Err(at) => {
                row.insert(at, (key.1, value()));
                *len += 1;
                at
            }
        };
        &mut row[at].1
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn iter(&self) -> impl Iterator<Item = ((u32, u32), &T)> {
        self.rows.iter().flat_map(|(first, row)| {
            row.iter()
                .map(move |(second, value)| ((first, *second), value))
        })
    }

    pub fn values(&self) -> impl Iterator<Item = &T> {
        self.iter().map(|(_, value)| value)
    }

    pub fn row(&self, first: u32) -> impl Iterator<Item = (u32, &T)> {
        self.rows
            .get(&first)
            .into_iter()
            .flat_map(|row| row.iter().map(|(second, value)| (*second, value)))
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct PairSet {
    held: PairColumn<()>,
}

impl PairSet {
    pub fn insert(&mut self, key: (u32, u32)) -> bool {
        self.held.insert(key, ()).is_none()
    }

    pub fn contains(&self, key: &(u32, u32)) -> bool {
        self.held.contains_key(key)
    }

    pub fn remove(&mut self, key: &(u32, u32)) -> bool {
        self.held.remove(key).is_some()
    }

    pub fn is_empty(&self) -> bool {
        self.held.is_empty()
    }

    pub fn row(&self, first: u32) -> impl Iterator<Item = u32> + '_ {
        self.held.row(first).map(|(second, ())| second)
    }
}

pub(crate) struct Entry<'a, T> {
    column: &'a mut Column<T>,
    id: u32,
}

impl<'a, T> Entry<'a, T> {
    pub fn or_default(self) -> &'a mut T
    where
        T: Default,
    {
        self.column.or_insert_with(self.id, T::default)
    }
}

impl<T> Column<T> {
    pub fn entry(&mut self, id: u32) -> Entry<'_, T> {
        Entry { column: self, id }
    }
}

pub(crate) struct PairEntry<'a, T> {
    column: &'a mut PairColumn<T>,
    key: (u32, u32),
}

impl<'a, T> PairEntry<'a, T> {
    pub fn or_default(self) -> &'a mut T
    where
        T: Default,
    {
        self.column.or_insert_with(self.key, T::default)
    }
}

impl<T> PairColumn<T> {
    pub fn entry(&mut self, key: (u32, u32)) -> PairEntry<'_, T> {
        PairEntry { column: self, key }
    }
}

impl FromIterator<u32> for ColumnSet {
    fn from_iter<I: IntoIterator<Item = u32>>(ids: I) -> Self {
        let mut set = ColumnSet::new();
        set.extend(ids);
        set
    }
}

impl Extend<u32> for ColumnSet {
    fn extend<I: IntoIterator<Item = u32>>(&mut self, ids: I) {
        for id in ids {
            self.insert(id);
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct TripleColumn<T> {
    rows: Column<PairColumn<T>>,
    len: usize,
}

impl<T> Default for TripleColumn<T> {
    fn default() -> Self {
        TripleColumn {
            rows: Column::new(),
            len: 0,
        }
    }
}

impl<T> TripleColumn<T> {
    pub fn get(&self, key: &(u32, u32, u32)) -> Option<&T> {
        self.rows.get(&key.0)?.get(&(key.1, key.2))
    }

    pub fn insert(&mut self, key: (u32, u32, u32), value: T) -> Option<T> {
        let old = self
            .rows
            .or_insert_with(key.0, PairColumn::default)
            .insert((key.1, key.2), value);
        if old.is_none() {
            self.len += 1;
        }
        old
    }

    pub fn remove(&mut self, key: &(u32, u32, u32)) -> Option<T> {
        let old = self.rows.get_mut(&key.0)?.remove(&(key.1, key.2));
        if old.is_some() {
            self.len -= 1;
        }
        old
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn iter(&self) -> impl Iterator<Item = ((u32, u32, u32), &T)> {
        self.rows.iter().flat_map(|(first, row)| {
            row.iter()
                .map(move |((second, third), value)| ((first, second, third), value))
        })
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct TripleSet {
    held: TripleColumn<()>,
}

impl TripleSet {
    pub fn insert(&mut self, key: (u32, u32, u32)) -> bool {
        self.held.insert(key, ()).is_none()
    }

    pub fn contains(&self, key: &(u32, u32, u32)) -> bool {
        self.held.get(key).is_some()
    }

    pub fn remove(&mut self, key: &(u32, u32, u32)) -> bool {
        self.held.remove(key).is_some()
    }

    pub fn is_empty(&self) -> bool {
        self.held.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (u32, u32, u32)> + '_ {
        self.held.iter().map(|(key, ())| key)
    }
}

pub(crate) fn dense(index: usize) -> u32 {
    u32::try_from(index).unwrap_or(NONE)
}

impl<T> Column<T> {
    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut T> {
        self.slots
            .iter_mut()
            .chain(std::iter::once(&mut self.none))
            .filter_map(Option::as_mut)
    }

    pub fn into_values(self) -> impl Iterator<Item = T> {
        self.slots.into_iter().chain([self.none]).flatten()
    }
}

impl<'a, T> Entry<'a, T> {
    pub fn or_insert(self, value: T) -> &'a mut T {
        self.column.or_insert_with(self.id, || value)
    }
}

impl<T> FromIterator<(u32, T)> for Column<T> {
    fn from_iter<I: IntoIterator<Item = (u32, T)>>(pairs: I) -> Self {
        let mut column = Column::new();
        for (id, value) in pairs {
            column.insert(id, value);
        }
        column
    }
}

impl<T> PairColumn<T> {
    pub fn get_mut(&mut self, key: &(u32, u32)) -> Option<&mut T> {
        let row = self.rows.get_mut(&key.0)?;
        let at = row
            .binary_search_by_key(&key.1, |(second, _)| *second)
            .ok()?;
        row.get_mut(at).map(|(_, value)| value)
    }

    pub fn clear(&mut self) {
        self.rows = Column::new();
        self.len = 0;
    }
}

impl PairSet {
    pub fn iter(&self) -> impl Iterator<Item = (u32, u32)> + '_ {
        self.held.iter().map(|(key, ())| key)
    }
}

impl FromIterator<(u32, u32)> for PairSet {
    fn from_iter<I: IntoIterator<Item = (u32, u32)>>(keys: I) -> Self {
        let mut set = PairSet::default();
        for key in keys {
            set.insert(key);
        }
        set
    }
}
