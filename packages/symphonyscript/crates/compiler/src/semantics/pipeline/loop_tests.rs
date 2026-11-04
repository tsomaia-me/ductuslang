use symphonyscript_core::Vocabulary;

use crate::compiler::Compiler;
use crate::semantics::counted_walk::CountedWalk;
use crate::syntax::front_end_goldens::source;

const VALID_LOOPS: [&str; 9] = [
    "Clip c:\n    for i in 0..3:\n        Note/(60 + i)\n",
    "Clip c:\n    for i in 0..3:\n        Note/(i) | velocity=i\n",
    "let xs = [1, 2, 3]\nClip c:\n    for i, p in xs:\n        Note/(p + i)\n",
    "Clip c:\n    for p in [1.5, 2.5]:\n        Note/1 | velocity=p\n",
    "let n = 3\nClip c:\n    for i in 0..n:\n        Note/i\n",
    "fn f(x): x + 1\nClip c:\n    for i in 0..3:\n        Note/(f(i))\n",
    "Clip c:\n    for i in 0..2:\n        for j in 0..2:\n            Note/(i * 10 + j)\n",
    "Clip c:\n    for x in [[1, 2], [3, 4]]:\n        for y in x:\n            Note/y\n",
    "Clip c:\n    for i in 0..3:\n        Note/1 Note/i\n",
];

const FAULTY_LOOPS: [&str; 21] = [
    "Clip c:\n    for i in 0..3:\n        Note/(i * 3000000000)\n",
    "Clip c:\n    for i in 0..3:\n        Note/(i + true)\n",
    "Clip c:\n    for i in 0..3:\n        Note/1.5\n",
    "Clip c:\n    for p in [1.5, 2.5]:\n        Note/p\n",
    "Clip c:\n    for i in 0..3:\n        Note/([1, 2][i])\n",
    "Clip c:\n    for i in 0..3:\n        Lamp\n",
    "Clip c:\n    for i, j in 0..3:\n        Note/i\n",
    "state s(3)\nClip c:\n    for i in 0..s:\n        Note/i\n",
    "state x([1.5])\nClip c:\n    for p in x:\n        Note/1\n",
    "Clip c:\n    for i in true:\n        Note/1\n",
    "Clip c:\n    for i in 0..2.5:\n        Note/1\n",
    "Clip c:\n    for i in 0..true:\n        Note/1\n",
    "Clip c:\n    for i in [1, 2]..3:\n        Note/1\n",
    "Clip c:\n    for i in 0..(1.5 + true):\n        Note/1\n",
    "Clip c:\n    for i in 0..(1.5 + [1, 2][5]):\n        Note/60\n",
    "Clip c:\n    for i in 0..3:\n        for i in 0..2:\n            Note/i\n",
    "let i = 4\nClip c:\n    for i in 0..3:\n        Note/i\n",
    "Clip c:\n    Note as i\n    for i in 0..3:\n        Note/i\n",
    "Clip c:\n    for i in 0..3:\n        Note/(i)\n    Note/zz\n",
    "Clip c:\n    for i in 0..0:\n        Note | loud=1\n",
    "Clip c:\n    for i in 0..0:\n        for j in 0..3:\n            Zap | loud=1\n",
];

const UNRUN_TWINS: [&str; 4] = [
    "Clip c:\n    for i in 0..0:\n        Note/(i + true)\n",
    "Clip c:\n    for i in 0..0:\n        Note/1 | pitch=2\n",
    "Clip c:\n    for i in 0..0:\n        Clip/1\n",
    "Clip c:\n    for i in 0..0:\n        Note | velocity=(1 + true)\n",
];

fn assert_engines_agree(vocabulary: &Vocabulary, programs: &[&str]) {
    let old = Compiler::new(vocabulary)
        .expect("the vocabulary compiles")
        .on_the_old_engine();
    let new = Compiler::new(vocabulary).expect("the vocabulary compiles");
    for text in programs {
        let sources = [source(text)];
        let found = new
            .checked_by_new_core(&sources)
            .unwrap_or_else(|| panic!("the new core takes {text}"));
        assert_eq!(found, old.check(&sources), "{text}");
    }
}

#[test]
fn a_placement_for_over_a_range_or_an_array_checks_as_the_old_engine_does() {
    let vocabulary = symphonyscript_fixtures::vocabulary();
    assert_engines_agree(&vocabulary, &VALID_LOOPS);
    assert_engines_agree_but_the_entry(&vocabulary, &FAULTY_LOOPS);
}

#[test]
fn the_body_of_a_placement_for_that_runs_no_iteration_has_its_names_resolved_and_nothing_else() {
    let vocabulary = symphonyscript_fixtures::vocabulary();
    let new = Compiler::new(&vocabulary).expect("the vocabulary compiles");
    for text in UNRUN_TWINS {
        let found = new
            .checked_by_new_core(&[source(text)])
            .unwrap_or_else(|| panic!("the new core takes {text}"));
        let codes: Vec<_> = found.iter().map(|diagnostic| diagnostic.code).collect();
        assert_eq!(
            codes,
            vec![crate::diagnostics::Code::NoEntryInstance],
            "{text}"
        );
    }
    assert_engines_agree(&vocabulary, &UNRUN_TWINS);
}

#[test]
fn a_placement_for_of_a_thousand_iterations_types_its_body_once() {
    let vocabulary = symphonyscript_fixtures::vocabulary();
    let new = Compiler::new(&vocabulary).expect("the vocabulary compiles");
    let text = "Clip c:\n    for i in 0..1000:\n        Note/(60 + i)\n";
    let bodies = CountedWalk::CoreLoopBody.steps();
    let iterations = CountedWalk::CoreLoopIteration.steps();
    let found = new
        .checked_by_new_core(&[source(text)])
        .expect("the new core takes the loop");
    assert_eq!(CountedWalk::CoreLoopBody.steps() - bodies, 1);
    assert_eq!(CountedWalk::CoreLoopIteration.steps() - iterations, 1000);
    assert!(
        found
            .iter()
            .all(|diagnostic| diagnostic.code == crate::diagnostics::Code::NoEntryInstance)
    );
    let text = "Clip c:\n    for p in [1, 2.5]:\n        Note/1 | velocity=p\n    for i in 0..3:\n        Note/i\n";
    let bodies = CountedWalk::CoreLoopBody.steps();
    new.checked_by_new_core(&[source(text)])
        .expect("the new core takes the loops");
    assert_eq!(CountedWalk::CoreLoopBody.steps() - bodies, 2);
}

#[test]
fn the_body_of_a_placement_of_an_unknown_kind_draws_each_fault_in_it_at_its_span_481() {
    let bounded = symphonyscript_fixtures::bounded_vocabulary();
    let program = "Root r:\n    Item\n    Link: r\nClip a:\n    Note/1\n";
    let new = Compiler::new(&bounded).expect("the vocabulary compiles");
    let found = new
        .checked_by_new_core(&[source(program)])
        .expect("the new core takes the program");
    let kinds: Vec<_> = found
        .iter()
        .map(|diagnostic| (diagnostic.code, diagnostic.span.range.clone()))
        .collect();
    assert_eq!(
        kinds,
        vec![
            (crate::diagnostics::Code::UnknownKind, 29..33),
            (crate::diagnostics::Code::UnknownKind, 41..45),
        ]
    );
    assert_engines_agree(
        &bounded,
        &[
            program,
            "Root r:\n    Item\n    Link: r\n",
            "Zap a:\n    Blip\n    Lamp\n",
            "Zap:\n    Note/1\n",
            "Clip c:\n    Zap:\n        Blip\n",
        ],
    );
    assert_engines_agree(
        &symphonyscript_fixtures::vocabulary(),
        &[
            "Zap a:\n    Note | loud=1\n",
            "Zap a:\n    Note/(1 + true)\n",
            "Zap a:\n    Lamp\n    Blip\n",
            "Zap a:\n    for i in 0..2:\n        Blip | x=1\n",
            "Clip c:\n    Zap:\n        Note | loud=1\n",
            "let zz = 1\nZap a:\n    zz\n",
        ],
    );
}

const OPEN_FAULTY_ITERABLES: [&str; 11] = [
    "Clip c:\n    for x in zz:\n        Note/x\n",
    "Clip c:\n    for x in (1 + true):\n        Note/x\n",
    "Clip c:\n    for x in [1, 2] + true:\n        Note/x\n",
    "Clip c:\n    for x in zz:\n        Note/(x + 1)\n        Bogus\n",
    "Clip c:\n    for x in zz:\n        Note/(yy)\n",
    "Clip c:\n    for i, x in zz:\n        Note/(x + i)\n",
    "Clip c:\n    for x in [zz, 1]:\n        Note/x\n",
    "Clip c:\n    for x in [1, 2][true..]:\n        Note/x\n",
    "let xs = [1, 2, 3][true..]\nClip c:\n    for x in xs:\n        Note/x\n",
    "Clip c:\n    for x in [1, 2][true..] + [3]:\n        Note/x\n",
    "Clip c:\n    for x in (if 1 then [1] else [2, 3]):\n        Note/x\n",
];

const OPEN_FAULTY_TWINS: [&str; 4] = [
    "Clip c:\n    for x in [1, 2]:\n        Note/x\n",
    "Clip c:\n    for x in [1, 2][1..]:\n        Note/x\n",
    "let xs = [1, 2, 3][1..]\nClip c:\n    for x in xs:\n        Note/x\n",
    "Clip c:\n    for x in (if true then [1] else [2, 3]):\n        Note/x\n",
];

const BOUNDED_OPEN_FAULTY_ITERABLES: [&str; 6] = [
    "Root r:\n    for x in zz:\n        Item\n    Link: r\n",
    "Root r:\n    for x in 0..0:\n        Item\n    Link: r\n",
    "Root r:\n    for x in 0..1:\n        Item\n    Link: r\n",
    "Root r:\n    Item\n    Item\n    Item\n    for x in zz:\n        Item\n    Link: r\n",
    "Root r:\n    for x in [1, 2][true..]:\n        Item\n        Zap\n    Link: r\n",
    "Root r:\n    Item\n    for x in zz:\n        Item/(1 + true)\n    Link: r\n",
];

fn spans(found: &[crate::diagnostics::Diagnostic]) -> Vec<(crate::diagnostics::Code, u32, u32)> {
    found
        .iter()
        .map(|diagnostic| {
            (
                diagnostic.code,
                diagnostic.span.range.start,
                diagnostic.span.range.end,
            )
        })
        .collect()
}

#[test]
fn a_placement_for_whose_iterable_an_error_leaves_of_an_open_length_runs_no_iteration_476() {
    use crate::diagnostics::Code;
    let vocabulary = symphonyscript_fixtures::vocabulary();
    let bounded = symphonyscript_fixtures::bounded_vocabulary();
    assert_engines_agree_but_the_entry(&vocabulary, &OPEN_FAULTY_ITERABLES);
    assert_engines_agree(&vocabulary, &OPEN_FAULTY_TWINS);
    assert_engines_agree(&bounded, &BOUNDED_OPEN_FAULTY_ITERABLES);
    let moved = [
        (
            &vocabulary,
            "Clip c:\n    for x in zz:\n        Note | bogus=1\n",
            vec![
                (Code::UnknownName, 21, 23),
                (Code::UnknownAttribute, 40, 45),
                (Code::NoEntryInstance, 0, 0),
            ],
        ),
        (
            &bounded,
            "Root r:\n    for x in (1 + true):\n        Item | x=1\n    Link: r\n",
            vec![
                (Code::OperandType, 22, 30),
                (Code::UnknownAttribute, 48, 49),
            ],
        ),
    ];
    for (vocabulary, text, expected) in moved {
        let sources = [source(text)];
        let found = Compiler::new(vocabulary)
            .expect("the vocabulary compiles")
            .checked_by_new_core(&sources)
            .unwrap_or_else(|| panic!("the new core takes {text}"));
        let mut found = spans(&found);
        found.sort_unstable();
        let mut wanted = expected.clone();
        wanted.sort_unstable();
        assert_eq!(found, wanted, "{text}");
        let old = Compiler::new(vocabulary)
            .expect("the vocabulary compiles")
            .on_the_old_engine()
            .check(&sources);
        assert_eq!(spans(&old), expected.get(..1).unwrap_or_default(), "{text}");
    }
    let new = Compiler::new(&vocabulary).expect("the vocabulary compiles");
    for text in [
        "Clip c:\n    for x in [1, true]:\n        Note/x\n",
        "let xs = [1, true]\nClip c:\n    for x in xs:\n        Note/x\n",
        "Clip c:\n    for x in [[1], [2]][5]:\n        Note/x\n",
        "Clip c:\n    for x in (if 1 then [1] else [2]):\n        Note/x\n",
    ] {
        assert!(new.checked_by_new_core(&[source(text)]).is_none(), "{text}");
    }
}

const CALLS_IN_HEADS: [&str; 6] = [
    "fn f(x): x + 1\nClip c:\n    for k in 0..f(2):\n        Note/k\n",
    "fn f(x): [x, x + 1]\nClip c:\n    for k in f(2):\n        Note/k\n",
    "fn f(x): x + true\nClip c:\n    for k in 0..f(2):\n        Note/k\n",
    "fn f(x): [x, 1.5]\nClip c:\n    for k in f(2):\n        Note/k\n",
    "fn g(n): for i in 0..n into a from []: a + [i]\nClip c:\n    for k in g(3):\n        Note/k\n",
    "fn f(x): x\nClip c:\n    for k in 0..(f(1) + f(2)):\n        for j in 0..f(k):\n            Note/(j)\n",
];

#[test]
fn a_call_in_a_loop_head_is_inlined_in_source_order_left_to_right() {
    use crate::diagnostics::Code;
    assert_engines_agree_but_the_entry(&symphonyscript_fixtures::vocabulary(), &CALLS_IN_HEADS);
    let limits = symphonyscript_core::LimitOverrides {
        inlined_calls: Some(10),
        inlined_calls_warning: Some(5),
        ..symphonyscript_core::LimitOverrides::default()
    };
    let new = Compiler::with_limits(&symphonyscript_fixtures::vocabulary(), limits)
        .expect("the vocabulary compiles");
    let calls = |count: usize| {
        let sum = vec!["f(0)"; count].join(" + ");
        format!("fn f(x): x\nClip c:\n    for k in 0..({sum}):\n        Note\n")
    };
    let found = new
        .checked_by_new_core(&[source(&calls(7))])
        .expect("the new core takes the loop");
    assert_eq!(
        spans(&found),
        vec![(Code::NoEntryInstance, 0, 0), (Code::LimitWarning, 71, 75)]
    );
    let found = new
        .checked_by_new_core(&[source(&calls(12))])
        .expect("the new core takes the loop");
    assert_eq!(
        spans(&found),
        vec![(Code::LimitWarning, 71, 75), (Code::InlineLimit, 106, 110)]
    );
}

const FLEXIBLE_LOOP_NAMES: [&str; 24] = [
    "Clip c:\n    for e in [2 ** -1, 1]:\n        Note/(e) | brightness=(e)\n",
    "let a = [2 ** -1, 1]\nClip c:\n    for e in a:\n        Note/(e) | brightness=(e)\n",
    "Clip c:\n    for e in [2 ** -1, 1]:\n        Note/(e + 0) | brightness=(e + 0)\n",
    "Clip c:\n    for e in [2 ** 1, 1]:\n        Note/(e) | brightness=(e)\n",
    "Clip c:\n    for e in [1, 2 ** -1]:\n        Note/(e) | brightness=(e)\n",
    "Clip c:\n    for e in [1, 2 ** -1]:\n        Note | brightness=(e)\n        Note/(e)\n",
    "fn f(x): x\nClip c:\n    for e in [2 ** -1]:\n        Note/(f(e)) | brightness=(f(e))\n",
    "Clip c:\n    for i, e in [2 ** -1, 1]:\n        Note/(e) | brightness=(e)\n",
    "Clip c:\n    for e in [[2 ** -1, 1]]:\n        for d in e:\n            Note/(d) | brightness=(d)\n",
    "Clip c:\n    for x in [7 / 2]:\n        Note/x | velocity=x\n",
    "Clip c:\n    for x in [1, 2]:\n        Note/x | velocity=x\n",
    "Clip c:\n    for x in [16777217, 2]:\n        Note/x | velocity=x\n",
    "Clip c:\n    for i, x in [7 / 2, 5 / 2]:\n        Note/(x + i) | velocity=x\n",
    "let xs = [7 / 2]\nClip c:\n    for x in xs:\n        Note/x | velocity=x\n",
    "Clip c:\n    for x in [7 / 2]:\n        Note/1 | velocity=x\n        Note/x\n",
    "Clip c:\n    for x in [1, 7 / 2]:\n        for y in [x]:\n            Note/y | velocity=y\n",
    "fn f(v): v\nClip c:\n    for x in [7 / 2]:\n        Note/(f(x)) | velocity=f(x)\n",
    "let a = 16777217\nClip c:\n    Note/a | velocity=a\n",
    "Clip c:\n    for x in [16777217]:\n        Note/1 | velocity=x\n",
    "Clip c:\n    for x in [2 ** 24 + 1]:\n        Note/x | velocity=x\n",
    "let a = 2 ** 24 + 1\nClip c:\n    Note/a | velocity=a\n",
    "Clip c:\n    for x in [2 ** 24 + 1, 3]:\n        Note/1 | velocity=x\n        Note/x\n",
    "Clip c:\n    for x in [-7 div 2]:\n        Note/x | velocity=x\n",
    "Clip c:\n    for i in 0..3:\n        Note/i | velocity=i\n",
];

#[test]
fn a_flexible_loop_name_read_as_an_integer_and_as_a_float_reads_its_element_in_each_type_4_1() {
    use crate::diagnostics::Code;
    assert_engines_agree(&symphonyscript_fixtures::vocabulary(), &FLEXIBLE_LOOP_NAMES);
    let new =
        Compiler::new(&symphonyscript_fixtures::vocabulary()).expect("the vocabulary compiles");
    let text =
        "Clip c:\n    for e in [1, 2 ** -1]:\n        Note | brightness=(e)\n        Note/(e)\n";
    let found = new
        .checked_by_new_core(&[source(text)])
        .expect("the new core takes the loop");
    let drawn: Vec<_> = found
        .iter()
        .filter(|diagnostic| diagnostic.code == Code::InconsistentUses)
        .map(|diagnostic| {
            (
                diagnostic.span.range.clone(),
                diagnostic
                    .related
                    .iter()
                    .map(|related| (related.span.range.start, related.span.range.end))
                    .collect::<Vec<_>>(),
            )
        })
        .collect();
    assert_eq!(drawn, vec![(79..80, vec![(62, 63)])]);
}

const CELL_READING_HEADS: [&str; 8] = [
    "state s(3)\nClip c:\n    for i in 0..(s + 1):\n        Note/i\n",
    "state s(3)\nlet n = s + 1\nClip c:\n    for i in 0..n:\n        Note/i\n",
    "state s([1, 2])\nlet xs = s\nClip c:\n    for x in xs:\n        Note/x\n",
    "state s([1, 2])\nClip c:\n    for x in s + [3]:\n        Note/x\n",
    "state s(3)\nClip c:\n    for x in [s, 1]:\n        Note/x\n",
    "state s(3)\nfn f(): s\nClip c:\n    for i in 0..f():\n        Note/i\n",
    "state s(3)\nClip c:\n    for i in (s + 1)..4:\n        Note/i\n",
    "state s([1, 2])\nClip c:\n    for x in s[0..1]:\n        Note/x\n",
];

#[test]
fn a_loop_head_reading_a_cell_draws_reactive_iterable_and_one_through_a_reactive_let_draws_that_lets_fault_alone()
 {
    let vocabulary = symphonyscript_fixtures::vocabulary();
    assert_engines_agree_but_the_entry(&vocabulary, &CELL_READING_HEADS);
    assert_engines_agree(
        &vocabulary,
        &[
            "let n = 1 + 1\nClip c:\n    for i in 0..n:\n        Note/i\n",
            "fn f(): 3\nClip c:\n    for i in 0..f():\n        Note/i\n",
            "Clip c:\n    for x in [3, 1][0..1]:\n        Note/x\n",
        ],
    );
}

fn assert_engines_agree_but_the_entry(vocabulary: &Vocabulary, programs: &[&str]) {
    let old = Compiler::new(vocabulary)
        .expect("the vocabulary compiles")
        .on_the_old_engine();
    let new = Compiler::new(vocabulary).expect("the vocabulary compiles");
    let rest = |set: Vec<crate::diagnostics::Diagnostic>| -> Vec<crate::diagnostics::Diagnostic> {
        set.into_iter()
            .filter(|diagnostic| diagnostic.code != crate::diagnostics::Code::NoEntryInstance)
            .collect()
    };
    for text in programs {
        let sources = [source(text)];
        let found = new
            .checked_by_new_core(&sources)
            .unwrap_or_else(|| panic!("the new core takes {text}"));
        let expected = old.check(&sources);
        if found != expected {
            assert!(
                found
                    .iter()
                    .any(|diagnostic| diagnostic.code == crate::diagnostics::Code::NoEntryInstance),
                "{text}"
            );
            assert_eq!(rest(found), rest(expected), "{text}");
        }
    }
}

const NAMED_AND_PATHS_IN_BODIES: [&str; 13] = [
    "Clip c:\n    for i in 0..2:\n        Note as n/i\n",
    "Clip c:\n    for i in 0..0:\n        Note as n/i\n",
    "Clip c:\n    for i in 0..2:\n        Note as n/i Note as m/1\n",
    "Clip c:\n    for i in 0..2:\n        for j in 0..2:\n            Note as n/i\n",
    "Session s:\n    for i in 0..2:\n        Track as t:\n            Piano as p\n",
    "Clip c:\n    Note as n/1\n    for i in 0..2:\n        Note as n/i\n",
    "Clip c | level=0.5:\n    for i in 0..2:\n        Note/i | velocity=c.level\n",
    "Clip c:\n    Note as first/1\n    for i in 0..2:\n        Note/(first.pitch + i)\n",
    "Clip c:\n    Note as first/1\n    for i in 0..2:\n        Note/(first.bogus + i)\n",
    "Clip c:\n    for i in 0..2:\n        Note/(c.zz)\n",
    "Clip d | level=0.25\nClip c:\n    for i in 0..3:\n        Note | velocity=(d.level * i)\n",
    "state s(1.0)\nClip d | level=s\nClip c:\n    for i in 0..3:\n        Note | velocity=d.level\n",
    "Clip c:\n    for i in 0..0:\n        Note/(c.zz)\n",
];

#[test]
fn a_named_placement_in_a_loop_body_draws_named_in_for_and_a_path_there_resolves_per_iteration() {
    assert_engines_agree(
        &symphonyscript_fixtures::vocabulary(),
        &NAMED_AND_PATHS_IN_BODIES,
    );
}

const ANNOTATED_LOOP_NAMES: [&str; 7] = [
    "Clip c:\n    for i: u8 in 0..3:\n        Note/i\n",
    "Clip c:\n    for i: f32 in 0..3:\n        Note | velocity=i\n",
    "Clip c:\n    for i: f32 in 0..3:\n        Note/i\n",
    "Clip c:\n    for x: f32 in [1, 2]:\n        Note | velocity=x\n",
    "Clip c:\n    for i: u8, x: f64 in [1, 2]:\n        Note/i | velocity=x\n",
    "Clip c:\n    for i: u8 in 0..300:\n        Note/i\n",
    "Clip c:\n    for x: i64 in [2 ** -1]:\n        Note | velocity=x\n",
];

#[test]
fn an_annotated_loop_name_fixes_its_type_and_elements_of_another_type_draw_at_the_name_476() {
    use crate::diagnostics::Code;
    let vocabulary = symphonyscript_fixtures::vocabulary();
    assert_engines_agree(&vocabulary, &ANNOTATED_LOOP_NAMES);
    let new = Compiler::new(&vocabulary).expect("the vocabulary compiles");
    for (text, summary) in [
        (
            "Clip c:\n    for x: i32 in [1.5, 2]:\n        Note/x\n",
            "the elements do not have the loop name's annotated type; a conversion states the rounding",
        ),
        (
            "Clip c:\n    for x: bool in [1, 2]:\n        Note/1\n",
            "the elements do not have the loop name's annotated type; a conversion states the rounding",
        ),
        (
            "Clip c:\n    for x: i32 in [[1], [2]]:\n        Note/1\n",
            "the elements' shape differs from the loop name's annotated type",
        ),
    ] {
        let found = new
            .checked_by_new_core(&[source(text)])
            .unwrap_or_else(|| panic!("the new core takes {text}"));
        let drawn: Vec<_> = found
            .iter()
            .filter(|diagnostic| diagnostic.code != Code::NoEntryInstance)
            .map(|diagnostic| {
                (
                    diagnostic.code,
                    diagnostic.span.range.clone(),
                    diagnostic.summary.as_str(),
                )
            })
            .collect();
        assert_eq!(drawn, vec![(Code::TypeMismatch, 16..17, summary)], "{text}");
    }
    for text in [
        "Clip c:\n    for x: u8 in [300, 2]:\n        Note/x\n",
        "Clip c:\n    for x: f32 in [16777217]:\n        Note | velocity=x\n",
    ] {
        assert!(new.checked_by_new_core(&[source(text)]).is_none(), "{text}");
    }
}

const WIRES_IN_BODIES: [&str; 6] = [
    "Session s:\n    Track as t:\n        Piano\n    for i in 0..2:\n        Includes: t\n",
    "Clip a:\n    Note/1\nClip b:\n    Note/2\nTrack t:\n    Piano\n    for i in 0..2:\n        Starts: [a, b][i]\n",
    "Clip a:\n    Note/1\nTrack t:\n    Piano\n    for i in 0..3:\n        Starts | gain=i: a\n",
    "Clip a:\n    Note/1\nTrack t:\n    Piano\n    for i in 0..2:\n        Starts: [a][i]\n",
    "Clip a:\n    Note/1\nTrack t:\n    Piano\n    for i in 0..0:\n        Starts: a\n",
    "Clip a:\n    Note/1\nTrack t:\n    for i in 0..2:\n        Piano:\n            Starts: a\n",
];

#[test]
fn a_wire_in_a_loop_body_is_one_wire_row_per_iteration_with_its_iterations_names() {
    assert_engines_agree(&symphonyscript_fixtures::vocabulary(), &WIRES_IN_BODIES);
}

const CLASHES_IN_BODIES: [&str; 6] = [
    "Track t:\n    for i in 0..2:\n        Synth:\n            Osc as i\n",
    "Clip c:\n    for i in 0..2:\n        Note as i\n",
    "Track t:\n    for i in 0..2:\n        Synth:\n            Osc as j\n",
    "Track t:\n    for i, k in [1, 2]:\n        Synth:\n            Osc as k\n",
    "Track t:\n    for i in 0..2:\n        for j in 0..2:\n            Synth:\n                Osc as i\n",
    "Track t:\n    for i in 0..0:\n        Synth:\n            Osc as i\n",
];

#[test]
fn a_child_named_like_an_enclosing_loop_name_inside_a_loop_body_draws_child_name_clash() {
    assert_engines_agree(&symphonyscript_fixtures::vocabulary(), &CLASHES_IN_BODIES);
}

type Drawn = Vec<(crate::diagnostics::Code, u32, u32)>;

type Moved<'a> = (&'a Vocabulary, &'a str, Drawn, Drawn);

fn assert_moved(moved: &[Moved<'_>]) {
    for (vocabulary, text, expected, kept) in moved {
        let sources = [source(text)];
        let found = Compiler::new(vocabulary)
            .expect("the vocabulary compiles")
            .checked_by_new_core(&sources)
            .unwrap_or_else(|| panic!("the new core takes {text}"));
        let mut found = spans(&found);
        found.sort_unstable();
        let mut wanted = expected.clone();
        wanted.sort_unstable();
        assert_eq!(found, wanted, "{text}");
        let old = Compiler::new(vocabulary)
            .expect("the vocabulary compiles")
            .on_the_old_engine()
            .check(&sources);
        let mut old = spans(&old);
        old.sort_unstable();
        let mut kept = kept.clone();
        kept.sort_unstable();
        assert_eq!(old, kept, "{text}");
    }
}

#[test]
fn a_placement_for_whose_head_draws_its_own_fault_runs_no_iteration_and_resolves_its_body_476() {
    use crate::diagnostics::Code;
    let vocabulary = symphonyscript_fixtures::vocabulary();
    let bounded = symphonyscript_fixtures::bounded_vocabulary();
    assert_moved(&[
        (
            &vocabulary,
            "Clip c:\n    for x in true:\n        Note | bogus=1\n",
            vec![
                (Code::IterableType, 21, 25),
                (Code::UnknownAttribute, 42, 47),
                (Code::NoEntryInstance, 0, 0),
            ],
            vec![(Code::IterableType, 21, 25)],
        ),
        (
            &vocabulary,
            "Clip c:\n    for x in 0..true:\n        Note | bogus=1\n        Note/(1 + true)\n",
            vec![
                (Code::RangeBound, 24, 28),
                (Code::UnknownAttribute, 45, 50),
                (Code::NoEntryInstance, 0, 0),
            ],
            vec![(Code::RangeBound, 24, 28)],
        ),
        (
            &vocabulary,
            "Clip c:\n    for x in 300000000000000000000..2:\n        Note | bogus=1\n",
            vec![
                (Code::RangeBound, 21, 42),
                (Code::UnknownAttribute, 62, 67),
                (Code::NoEntryInstance, 0, 0),
            ],
            vec![(Code::RangeBound, 21, 42)],
        ),
        (
            &vocabulary,
            "state s([1, 2])\nClip c:\n    for x in s:\n        Note | bogus=1\n",
            vec![
                (Code::ReactiveIterable, 37, 38),
                (Code::UnknownAttribute, 55, 60),
                (Code::NoEntryInstance, 0, 0),
            ],
            vec![(Code::ReactiveIterable, 37, 38)],
        ),
        (
            &vocabulary,
            "state s(1)\nlet ys = [s]\nClip c:\n    for x in ys:\n        Note | bogus=1\n",
            vec![
                (Code::ReactiveLet, 15, 17),
                (Code::UnknownAttribute, 64, 69),
                (Code::NoEntryInstance, 0, 0),
            ],
            vec![(Code::ReactiveLet, 15, 17)],
        ),
        (
            &bounded,
            "Root r:\n    for i, x in 0..2:\n        Item | zz=1\n    Link: r\n",
            vec![
                (Code::IterableType, 19, 20),
                (Code::UnknownAttribute, 45, 47),
            ],
            vec![(Code::IterableType, 19, 20)],
        ),
        (
            &bounded,
            "Root r:\n    for x in 0..true:\n        Item\n        Item\n        Item\n    Link: r\n",
            vec![(Code::RangeBound, 24, 28)],
            vec![(Code::RangeBound, 24, 28)],
        ),
    ]);
    assert_engines_agree(
        &vocabulary,
        &[
            "Clip c:\n    for x in [1, 2]:\n        Note | velocity=1\n",
            "Clip c:\n    for x in 0..2:\n        Note | velocity=1\n",
            "state s([1, 2])\nClip c:\n    for x in [1, 2]:\n        Note | velocity=1\n",
        ],
    );
    assert_engines_agree(
        &bounded,
        &[
            "Root r:\n    for i, x in [1, 2]:\n        Item\n    Link: r\n",
            "Root r:\n    for x in 0..2:\n        Item\n    Link: r\n",
        ],
    );
}

#[test]
fn a_placement_for_over_a_slice_whose_bound_is_outside_i64_runs_no_iteration_476() {
    use crate::diagnostics::Code;
    let vocabulary = symphonyscript_fixtures::vocabulary();
    let bounded = symphonyscript_fixtures::bounded_vocabulary();
    assert_moved(&[
        (
            &vocabulary,
            "Clip c:\n    for x in [1, 2, 3][0..300000000000000000000]:\n        Note/(x + true)\n",
            vec![
                (Code::IndexOutOfRange, 34, 55),
                (Code::NoEntryInstance, 0, 0),
            ],
            vec![
                (Code::IndexOutOfRange, 34, 55),
                (Code::NoEntryInstance, 0, 0),
                (Code::OperandType, 72, 80),
            ],
        ),
        (
            &vocabulary,
            "let xs = [1, 2, 3][0..300000000000000000000]\nClip c:\n    for x in xs + [4]:\n        Note/(x + true)\n        Nope\n",
            vec![
                (Code::IndexOutOfRange, 22, 43),
                (Code::UnknownKind, 108, 112),
            ],
            vec![
                (Code::IndexOutOfRange, 22, 43),
                (Code::UnknownKind, 108, 112),
                (Code::OperandType, 90, 98),
            ],
        ),
        (
            &bounded,
            "Root r:\n    Item\n    for x in [1, 2][0..300000000000000000000]:\n        Item\n        Item\n    Link: r\n",
            vec![(Code::IndexOutOfRange, 40, 61)],
            vec![
                (Code::IndexOutOfRange, 40, 61),
                (Code::CountExceeded, 85, 89),
            ],
        ),
    ]);
    assert_engines_agree(
        &vocabulary,
        &[
            "Clip c:\n    for x in [1, 2, 3][0..2]:\n        Note/(x + true)\n",
            "Clip c:\n    for x in [[1, 2][0..300000000000000000000]]:\n        Note/x\n",
        ],
    );
}

const TO_IN_BODIES: [&str; 9] = [
    "Clip as a:\n    for i in 0..2:\n        Plays | gain=to.gain * 2: [b, c][i]\nClip as b | gain=0.5\nClip as c | gain=3\n",
    "Clip as a:\n    for i in 0..2:\n        Plays | gain=to.bogus: [b, c][i]\nClip as b\nClip as c\n",
    "Clip as a:\n    for i in 0..2:\n        Cues | gain=to.gain: [b, d][i]\nClip as b\nLane as d\n",
    "Clip as a:\n    for i in 0..2:\n        Cues | gain=to.gain: [d, b][i]\nClip as b\nLane as d\n",
    "Clip as a:\n    for i in 0..2:\n        Cues | gain=to.gain + true: [d, b][i]\nClip as b\nLane as d\n",
    "Clip as a:\n    for i in 0..3:\n        Cues | gain=to.gain * 3000000000000: [b, d, c][i]\nClip as b\nClip as c | gain=2\nLane as d\n",
    "Clip as a:\n    for d in [b, c]:\n        Plays | gain=to.gain: d\nClip as b | gain=0.5\nClip as c\n",
    "Clip as a:\n    for i in 0..2:\n        Plays as w | gain=to.gain: [b, c][i]\nClip as b\nClip as c\n",
    "Clip as a:\n    for i in 0..2:\n        Plays: [b, c][i]\nClip as b\nClip as c\n",
];

const REFERENCE_ITERABLES: [&str; 4] = [
    "Clip as a:\n    for d in [b, c]:\n        Plays: d\nClip as b\nClip as c\n",
    "Clip as a:\n    for i, d in [b, c]:\n        Plays | gain=i: d\nClip as b\nClip as c\n",
    "let ds = [b, c]\nClip as a:\n    for d in ds:\n        Plays: d\nClip as b\nClip as c\n",
    "Clip as a:\n    for d in [b, e]:\n        Plays: d\nClip as b\nLane as e\n",
];

const UNRUN_WIRES: [&str; 4] = [
    "Clip as a:\n    for i in 0..0:\n        Plays | bogus=1: b\nClip as b\n",
    "Clip as a:\n    for i in 0..0:\n        Plays | gain=1: b\nClip as b\n",
    "Clip as a:\n    for i in 0..0:\n        Plays/1: b\nClip as b\n",
    "Clip as a:\n    for i in 0..0:\n        Plays/(1 + true) | trio=[1, 2]: b\nClip as b\n",
];

const CHOICE_HEADS: [&str; 4] = [
    "state sec(0)\nClip c:\n    for i in (if sec > 0 then [1] else [2]):\n        Note/i\n",
    "state sec(0)\nClip c:\n    for i in 0..(if sec > 0 then 1 else 2):\n        Note/i\n",
    "state sec(0)\nlet k = sec\nClip c:\n    for i in (if true then [k] else [2]):\n        Note/i\n",
    "state sec(0)\nClip c:\n    for i in (if sec > 0 then 1 else 2):\n        Note/i\n",
];

#[test]
fn a_to_path_in_a_loop_body_resolves_per_wire_row_and_reference_and_choice_heads_iterate() {
    let to_arrays = symphonyscript_fixtures::to_array_vocabulary();
    assert_engines_agree(&to_arrays, &TO_IN_BODIES);
    assert_engines_agree(&to_arrays, &REFERENCE_ITERABLES);
    assert_engines_agree(&to_arrays, &UNRUN_WIRES);
    let vocabulary = symphonyscript_fixtures::vocabulary();
    assert_engines_agree_but_the_entry(&vocabulary, &CHOICE_HEADS);
    assert_engines_agree(
        &vocabulary,
        &[
            "Clip c:\n    for i in (if true then [1] else [2]):\n        Note/i\n",
            "Clip c:\n    for i in 0..(if false then 1 else 2):\n        Note/i\n",
        ],
    );
    let new = Compiler::new(&to_arrays).expect("the vocabulary compiles");
    let text = "Clip as a:\n    for i in 0..2:\n        Plays | gain=to.bogus: [b, c][i]\nClip as b\nClip as c\n";
    let found = new
        .checked_by_new_core(&[source(text)])
        .expect("the new core takes the loop");
    let summaries: Vec<&str> = found
        .iter()
        .map(|diagnostic| diagnostic.summary.as_str())
        .collect();
    assert_eq!(
        summaries,
        vec![
            "the destination, the `Clip` named `b`, has no attribute `bogus`",
            "the destination, the `Clip` named `c`, has no attribute `bogus`",
        ]
    );
}

const REACTIVE_FOLDS: [&str; 18] = [
    "state readings([1, 2, 3])\nstate total = 1 + (for i, x in readings into sum from 0: sum + i * x)\n",
    "state r([1, 2])\nlet t = for x in r into s from 0: s + x\n",
    "state s0(1)\nstate t = for i in 0..3 into s from s0: s + i\n",
    "state k(2)\nstate t = for i in 0..3 into s from 0: s + k\n",
    "state r([1, 2])\nstate t = for x in r into s from 0: s + true\n",
    "state r([1, 2])\nstate t = for x in r into s from 0.5: s + x\n",
    "state r([1.5, 2])\nstate t = for x in r into s from 0: s + x\n",
    "state r([1, 2])\nClip c:\n    Note/(for x in r into s from 0: s + x)\n",
    "state r([1, 2])\nfn f(xs): for x in xs into s from 0: s + x\nstate t = f(r)\n",
    "state r([1, 2])\nstate t = for x in r into s from 0: s + (for y in r into u from 0: u + y * x)\n",
    "state r([1, 2])\nstate t = for x in r into s from 0: s + 1.5\n",
    "state r([1, 2])\nstate t = for x in r into s: f32 from 0: s + x\n",
    "state r([1, 2])\nClip c:\n    for i in 0..2:\n        Note/(for x in r into s from i: s + x)\n",
    "state r([1, 2])\nstate t = for x in r into s from 0: if x > 1 then s + x else s\n",
    "state r([true, false])\nstate t = for x in r into s from 0: s + x\n",
    "state r([1, 2])\nstate t = for i, x in r into s from 0: s + i * x + 3000000000\n",
    "state r([1, 2])\nstate t: u8 = for x in r into s from 0: s + x\n",
    "state k(1)\nlet big = for i in 0..3 into s from 0: s + i\nstate t = big + k\n",
];

#[test]
fn a_for_expression_reading_a_cell_is_one_cell_its_body_typed_with_its_slots_and_accumulator_476() {
    let vocabulary = symphonyscript_fixtures::vocabulary();
    assert_engines_agree(&vocabulary, &REACTIVE_FOLDS);
    assert_engines_agree(
        &vocabulary,
        &[
            "state readings(1)\nstate total = 1 + (for i, x in readings into sum from 0: sum + i * x)\n",
        ],
    );
    assert_moved(&[(
        &vocabulary,
        "state readings([]): i32\n\nstate total = 1 + (for i, x in readings into sum from 0: sum + i * x)\n",
        vec![
            (crate::diagnostics::Code::NoEntryInstance, 0, 0),
            (crate::diagnostics::Code::TypeMismatch, 15, 17),
            (crate::diagnostics::Code::IterableType, 56, 64),
        ],
        vec![
            (crate::diagnostics::Code::NoEntryInstance, 0, 0),
            (crate::diagnostics::Code::TypeMismatch, 15, 17),
        ],
    )]);
    let new = Compiler::new(&vocabulary).expect("the vocabulary compiles");
    for text in [
        "state n(3)\nstate t = for i in 0..n into s from 0: s + i\n",
        "state r([1, 2])\nstate t = for x in r into s from []: s + [x]\n",
        "state r([1, 2])\nstate t = for x in r into s: i32 from 0: s + 1.5\n",
    ] {
        assert!(new.checked_by_new_core(&[source(text)]).is_none(), "{text}");
    }
}
