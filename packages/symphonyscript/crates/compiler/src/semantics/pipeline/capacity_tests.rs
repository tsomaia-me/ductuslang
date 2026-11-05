use super::bound_form::BoundForm;
use super::extreme::Extreme;
use super::piecewise::Piecewise;
use super::ratio::Ratio;
use super::slice_transform::slice_transform;
use crate::compiler::Compiler;
use crate::diagnostics::Code;
use crate::semantics::counted_walk::CountedWalk;
use crate::syntax::front_end_goldens::source;

type Case = (BoundForm, BoundForm, &'static [(i128, i128)]);

fn at(function: &Piecewise, value: i128) -> i128 {
    function.at(Ratio::int(value)).floor()
}

#[test]
fn a_slice_holds_the_most_elements_any_live_length_up_to_the_capacity_gives() {
    use BoundForm::{Absent, End, Reactive, Start};
    let cases: &[Case] = &[
        (End(8), Absent, &[(0, 0), (3, 3), (8, 8), (100, 8)]),
        (Start(1), Absent, &[(0, 0), (1, 0), (2, 1), (10, 9)]),
        (Absent, Start(3), &[(0, 0), (2, 2), (3, 3), (9, 3)]),
        (Absent, End(2), &[(0, 0), (2, 0), (5, 3)]),
        (Start(1), End(1), &[(0, 0), (2, 0), (3, 1), (10, 8)]),
        (End(5), Start(3), &[(0, 0), (3, 3), (5, 3), (8, 3), (50, 3)]),
        (Start(5), End(3), &[(0, 0), (8, 0), (9, 1), (12, 4)]),
        (End(4), End(1), &[(0, 0), (1, 0), (2, 1), (4, 3), (20, 3)]),
        (
            Reactive(Extreme::Unbounded),
            Reactive(Extreme::Unbounded),
            &[(0, 0), (7, 7)],
        ),
        (
            Reactive(Extreme::Start(2)),
            Reactive(Extreme::Start(4)),
            &[(0, 0), (3, 1), (10, 2)],
        ),
        (
            Absent,
            Reactive(Extreme::Start(4)),
            &[(0, 0), (3, 3), (10, 4)],
        ),
        (
            Reactive(Extreme::End(8)),
            Absent,
            &[(0, 0), (8, 8), (100, 8)],
        ),
        (
            Absent,
            Reactive(Extreme::End(1)),
            &[(0, 0), (1, 0), (10, 9)],
        ),
        (Start(-3), Start(20), &[(0, 0), (10, 10), (25, 20)]),
    ];
    for (lower, upper, expected) in cases {
        let transform = slice_transform(*lower, *upper);
        for (capacity, holds) in *expected {
            assert_eq!(
                at(&transform, *capacity),
                *holds,
                "{lower:?}..{upper:?} at {capacity}"
            );
        }
    }
}

#[test]
fn composing_a_slice_with_a_growing_capacity_keeps_its_breakpoints() {
    let growing = Piecewise::affine(1, 1);
    let kept = Piecewise::compose(
        &slice_transform(BoundForm::End(8), BoundForm::Absent),
        &growing,
    );
    for (t, holds) in [(0, 1), (6, 7), (7, 8), (1000, 8)] {
        assert_eq!(at(&kept, t), holds, "t {t}");
    }
    let run = kept.add(&growing.neg()).zero_run();
    assert_eq!(run, Some(Ratio::int(7)));
}

fn items(text: &str) -> Vec<String> {
    let mut items: Vec<String> = Vec::new();
    for line in text.lines() {
        match items.last_mut() {
            Some(item) if line.starts_with(' ') => {
                item.push('\n');
                item.push_str(line);
            }
            _ => items.push(line.to_string()),
        }
    }
    items
}

fn orders(items: &[String]) -> Vec<String> {
    let mut found = Vec::new();
    let mut order: Vec<usize> = (0..items.len()).collect();
    let mut counts = vec![0usize; items.len()];
    let joined = |order: &[usize]| {
        order
            .iter()
            .filter_map(|at| items.get(*at))
            .fold(String::new(), |text, item| text + item + "\n")
    };
    found.push(joined(&order));
    let mut at = 0;
    while at < order.len() {
        let count = counts.get(at).copied().unwrap_or(0);
        if count < at {
            if at % 2 == 0 {
                order.swap(0, at);
            } else {
                order.swap(count, at);
            }
            found.push(joined(&order));
            if let Some(slot) = counts.get_mut(at) {
                *slot += 1;
            }
            at = 0;
        } else {
            if let Some(slot) = counts.get_mut(at) {
                *slot = 0;
            }
            at += 1;
        }
    }
    found
}

const LENGTH_LIMITS: [(&str, &str, &str); 9] = [
    (
        "state q([]): i32[..#]\nstate r = q + q\nClip c:\n    Note/(r[0])\n",
        "2000000000",
        "1000000000",
    ),
    (
        "state q([]): i32[..#]\nClip c:\n    Note/((q + q)[0])\n",
        "2000000000",
        "1000000000",
    ),
    (
        "state q([]): i32[..#]\nlet r = q + q\nClip c:\n    Note/(r[0])\n",
        "2000000000",
        "1000000000",
    ),
    (
        "state q([]): i32[..#]\nstate r = q + [1]\nstate t = r + r\nClip c:\n    Note/(t[0])\n",
        "2000000000",
        "1000000000",
    ),
    (
        "state q([]): i32[..#]\nstate r = (q + q)[^5..]\nClip c:\n    Note/(r[0])\n",
        "2000000000",
        "1000000000",
    ),
    (
        "state q([]): i32[..#]\nfn twice(xs): xs + xs\nstate r = twice(twice(q))\nClip c:\n    Note/(r[0])\n",
        "600000000",
        "500000000",
    ),
    (
        "state q([]): i32[..#]\nstate v = for x in q into s from []: s + [x, x]\nClip c:\n    Note/(v[0])\n",
        "1500000000",
        "1000000000",
    ),
    (
        "state q([]): i32[..#]\nstate v = for x in q into s from []: s + s + [x]\nClip c:\n    Note/(v[0])\n",
        "100000",
        "20",
    ),
    (
        "state q([]): i32[..#]\nClip c:\n    Note/((for x in q into s from []: s + [x, x])[0])\n",
        "1500000000",
        "1000000000",
    ),
];

#[test]
fn a_capacity_reaching_two_to_the_thirty_one_draws_length_limit_where_it_arises_in_every_order() {
    let vocabulary = symphonyscript_fixtures::vocabulary();
    let old = Compiler::new(&vocabulary)
        .expect("the vocabulary compiles")
        .on_the_old_engine();
    let new = Compiler::new(&vocabulary).expect("the vocabulary compiles");
    let mut programs = 0;
    for (shape, past, within) in LENGTH_LIMITS {
        for (capacity, limited) in [(past, true), (within, false)] {
            for text in orders(&items(&shape.replace('#', capacity))) {
                let sources = [source(&text)];
                let found = new
                    .checked_by_new_core(&sources)
                    .unwrap_or_else(|| panic!("the new core takes {text}"));
                assert_eq!(found, old.check(&sources), "{text}");
                let limits = found
                    .iter()
                    .filter(|diagnostic| diagnostic.code == Code::LengthLimit)
                    .count();
                assert_eq!(limits, usize::from(limited), "{text}");
                programs += 1;
            }
        }
    }
    assert_eq!(programs, 164);
}

#[test]
fn the_capacity_pass_counts_its_walk_linear_in_a_chain_of_joins() {
    let vocabulary = symphonyscript_fixtures::vocabulary();
    let new = Compiler::new(&vocabulary).expect("the vocabulary compiles");
    let mut walked = Vec::new();
    for count in [100usize, 200, 400] {
        let joins = vec!["q"; count].join(" + ");
        let text = format!("state q([]): i32[..4]\nstate r = {joins}\nClip c:\n    Note/(r[0])\n");
        let before = CountedWalk::CoreCapacity.steps();
        let found = new
            .checked_by_new_core(&[source(&text)])
            .unwrap_or_else(|| panic!("the new core takes {text}"));
        assert!(
            found
                .iter()
                .all(|diagnostic| diagnostic.code == Code::NoEntryInstance),
            "{found:?}"
        );
        walked.push(CountedWalk::CoreCapacity.steps() - before);
    }
    assert!(walked.first().is_some_and(|steps| *steps > 0), "{walked:?}");
    for pair in walked.windows(2) {
        if let [smaller, larger] = pair {
            assert!(*larger <= smaller * 5 / 2, "{walked:?}");
        }
    }
}
