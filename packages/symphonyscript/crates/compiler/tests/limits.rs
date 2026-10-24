mod support;

use std::time::{Duration, Instant};

use symphonyscript::Code;

fn codes(text: &str) -> Vec<Code> {
    support::checked(text)
        .iter()
        .map(|diagnostic| diagnostic.code)
        .collect()
}

#[test]
fn expansion_stops_after_a_million_iterations_and_uses_whatever_the_loop_bounds() {
    let mut chain = String::new();
    for level in 1..25 {
        let next = level + 1;
        chain.push_str(&format!("pattern P{level}():\n    P{next}\n    P{next}\n"));
    }
    chain.push_str("pattern P25():\n    for j in 0..0:\n        Note\nClip c:\n    P1\n");
    let unbounded =
        "Clip c:\n    for i in 0..1000000000000:\n        for j in 0..0:\n            Note\n";
    for text in [unbounded, chain.as_str()] {
        let start = Instant::now();
        let found = codes(text);
        let elapsed = start.elapsed();
        assert_eq!(found, vec![Code::ExpansionLimit], "{text}");
        assert!(elapsed < Duration::from_secs(1), "{elapsed:?}: {text}");
    }
}

#[test]
fn a_million_iterations_and_uses_in_all_are_accepted_and_one_more_is_not() {
    let million = "pattern P():\n    for j in 0..0:\n        Note\nClip c:\n    for i in 0..500000:\n        P\n";
    assert_eq!(codes(million), vec![]);
    let one_more =
        format!("{million}    for k in 0..1:\n        for j in 0..0:\n            Note\n");
    assert_eq!(codes(&one_more), vec![Code::ExpansionLimit]);
}
