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

#[test]
fn an_integer_that_would_reach_two_to_the_127_is_reported_before_it_is_computed() {
    let just_under = "170141183460469231731687303715884105727";
    let programs = [
        "Clip c:\n    Note/(2 ** 1000000000)\n".to_string(),
        "let e = 2 ** 2 ** 2 ** 2 ** 2\nClip c:\n    Note/e\n".to_string(),
        format!("Clip c:\n    Note/({just_under} * {just_under})\n"),
    ];
    for text in &programs {
        let start = Instant::now();
        let found = codes(text);
        let elapsed = start.elapsed();
        assert_eq!(found, vec![Code::IntegerOverflow], "{text}");
        assert!(elapsed < Duration::from_secs(1), "{elapsed:?}: {text}");
    }
}

fn doubling(name: &str, levels: usize, left: &str, right: &str) -> String {
    let mut text = format!("input i = 1\nfn {name}0(x): x\n");
    for level in 1..=levels {
        let below = level - 1;
        text.push_str(&format!(
            "fn {name}{level}(x): {name}{below}({left}) + {name}{below}({right})\n"
        ));
    }
    text.push_str(&format!("Clip c:\n    Note/({name}{levels}(i))\n"));
    text
}

#[test]
fn a_compilation_inlines_at_most_a_million_calls() {
    let rows = [
        (doubling("f", 18, "x", "x"), vec![]),
        (doubling("f", 19, "x", "x"), vec![Code::InlineLimit]),
        (doubling("g", 19, "x * 2", "x * 3"), vec![Code::InlineLimit]),
        (doubling("g", 40, "x * 2", "x * 3"), vec![Code::InlineLimit]),
    ];
    for (text, expected) in rows {
        let start = Instant::now();
        let found = codes(&text);
        let elapsed = start.elapsed();
        assert_eq!(found, expected, "{text}");
        assert!(elapsed < Duration::from_secs(10), "{elapsed:?}: {text}");
    }
}
