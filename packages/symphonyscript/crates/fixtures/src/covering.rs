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
    let mut bound = vec![pairs; tags.len()];
    while threshold > 0 {
        for (index, row) in rows.chunks(width.max(1)).enumerate() {
            if chosen[index] || bound[index] < threshold {
                continue;
            }
            bound[index] = fresh(row, &covered, total);
            if bound[index] < threshold {
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

fn fresh(row: &[usize], covered: &[bool], total: usize) -> usize {
    let mut fresh = 0;
    for (at, first) in row.iter().enumerate() {
        for second in &row[at..] {
            if *first != usize::MAX && *second != usize::MAX && !covered[first * total + second] {
                fresh += 1;
            }
        }
    }
    fresh
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
    covered_where(run, |_| true)
}

pub fn covered_where<T>(run: impl Fn(&Rows) -> T, keep: impl Fn(usize) -> bool) -> T {
    let seen = Mutex::new(Vec::new());
    run(&Rows::Recording(&seen));
    let mut tags: Vec<Vec<usize>> = seen.into_inner().unwrap_or_default();
    tags.sort_unstable();
    tags.dedup();
    let chosen = covering(&tags)
        .into_iter()
        .enumerate()
        .filter(|(at, _)| keep(*at))
        .map(|(_, index)| tags[index].clone())
        .collect();
    run(&Rows::Chosen(Mutex::new(chosen)))
}

#[cfg(test)]
mod tests {
    use super::covering;

    fn covering_by_passes(tags: &[Vec<usize>]) -> Vec<usize> {
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
                let mut missed = 0;
                for (at, first) in row.iter().enumerate() {
                    for second in &row[at..] {
                        if *first == usize::MAX
                            || *second == usize::MAX
                            || covered[first * total + second]
                        {
                            missed += 1;
                        }
                    }
                }
                if chosen[index] || missed > pairs - threshold {
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

    #[test]
    fn the_bounded_covering_chooses_the_rows_the_pass_by_pass_covering_chooses() {
        let mut state: u64 = 0x2545_f491_4f6c_dd1d;
        let mut next = |below: u64| {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            usize::try_from((state >> 33) % below).unwrap_or(0)
        };
        for (count, width, values) in [(0, 0, 1), (1, 3, 2), (40, 4, 3), (300, 6, 5), (900, 9, 4)] {
            let mut tags: Vec<Vec<usize>> = (0..count)
                .map(|_| {
                    let length = width - next(2).min(width);
                    (0..length).map(|_| next(values)).collect()
                })
                .collect();
            tags.sort_unstable();
            tags.dedup();
            assert_eq!(
                covering(&tags),
                covering_by_passes(&tags),
                "{count} {width}"
            );
        }
    }
}
