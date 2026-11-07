use super::column::{Column, PairColumn, TripleSet};
use crate::semantics::ir::flat::NONE;

#[test]
fn a_column_holds_each_id_once_and_lists_them_in_id_order_whatever_order_they_were_written_in() {
    let mut column = Column::new();
    for id in [40u32, 7, 39, 3, NONE, 1000, 0] {
        assert_eq!(column.insert(id, id / 2), None);
    }
    assert_eq!(column.insert(7, 1), Some(3));
    assert_eq!(column.remove(&3), Some(1));
    assert_eq!(column.remove(&3), None);
    assert_eq!(column.remove(&5000), None);
    let listed: Vec<(u32, u32)> = column.iter().map(|(id, value)| (id, *value)).collect();
    assert_eq!(
        listed,
        [
            (0, 0),
            (7, 1),
            (39, 19),
            (40, 20),
            (1000, 500),
            (NONE, NONE / 2)
        ]
    );
    let mut pairs = PairColumn::default();
    for (first, second) in [(5u32, 9u32), (5, 2), (1, NONE), (5, 7), (1, 0)] {
        pairs.insert((first, second), first.wrapping_add(second));
    }
    *pairs.entry((5, 2)).or_default() += 100;
    let listed: Vec<(u32, u32)> = pairs.iter().map(|(key, _)| key).collect();
    assert_eq!(listed, [(1, 0), (1, NONE), (5, 2), (5, 7), (5, 9)]);
    assert_eq!(pairs.get(&(5, 2)), Some(&107));
    let mut triples = TripleSet::default();
    assert!(triples.insert((2, 1, 0)));
    assert!(!triples.insert((2, 1, 0)));
    assert!(triples.contains(&(2, 1, 0)));
    assert!(triples.remove(&(2, 1, 0)));
    assert!(triples.is_empty());
}
