use super::{BoundForm, Extreme, Piecewise, Ratio, slice_transform};

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
