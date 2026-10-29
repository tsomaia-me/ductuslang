#[derive(Clone, Copy)]
pub enum Orders {
    Every,
    ForwardAndReversed,
}

impl Orders {
    pub fn of<T: Copy>(self, items: &[T]) -> Vec<Vec<T>> {
        match self {
            Orders::Every => permutations(items),
            Orders::ForwardAndReversed => {
                vec![items.to_vec(), items.iter().rev().copied().collect()]
            }
        }
    }
}

fn permutations<T: Copy>(items: &[T]) -> Vec<Vec<T>> {
    let mut found = Vec::new();
    let mut order: Vec<usize> = (0..items.len()).collect();
    let mut counters = vec![0; items.len()];
    found.push(order.iter().map(|index| items[*index]).collect());
    let mut level = 0;
    while level < items.len() {
        if counters[level] < level {
            if level % 2 == 0 {
                order.swap(0, level);
            } else {
                order.swap(counters[level], level);
            }
            found.push(order.iter().map(|index| items[*index]).collect());
            counters[level] += 1;
            level = 0;
        } else {
            counters[level] = 0;
            level += 1;
        }
    }
    found
}
