use std::collections::BTreeSet;
use std::sync::Mutex;

pub fn covering(tags: &[Vec<usize>]) -> Vec<usize> {
    let width = tags.iter().map(Vec::len).max().unwrap_or(0);
    let mut slots: Vec<Vec<usize>> = vec![Vec::new(); width];
    for tag in tags {
        for (dimension, value) in tag.iter().enumerate() {
            let column = &mut slots[dimension];
            if column.len() <= *value {
                column.resize(value + 1, usize::MAX);
            }
            column[*value] = 0;
        }
    }
    let mut total = 0;
    for column in &mut slots {
        for slot in column.iter_mut().filter(|slot| **slot == 0) {
            *slot = total;
            total += 1;
        }
    }
    let rows: Vec<usize> = tags
        .iter()
        .flat_map(|tag| {
            let slots = &slots;
            (0..width).map(move |dimension| match tag.get(dimension) {
                Some(value) => slots[dimension][*value],
                None => usize::MAX,
            })
        })
        .collect();
    let mut covered = vec![false; total * total];
    let mut chosen = vec![false; tags.len()];
    let pairs = width * (width + 1) / 2;
    let mut threshold = pairs;
    while threshold > 0 {
        for (index, row) in rows.chunks(width.max(1)).enumerate() {
            if chosen[index] || !fresh_at_least(row, &covered, total, pairs - threshold) {
                continue;
            }
            chosen[index] = true;
            for (at, first) in row.iter().enumerate() {
                for second in &row[at..] {
                    if *first != usize::MAX && *second != usize::MAX {
                        covered[first * total + second] = true;
                    }
                }
            }
        }
        threshold = threshold * 3 / 4;
    }
    (0..tags.len()).filter(|index| chosen[*index]).collect()
}

fn fresh_at_least(row: &[usize], covered: &[bool], total: usize, misses: usize) -> bool {
    let mut missed = 0;
    for (at, first) in row.iter().enumerate() {
        for second in &row[at..] {
            if *first == usize::MAX || *second == usize::MAX || covered[first * total + second] {
                missed += 1;
                if missed > misses {
                    return false;
                }
            }
        }
    }
    true
}

pub enum Rows<'a> {
    Every,
    Recording(&'a Mutex<Vec<Vec<usize>>>),
    Chosen(Mutex<BTreeSet<Vec<usize>>>),
}

impl Rows<'_> {
    pub fn take(&self, tag: &[usize]) -> bool {
        match self {
            Rows::Every => true,
            Rows::Recording(seen) => {
                if let Ok(mut seen) = seen.lock() {
                    seen.push(tag.to_vec());
                }
                false
            }
            Rows::Chosen(unseen) => unseen.lock().is_ok_and(|mut unseen| unseen.remove(tag)),
        }
    }
}

pub fn covered<T>(run: impl Fn(&Rows) -> T) -> T {
    let seen = Mutex::new(Vec::new());
    run(&Rows::Recording(&seen));
    let mut tags: Vec<Vec<usize>> = seen.into_inner().unwrap_or_default();
    tags.sort_unstable();
    tags.dedup();
    let chosen = covering(&tags)
        .into_iter()
        .map(|index| tags[index].clone())
        .collect();
    run(&Rows::Chosen(Mutex::new(chosen)))
}
