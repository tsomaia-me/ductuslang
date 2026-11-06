use crate::compiler::Compiler;
use crate::diagnostics::Code;
use crate::syntax::front_end_goldens::source;
use symphonyscript_fixtures::Orders;

const TWIN: &str = "Clip twin:\n    Note/61 when true\n";

fn twins(items: &[&str]) -> Vec<String> {
    let mut all: Vec<&str> = items.to_vec();
    all.push(TWIN);
    Orders::Every
        .of(&all)
        .into_iter()
        .map(|order| order.concat())
        .collect()
}

fn same_as_the_old_engine(rows: &[&[&str]]) -> usize {
    let vocabulary = symphonyscript_fixtures::vocabulary();
    let old = Compiler::new(&vocabulary)
        .expect("the vocabulary compiles")
        .on_the_old_engine();
    let new = Compiler::new(&vocabulary).expect("the vocabulary compiles");
    let mut programs = 0;
    for items in rows {
        for text in twins(items) {
            let sources = [source(&text)];
            let found = new
                .checked_by_new_core(&sources)
                .unwrap_or_else(|| panic!("the new core takes {text}"));
            assert_eq!(found, old.check(&sources), "{text}");
            programs += 1;
        }
    }
    programs
}

const REACTIVE_ARGUMENTS: [&[&str]; 13] = [
    &[
        "state big(1)\n",
        "pattern Gated(default active):\n    Note when active\n",
        "Clip c:\n    Gated/(big > u8(255) + 4)\n    Gated/true\n",
    ],
    &[
        "state big(1)\n",
        "pattern Gated(default active):\n    Note | accent=active\n",
        "Clip c:\n    Gated/(big > u8(255) + 4)\n",
    ],
    &[
        "state big(1)\n",
        "pattern Gated(default active):\n    Note | accent=active\n",
        "Clip c:\n    Gated/(big > 2 / 0)\n",
    ],
    &[
        "state big(1.5)\n",
        "pattern Gated(default active):\n    Note when (active and true)\n",
        "Clip c:\n    Gated/(big > 1.5 + 1e400)\n",
    ],
    &[
        "state big(1)\n",
        "pattern Gated(default active):\n    Note | accent=(not active)\n",
        "Clip c:\n    Gated/(big > [1, 2][5])\n",
    ],
    &[
        "state big(1)\n",
        "pattern Gated(default active):\n    Note\n",
        "Clip c:\n    Gated/(big > u8(255) + 4)\n",
    ],
    &[
        "state big(1)\n",
        "pattern Gated(default active):\n    Note when active\n",
        "Clip c:\n    Gated/(not (big > 1) and (1 + true))\n",
    ],
    &[
        "state big(1)\n",
        "pattern Gated(default active):\n    for i in 0..2:\n        Note when active\n",
        "Clip c:\n    Gated/((big > 1) and [true][3])\n",
    ],
    &[
        "state big(1)\n",
        "pattern Gated(default active):\n    Note when active\n",
        "Clip c:\n    for i in 0..3:\n        Gated/(big > i + u8(255) + 4)\n",
    ],
    &[
        "state big(1)\n",
        "pattern Gated(active):\n    Note when active\n",
        "Clip c:\n    Gated | active=(big > 2 / 0)\n",
    ],
    &[
        "state big(1)\n",
        "pattern Gated(default active):\n    Note/60 when active\n    Note/61 | accent=active\n",
        "Clip c:\n    Gated/(big is 300000000000)\n",
    ],
    &[
        "state big(1.5)\n",
        "pattern P(default x):\n    Note | accent=(x > u8(255) + 4)\n",
        "Clip c:\n    P/(big * 2)\n",
    ],
    &[
        "state big(1)\n",
        "pattern P(default x):\n    Note when x\n",
        "Clip c:\n    P/(big + 1)\n",
    ],
];

#[test]
fn a_pattern_parameter_bound_to_a_reactive_argument_is_typed_as_a_derived_cell_and_draws_what_the_old_engine_draws_at_a_gate_or_an_attribute_beside_a_valid_twin_in_every_order_513()
 {
    assert_eq!(same_as_the_old_engine(&REACTIVE_ARGUMENTS), 312);
}

const CELL_DEFAULTS: [&[&str]; 5] = [
    &[
        "state big(1)\n",
        "pattern G(active = big > u8(255) + 4):\n    Note when active\n",
        "Clip c:\n    G\n    G | active=true\n",
    ],
    &[
        "state big(1)\n",
        "pattern G(active = big > 2 / 0):\n    Note | accent=active\n",
        "Clip c:\n    G\n",
    ],
    &[
        "state big(1)\n",
        "pattern G(active = big > [1, 2][5]):\n    Note/active\n",
        "Clip c:\n    G\n",
    ],
    &[
        "state big(1)\n",
        "pattern G(active = big.past(1, 0) > 1):\n    Note when active\n",
        "Clip c:\n    G\n",
    ],
    &[
        "state big(1)\n",
        "pattern G(default x, active = big > 1):\n    Note/x when active\n",
        "Clip c:\n    G/2\n",
    ],
];

#[test]
fn a_parameters_default_reading_a_cell_is_bound_once_the_cells_are_known_and_draws_at_its_use_what_the_old_engine_draws_beside_a_valid_twin_in_every_order()
 {
    assert_eq!(same_as_the_old_engine(&CELL_DEFAULTS), 120);
}

const WIRE_GATE_PATHS: [&[&str]; 4] = [
    &[
        "Clip a:\n    Plays when to.gain > 0.5: b\n",
        "Clip b | gain=0.7:\n    Note/1\n",
    ],
    &[
        "Clip a:\n    Plays when to.gain: b\n",
        "Clip b:\n    Note/1\n",
    ],
    &[
        "Clip a:\n    Plays when to.gain > 300000000000: b\n",
        "Clip b:\n    Note/1\n",
    ],
    &[
        "Clip a:\n    for i in 0..2:\n        Plays when to.gain > i: b\n",
        "Clip b:\n    Note/1\n",
    ],
];

#[test]
fn a_to_path_in_a_wires_gate_starts_at_the_wires_destination_and_draws_what_the_old_engine_draws_beside_a_valid_twin_in_every_order()
 {
    assert_eq!(same_as_the_old_engine(&WIRE_GATE_PATHS), 24);
}

const FAULTY_DEFAULT_GATES: [&[&str]; 3] = [
    &["let j = [1, 2]\n", "Clip c:\n    Note/when (j[^1])\n"],
    &["Clip c:\n    Note/when 1\n"],
    &["Clip c:\n    Note/when [1, 2][5] > 1\n"],
];

#[test]
fn a_gate_beside_a_faulty_default_is_checked_as_a_bool_drawing_what_the_old_engine_draws_beside_a_valid_twin_in_every_order()
 {
    assert_eq!(same_as_the_old_engine(&FAULTY_DEFAULT_GATES), 10);
}

const ANNOTATED_HISTORY_TARGETS: [(&[&str], &[Code]); 4] = [
    (
        &[
            "state x(1)\n",
            "let l: i32 = x + 1\n",
            "state s = l.past(1, 0)\n",
            "Clip c:\n    Note when s > 3000000000\n",
        ],
        &[Code::DoesNotFit, Code::HistoryTarget, Code::ReactiveLet],
    ),
    (
        &[
            "state x(1)\n",
            "let l: i32 = x + 1\n",
            "Clip c:\n    Note/(l.past(1, 0) + 3000000000)\n",
        ],
        &[Code::DoesNotFit, Code::HistoryTarget, Code::ReactiveLet],
    ),
    (
        &[
            "state x(1)\n",
            "let t: i32 = s * x\n",
            "state s = t.past(1, 0)\n",
            "Clip k:\n    Note/(s)\n",
        ],
        &[Code::HistoryTarget, Code::ReactiveLet],
    ),
    (
        &[
            "let l: i32 = 1\n",
            "state s = l.past(1, 0)\n",
            "Clip c:\n    Note when s\n",
        ],
        &[Code::ConditionType, Code::HistoryTarget],
    ),
];

#[test]
fn a_history_read_of_an_annotated_let_keeps_the_annotated_type_so_its_readers_draw_the_faults_that_type_gives_in_every_order()
 {
    let vocabulary = symphonyscript_fixtures::vocabulary();
    let new = Compiler::new(&vocabulary).expect("the vocabulary compiles");
    let mut programs = 0;
    for (items, codes) in ANNOTATED_HISTORY_TARGETS {
        for text in twins(items) {
            let found = new
                .checked_by_new_core(&[source(&text)])
                .unwrap_or_else(|| panic!("the new core takes {text}"));
            let mut found: Vec<Code> = found
                .iter()
                .map(|diagnostic| diagnostic.code)
                .filter(|code| *code != Code::NoEntryInstance)
                .collect();
            found.sort_by_key(|code| format!("{code:?}"));
            assert_eq!(found, codes, "{text}");
            programs += 1;
        }
    }
    assert_eq!(programs, 288);
}

const UNBOUND_ARGUMENTS: [&[&str]; 9] = [
    &["fn g(): 1\n", "let x = g(x)\n", "Clip c:\n    Note/(x)\n"],
    &["fn g(a): 1\n", "let x = g(x)\n", "Clip c:\n    Note/(x)\n"],
    &[
        "fn g(a): a\n",
        "let x = g(1, x + true)\n",
        "Clip c:\n    Note/(x)\n",
    ],
    &[
        "fn g(a): a\n",
        "let x = g(1, [1, 2][5])\n",
        "Clip c:\n    Note/(x)\n",
    ],
    &[
        "fn g(a): a\n",
        "let x = g(a=1, b=x)\n",
        "Clip c:\n    Note/(x)\n",
    ],
    &[
        "fn g(): 1\n",
        "state s(1)\n",
        "let x = g(s)\nClip c:\n    Note/(x)\n",
    ],
    &[
        "fn g(): 1\n",
        "state v(1)\n",
        "Clip c:\n    Note/(g(v * 1.5))\n",
    ],
    &[
        "fn g(a): a\n",
        "fn h(b): g(b, [1, 2][5] + b)\n",
        "Clip c:\n    Note/(h(1))\n",
    ],
    &[
        "fn g(): 1\n",
        "let y = 2\n",
        "Clip c:\n    Note | velocity=g(y + 1e39)\n",
    ],
];

#[test]
fn an_argument_that_binds_no_parameter_is_no_read_and_draws_nothing_of_its_own_beside_the_calls_fault_510()
 {
    assert_eq!(same_as_the_old_engine(&UNBOUND_ARGUMENTS), 216);
}

const PLAIN_STARTS: [&str; 6] = [
    "Bar b | bar_len=(for i in [1beats, 2beats] into a from 3: a + i)\n",
    "Bar b | bar_len=(for i in [1beats, 2beats] into a from 3: a * 2 + i)\n",
    "Bar b | bar_len=(for i in [1beats, 2beats] into a from 3: a + i + 1)\n",
    "Bar b | bar_len=(for i in [1beats, 2beats] into a from 1000000000000000000: a + i)\n",
    "Bar b | bar_len=(for i in [1beats, 2beats] into a from 3000000000000000000: a + i)\n",
    "Bar b | bar_len=(for i in [1beats, 2beats] into a from 0: a + i + 3000000000000000000)\n",
];

#[test]
fn a_plain_fold_start_is_read_once_in_the_default_unit_and_the_accumulator_in_base_units_drawing_no_plain_in_family_510()
 {
    let vocabulary = symphonyscript_fixtures::bar_vocabulary();
    let new = Compiler::new(&vocabulary).expect("the vocabulary compiles");
    let twin = "Bar t | bar_len=(for i in [1beats] into a from 1beats: a + i)\n";
    let overflows = [false, false, false, false, true, true];
    for (program, overflow) in PLAIN_STARTS.iter().zip(overflows) {
        for order in Orders::Every.of(&[*program, twin]) {
            let text = order.concat();
            let found: Vec<Code> = new
                .checked_by_new_core(&[source(&text)])
                .unwrap_or_else(|| panic!("the new core takes {text}"))
                .iter()
                .map(|diagnostic| diagnostic.code)
                .filter(|code| *code != Code::NoEntryInstance)
                .collect();
            let expected: &[Code] = if overflow { &[Code::DoesNotFit] } else { &[] };
            assert_eq!(found, expected, "{text}");
        }
    }
}

const TOP_GATED_USES: [&[&str]; 4] = [
    &[
        "pattern Bank(default base):\n    Plant:\n        Valve/base Valve/(base + 0.25)\n",
        "Bank manifold /0 when true\n",
    ],
    &[
        "state live(true)\n",
        "pattern Bank(default base):\n    Plant:\n        Valve/base\n",
        "Bank manifold /0 when live\n",
    ],
    &[
        "pattern Bank(default base):\n    Plant:\n        Valve/base\n",
        "Bank manifold /0 when 1\n",
    ],
    &[
        "pattern Bank(default base):\n    Plant:\n        Valve/base\n",
        "Bank manifold /0 when [1][3] > 0\n",
    ],
];

#[test]
fn a_gate_on_a_top_level_pattern_use_is_a_bool_drawing_what_the_old_engine_draws_beside_a_valid_twin_in_every_order()
 {
    assert_eq!(same_as_the_old_engine(&TOP_GATED_USES), 42);
}

const INNER_FITS: [&[&str]; 16] = [
    &[
        "state big(1.5)\n",
        "Clip c:\n    Note | velocity=(big * (2 / 0))\n    Note | velocity=(big * (2 / 1))\n",
    ],
    &["state big(1)\n", "state k = big + 1e39\n"],
    &["state big(1)\n", "state k = big + (u8(255) + 4)\n"],
    &[
        "state big(1.5)\n",
        "state k = big * (2 / 0)\n",
        "Synth s:\n    Osc | freq=k\n",
    ],
    &[
        "state big(1.5)\n",
        "Synth s:\n    Osc | freq=(big * (1e300 * 1e300))\n",
    ],
    &[
        "state big(1.5)\n",
        "Clip c:\n    Note | velocity=((2 / 0) + (1 / 0) * big)\n    Note | velocity=(big * -(2 / 0))\n",
    ],
    &[
        "state big(1.5)\n",
        "Clip c:\n    Note | velocity=(big * (if true then 2 / 0 else 1))\n",
    ],
    &[
        "state big(1.5)\n",
        "pattern G(default x):\n    Note\n",
        "Clip c:\n    G/(big * (2 / 0))\n    G/(big * 1e39)\n    G/(big * 1.5)\n",
    ],
    &[
        "state big(1)\n",
        "pattern G(default x):\n    Note\n",
        "Clip c:\n    G/(big + (u8(255) + 4))\n",
    ],
    &[
        "state big(1)\n",
        "pattern G(default x):\n    Note/x\n",
        "Clip c:\n    G/(big + (u8(255) + 4))\n    G/(big + (2147483647 * 2))\n",
    ],
    &[
        "state big(1)\n",
        "state k = big + i32(1) * 3000000000\n",
        "state j = big + (i32(1) + 2147483647)\n",
    ],
    &["state big(1): i32\n", "state k = big + 3000000000\n"],
    &["state big(1)\n", "state k = big + 10000000000000000000\n"],
    &[
        "state big(1.5)\n",
        "state k = big * 1.5\n",
        "state j = k * 1e39\n",
    ],
    &["state big(1.5)\n", "state k = [big * 1e39, big]\n"],
    &[
        "state big(1.5)\n",
        "state k = big * 1e39\n",
        "Clip c:\n    Note | velocity=k\n",
    ],
];

#[test]
fn a_compile_time_operand_inside_a_reactive_expression_is_checked_in_the_type_it_takes_at_attributes_derived_cells_and_unread_arguments_beside_a_valid_twin_in_every_order_515()
 {
    assert_eq!(same_as_the_old_engine(&INNER_FITS), 222);
}

const UNANNOTATED_HISTORY_TARGETS: [(&[&str], &[Code]); 8] = [
    (
        &[
            "state x(1)\n",
            "let l = x + 1\n",
            "Clip c:\n    Note/(l.past(1, 0) + 3000000000)\n    Note/(l.past(1, 0) + 3)\n",
        ],
        &[
            Code::DoesNotFit,
            Code::HistoryTarget,
            Code::HistoryTarget,
            Code::ReactiveLet,
        ],
    ),
    (
        &[
            "state x(1)\n",
            "let l = x + 1\n",
            "state s = l.past(1, 0)\n",
            "Clip c:\n    Note/(s + 3000000000)\n",
        ],
        &[Code::DoesNotFit, Code::HistoryTarget, Code::ReactiveLet],
    ),
    (
        &[
            "state x(1)\n",
            "let l = x + 1\n",
            "state s = l.past(1, 0)\n",
            "Clip c:\n    Note when s > 3000000000\n",
        ],
        &[Code::HistoryTarget, Code::ReactiveLet],
    ),
    (
        &[
            "state x(1.5)\n",
            "let l = x + 1\n",
            "Clip c:\n    Note/(l.past(1, 0))\n",
        ],
        &[Code::FloatToInteger, Code::HistoryTarget, Code::ReactiveLet],
    ),
    (
        &[
            "let l = 2\n",
            "Clip c:\n    Note/(l.past(1, 0) + 3000000000)\n",
        ],
        &[Code::DoesNotFit, Code::HistoryTarget],
    ),
    (
        &["let l = 2.5\n", "Clip c:\n    Note/(l.past(1, 0))\n"],
        &[Code::FloatToInteger, Code::HistoryTarget],
    ),
    (
        &[
            "let l = [1, 2]\n",
            "Clip c:\n    Note/(l.past(1, [0, 0])[5])\n",
        ],
        &[Code::HistoryTarget, Code::IndexOutOfRange],
    ),
    (
        &[
            "state x(1)\n",
            "let l = x + 1\n",
            "let m = l.past(1, 0) + 1\n",
            "Clip c:\n    Note/(m + 3000000000)\n",
        ],
        &[
            Code::DoesNotFit,
            Code::HistoryTarget,
            Code::ReactiveLet,
            Code::ReactiveLet,
        ],
    ),
];

#[test]
fn a_history_read_of_an_unannotated_let_is_typed_as_the_cell_the_let_repaired_to_a_state_would_be_in_every_order_516()
 {
    let vocabulary = symphonyscript_fixtures::vocabulary();
    let new = Compiler::new(&vocabulary).expect("the vocabulary compiles");
    let mut programs = 0;
    for (items, codes) in UNANNOTATED_HISTORY_TARGETS {
        let mut expected = codes.to_vec();
        expected.sort_by_key(|code| format!("{code:?}"));
        for text in twins(items) {
            let found = new
                .checked_by_new_core(&[source(&text)])
                .unwrap_or_else(|| panic!("the new core takes {text}"));
            let mut found: Vec<Code> = found
                .iter()
                .map(|diagnostic| diagnostic.code)
                .filter(|code| *code != Code::NoEntryInstance)
                .collect();
            found.sort_by_key(|code| format!("{code:?}"));
            assert_eq!(found, expected, "{text}");
            programs += 1;
        }
    }
    assert_eq!(programs, 426);
    for text in twins(&[
        "state x(1)\n",
        "let t = s * x\n",
        "state s = t.past(1, 0)\n",
        "Clip k:\n    Note/(s)\n",
    ]) {
        assert!(
            new.checked_by_new_core(&[source(&text)]).is_none(),
            "{text}"
        );
    }
}
