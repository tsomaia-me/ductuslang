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
    same_in(&symphonyscript_fixtures::vocabulary(), rows)
}

fn same_in(vocabulary: &symphonyscript_core::Vocabulary, rows: &[&[&str]]) -> usize {
    let old = Compiler::new(vocabulary)
        .expect("the vocabulary compiles")
        .on_the_old_engine();
    let new = Compiler::new(vocabulary).expect("the vocabulary compiles");
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

const UNANNOTATED_HISTORY_TARGETS: [(&[&str], &[Code]); 12] = [
    (
        &[
            "state x(1)\n",
            "let t = s * x\n",
            "state s = t.past(1, 0)\n",
            "Clip k:\n    Note/(s)\n",
        ],
        &[Code::HistoryTarget, Code::ReactiveLet],
    ),
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
        &[Code::HistoryTarget],
    ),
    (
        &["let l = 2.5\n", "Clip c:\n    Note/(l.past(1, 0))\n"],
        &[Code::HistoryTarget],
    ),
    (
        &[
            "let l = [1, 2]\n",
            "Clip c:\n    Note/(l.past(1, [0, 0])[5])\n",
        ],
        &[Code::HistoryTarget],
    ),
    (
        &[
            "let q = [1, 2]\n",
            "Clip c:\n    Note/(i32(q.past(1, 0) * 0))\n",
        ],
        &[Code::HistoryTarget],
    ),
    (
        &[
            "let q = [1, 2]\n",
            "state s = (i32(q.past(1, 0) * 0))\n",
            "Clip c:\n    Note/s\n",
        ],
        &[Code::HistoryTarget],
    ),
    (
        &[
            "let q = [1, 2]\n",
            "fn f(): (i32(q.past(1, 0) * 0))\n",
            "Clip c:\n    Note/(f())\n",
        ],
        &[Code::HistoryTarget],
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
fn a_history_read_of_an_unannotated_let_reading_a_cell_is_typed_as_the_cell_the_let_repaired_to_a_state_would_be_and_of_a_compile_time_let_draws_nothing_further_in_every_order_516()
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
    assert_eq!(programs, 600);
}

const FIXED_OPERANDS: [(&[&str], &[Code]); 10] = [
    (
        &["state big(1)\n", "state k = i32(big) + 300000000000\n"],
        &[Code::DoesNotFit],
    ),
    (
        &[
            "state big(1)\n",
            "state k = i32(big) + 300000000000\n",
            "Clip c:\n    Note | velocity=k\n",
        ],
        &[Code::DoesNotFit],
    ),
    (
        &[
            "state big(1)\n",
            "state k = u8(big) + 300\n",
            "Clip c:\n    Note when k > 1\n",
        ],
        &[Code::DoesNotFit],
    ),
    (
        &[
            "state big(1)\n",
            "state k = u8(big) + 300\n",
            "Clip c:\n    Note/k\n",
        ],
        &[],
    ),
    (
        &[
            "state big(1)\n",
            "state k = big + 16777217\n",
            "Clip c:\n    Note | velocity=k\n",
        ],
        &[],
    ),
    (
        &[
            "state big(1)\n",
            "pattern G(default x):\n    Note\n",
            "Clip c:\n    G/(i32(big) + 300000000000)\n",
        ],
        &[Code::DoesNotFit],
    ),
    (
        &[
            "state big(1.5)\n",
            "pattern G(default x):\n    Note | velocity=x\n",
            "Clip c:\n    G/(u8(big) + 300)\n",
        ],
        &[],
    ),
    (
        &[
            "state x(1)\n",
            "state s = i32(x)\n",
            "Clip c:\n    Note when s > 3000000000\n",
        ],
        &[Code::DoesNotFit],
    ),
    (
        &[
            "state q(0): i32\n",
            "Clip c:\n    Note when ((i32(1) div i32(1)) % (300000000000 * [1, 2][1]))\n",
        ],
        &[Code::ConditionType, Code::DoesNotFit],
    ),
    (
        &[
            "state q(0): i32\n",
            "Clip c:\n    Note when ((300000000001 % 1) div (if i32(true) then q else 300000000000))\n",
        ],
        &[
            Code::ConditionType,
            Code::ConditionType,
            Code::ConversionType,
            Code::DoesNotFit,
        ],
    ),
];

#[test]
fn a_fixed_type_operand_gives_its_literals_its_type_in_a_derived_cell_with_no_integer_use_an_unread_argument_a_comparison_and_a_refused_gate_in_every_order()
 {
    let vocabulary = symphonyscript_fixtures::vocabulary();
    let new = Compiler::new(&vocabulary).expect("the vocabulary compiles");
    let mut programs = 0;
    for (items, codes) in FIXED_OPERANDS {
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
    assert_eq!(programs, 186);
}

const COMPILE_TIME_ACCUMULATORS: [(&[&str], &[Code]); 6] = [
    (
        &[
            "let xs = for i in 0..5 into a: i32[..3] from []: a + [i]\n",
            "Clip c:\n    Note/(xs[0])\n",
        ],
        &[Code::CapacityExceeded],
    ),
    (
        &[
            "let xs = for i in 0..2 into a: i32[..3] from []: a + [i]\n",
            "Clip c:\n    Note/(xs[0])\n",
        ],
        &[],
    ),
    (
        &["Clip c:\n    Note/((for i in 0..5 into a: i32[..3] from [1, 2, 3, 4]: a + [i])[0])\n"],
        &[Code::CapacityExceeded],
    ),
    (
        &[
            "let xs = for i in [1, 2, 3, 4, 5] into a: i32[..3] from []: a + [i]\n",
            "let ys = xs\n",
            "Clip c:\n    Note/(ys[0])\n",
        ],
        &[Code::CapacityExceeded],
    ),
    (
        &[
            "let xs = for i in 0..5 into a: i32[..3] from []: (if i > 2 then a else a + [i])\n",
            "Clip c:\n    Note/(xs[4])\n",
        ],
        &[Code::CapacityExceeded],
    ),
    (
        &[
            "let xs = for i in 0..5 into a: i32[..3] from []: a + [i]\n",
            "Clip c:\n    Note/(xs[0] + 3000000000)\n",
        ],
        &[Code::DoesNotFit, Code::CapacityExceeded],
    ),
];

#[test]
fn a_compile_time_for_expression_whose_annotated_accumulator_its_capacity_rule_outgrows_draws_capacity_exceeded_in_every_order_502()
 {
    let vocabulary = symphonyscript_fixtures::vocabulary();
    let new = Compiler::new(&vocabulary).expect("the vocabulary compiles");
    let mut programs = 0;
    for (items, codes) in COMPILE_TIME_ACCUMULATORS {
        for text in twins(items) {
            let mut found: Vec<Code> = new
                .checked_by_new_core(&[source(&text)])
                .unwrap_or_else(|| panic!("the new core takes {text}"))
                .iter()
                .map(|diagnostic| diagnostic.code)
                .filter(|code| *code != Code::NoEntryInstance)
                .collect();
            found.sort_by_key(|code| format!("{code:?}"));
            let mut expected = codes.to_vec();
            expected.sort_by_key(|code| format!("{code:?}"));
            assert_eq!(found, expected, "{text}");
            programs += 1;
        }
    }
    assert_eq!(programs, 50);
}

const NESTED_WHENS: [&[&str]; 10] = [
    &[
        "state live(true)\n",
        "Clip c:\n    for i in 0..3:\n        when live:\n            Note/(60 + i)\n",
    ],
    &[
        "Clip c:\n    for i in 0..3:\n        when i > 1:\n            Note/(60 + i)\n        else:\n            Note/1\n",
    ],
    &[
        "state live(true)\n",
        "pattern P(default x):\n    when live:\n        Note/x\n",
        "Clip c:\n    P/3\n",
    ],
    &[
        "pattern P(default x):\n    when x > 1:\n        Note/x\n    when 1:\n        Note\n",
        "Clip c:\n    P/3\n    P/0\n",
    ],
    &[
        "Clip c:\n    for i in 0..3:\n        when [1, 2][i + 1] > 1:\n            Note/(3000000000)\n",
    ],
    &[
        "state live(true)\n",
        "Clip c:\n    for i in 0..3:\n        when live:\n            Note as n /(60 + i)\n",
    ],
    &[
        "pattern P(default x):\n    when x:\n        Q\n",
        "pattern Q():\n    P/true\n",
        "Clip c:\n    P/true\n",
    ],
    &[
        "state live(true)\n",
        "Clip c:\n    for i in 0..3:\n        when live:\n            Bogus/1\n            for j in 0..i:\n                Note/(j)\n",
    ],
    &["Clip c:\n    for i in 0..3:\n        when i:\n            Note\n"],
    &[
        "state s(1)\n",
        "Clip c:\n    for i in 0..3:\n        when s > i:\n            Note/(i) Note/(i + 1)\n        else:\n            (Note/(i * 2))\n",
    ],
];

#[test]
fn a_when_block_inside_a_placement_for_body_or_a_pattern_body_draws_what_the_old_engine_draws_beside_a_valid_twin_in_every_order()
 {
    assert_eq!(same_as_the_old_engine(&NESTED_WHENS), 84);
}

const NESTED_BUNDLES: [&[&str]; 9] = [
    &[
        "pattern Stab(default root, gap = 3):\n    [Note/root Note/(root + gap)]\n    Note/(root + 12)\nClip c:\n    Stab/55 | gap=5\n    Stab/55 | gap=true\n    Stab/55 | gap=3000000000\n",
    ],
    &[
        "Clip c:\n    for i in 0..3:\n        [Note/(i) Note/(i + 1)]\n        (Note/(i) when i > 1)\n",
    ],
    &[
        "pattern P(default x):\n    [Note/x Note/(x + true)]\n    (Note/x)\n",
        "Clip c:\n    P/1\n    P/2\n",
    ],
    &[
        "state live(true)\n",
        "Clip c:\n    for i in 0..3:\n        when live:\n            [Note/(i) Note/(3000000000)]\n",
    ],
    &[
        "Clip c:\n    for i in 0..3:\n        [Note as n /(i) Note/(i + 1)]\n        (Note as m /(i))\n",
    ],
    &["Clip c:\n    for i in 0..3:\n        [Note/(i) Bogus/1]\n"],
    &["Clip c:\n    for i in 0..0:\n        [Note/(i) (Note/(i) | velocity=true)]\n"],
    &["Rig r:\n    for i in 0..2:\n        [Lamp/0.2 Note/1]\n"],
    &[
        "state lit(0.5)\n",
        "pattern P(default x):\n    [Lamp/x (Lamp/0.5 | hue=lit)]\n",
        "Rig r:\n    P/0.2\n",
    ],
];

#[test]
fn a_bundle_or_a_parenthesized_placement_inside_a_placement_for_body_or_a_pattern_body_draws_what_the_old_engine_draws_beside_a_valid_twin_in_every_order()
 {
    assert_eq!(same_as_the_old_engine(&NESTED_BUNDLES), 48);
}

const NESTED_INLINE_BODIES: [&[&str]; 6] = [
    &["Clip c:\n    for i in 0..3:\n        Note: Note/(i)\n"],
    &[
        "pattern P(default x):\n    Note: Note/x\n",
        "Clip c:\n    P/1\n",
    ],
    &["Clip c:\n    for i in 0..3:\n        Rig: Lamp/(i)\n"],
    &[
        "pattern P(default x):\n    Rig as r: Lamp/x Lamp/(x + true)\n",
        "Rig q:\n    P/1\n",
    ],
    &["Rig r:\n    for i in 0..2:\n        Rig: Lamp/0.2 Note/1\n"],
    &["Rig q:\n    for i in 0..0:\n        Rig: Lamp/(i + 0.5)\n"],
];

#[test]
fn a_placement_body_after_the_colon_inside_a_placement_for_body_or_a_pattern_body_draws_what_the_old_engine_draws_beside_a_valid_twin_in_every_order()
 {
    assert_eq!(same_as_the_old_engine(&NESTED_INLINE_BODIES), 20);
}

const PATTERN_MEMBERS: [&[&str]; 7] = [
    &[
        "pattern P(default x):\n    Lamp/x\n",
        "Rig r:\n    [Lamp/0.2 P/0.5]\n",
    ],
    &[
        "pattern P(default x):\n    Lamp/x\n    Lamp/(x + 0.1)\n",
        "Rig r:\n    [P/0.5 P/0.6]\n    [P/true]\n",
    ],
    &[
        "pattern P(default x):\n    Lamp/x\n    Note/1\n",
        "Rig r:\n    [P/0.5]\n",
    ],
    &[
        "pattern P(default x):\n    for i in 0..2:\n        Lamp/(x + i)\n",
        "Rig r:\n    [P/0.5]\n",
    ],
    &[
        "pattern P(default x):\n    Lamp as l /x\n",
        "Rig r:\n    [P/0.5 P/0.6]\n",
    ],
    &[
        "state s(0.5)\n",
        "pattern P(default x):\n    Lamp/x when x > 0.3\n",
        "Rig r:\n    [P/s P/0.1]\n",
    ],
    &[
        "pattern P(default x):\n    when x > 0.3:\n        Lamp/x\n",
        "Rig r:\n    [P/0.5]\n",
    ],
];

#[test]
fn a_bundle_member_using_a_pattern_that_expands_only_to_node_placements_draws_what_the_old_engine_draws_beside_a_valid_twin_in_every_order()
 {
    assert_eq!(same_as_the_old_engine(&PATTERN_MEMBERS), 60);
}

const FAULTY_BUNDLES: [&[&str]; 7] = [
    &["Rig r:\n    [Lamp/0.2 Lamp/(true + 1)\n"],
    &["Rig r:\n    for i in 0..2:\n        [Lamp/0.2 Lamp/(i + true)\n"],
    &[
        "pattern P(default x):\n    [Lamp/x Lamp/0.3\n",
        "Rig r:\n    P/0.5\n",
    ],
    &["Rig r:\n    [Lamp/0.2 Nope/1\n"],
    &["Rig r:\n    [Lamp/0.2 Lamp/0.3 Lamp/\n"],
    &["Clip c:\n    [Note/1 Note/2\n    Note/3\n"],
    &["Rig r:\n    [Lamp/0.2 Lamp as a /0.3]\n    [Lamp/0.2 Lamp/0.3\n"],
];

#[test]
fn a_bundle_holding_a_syntax_fault_draws_what_the_old_engine_draws_beside_a_valid_twin_in_every_order()
 {
    assert_eq!(same_as_the_old_engine(&FAULTY_BUNDLES), 18);
}

const RULE_WIDTHS: [&[&str]; 14] = [
    &[
        "state big(1.5)\n",
        "state k = f64(big) * 2.0\n",
        "Clip c:\n    Note | velocity=k\n",
    ],
    &[
        "state big(1.5)\n",
        "state k = 2.0 * f64(big)\n",
        "Clip c:\n    Note | velocity=(k + 1.0)\n",
    ],
    &["state big(1.5)\n", "state k = big * 2.0f64 + 1.0\n"],
    &["state big(1)\n", "state k = (big + 1.0) * 2.0f64\n"],
    &[
        "state big(1.5)\n",
        "state k = if big > 1.0 then f64(big) else 2.0\n",
    ],
    &[
        "state big(1.5)\n",
        "state k = [f64(big), 2.0]\n",
        "Clip c:\n    Note | velocity=k[0]\n",
    ],
    &["state big(1.5)\n", "state k = [big, 2.0f64]\n"],
    &[
        "state big(1.5)\nstate k = f64(big) * 2.0\n",
        "state m = k * 2.0\n",
        "Clip c:\n    Note | velocity=m\n",
    ],
    &[
        "state big(1)\n",
        "state k = i64(big) * 2\n",
        "Clip c:\n    Note/(k + 3000000000)\n",
    ],
    &[
        "state big(1)\n",
        "state k = big + i64(2)\n",
        "Clip c:\n    Note/(k)\n",
    ],
    &[
        "state big(1.5)\nstate k = f64(big) * 2.0\n",
        "let m: f64 = 0\n",
        "state j = k + m\n",
    ],
    &[
        "state big(1.5)\nstate k = f64(big) * 2.0\n",
        "state j: f64 = k + 1.0\n",
        "Synth s:\n    Osc | freq=k\n",
    ],
    &[
        "state big(1.5)\nstate k = f64(big) * 2.0\n",
        "Clip c:\n    Note | velocity=(k * 2.0f64)\n",
    ],
    &[
        "state big(1.5)\nstate k = f64(big) * 2.0\n",
        "fn g(x): x\n",
        "Clip c:\n    Note | velocity=g(k)\n",
    ],
];

#[test]
fn a_derived_cells_fixed_operand_wider_than_the_width_its_uses_expect_draws_type_mismatch_at_that_operand_beside_a_valid_twin_in_every_order_517()
 {
    assert_eq!(same_as_the_old_engine(&RULE_WIDTHS), 246);
}

const COMPARED_WIDTHS: [&[&str]; 14] = [
    &[
        "state big(1.5)\n",
        "state k = big * 2.0\n",
        "Clip c:\n    Note when k + 1.0 > 1e39\n",
    ],
    &[
        "state big(1.5)\n",
        "state k = big * 2.0\n",
        "Synth s:\n    Osc | freq=k\n",
        "Clip c:\n    Note when k + 1.0 > 1e39\n",
    ],
    &[
        "state big(1.5)\n",
        "state k = big * 2.0\n",
        "Clip c:\n    Note when 1e39 < k * 2.0\n",
    ],
    &[
        "state big(1.5)\n",
        "state k = big * 2.0\n",
        "Clip c:\n    Note when k + 1.0 > 1.5\n",
    ],
    &[
        "state big(1.5)\n",
        "state k = big * 1e39\n",
        "Clip c:\n    Note when k > 1\n",
    ],
    &[
        "state big(1.5)\n",
        "state k = big * 1e39\n",
        "Clip c:\n    Note when k + 1.0 > 1\n",
    ],
    &[
        "state big(1.5)\n",
        "pattern G(default x):\n    Note when x > 1\n",
        "Clip c:\n    G/(big * 1e39)\n",
    ],
    &[
        "state big(1.5)\n",
        "pattern G(default x):\n    Note when x > 1e39\n",
        "Clip c:\n    G/(big)\n",
    ],
    &[
        "state a(1)\n",
        "state b = a + 0.5\n",
        "Clip c:\n    Note when b > 1e39\n",
    ],
    &[
        "state big(1.5)\nstate k = big * 2.0\n",
        "Synth s:\n    Osc | freq=k\n",
        "Clip c:\n    Note when k > 1e39\n",
    ],
    &[
        "state big(1.5)\nstate k = big * 2.0\n",
        "Clip c:\n    Note when k > 1e39\n    Note | velocity=k\n",
    ],
    &[
        "state big(1.5)\nstate k = big * 1e39\n",
        "fn g(x): x > 1\n",
        "Clip c:\n    Note when g(k)\n",
    ],
    &[
        "state big(1.5)\n",
        "state k = big * 1e39\n",
        "Clip c:\n    Note when k.past(1, 0.0) > 1\n",
    ],
    &[
        "state big(1.5)\n",
        "Clip c:\n    Note when big * 1e39 > 1\n",
    ],
];

#[test]
fn a_derived_float_cell_or_a_reactive_argument_read_only_in_a_comparison_with_a_flexible_operand_takes_the_default_f32_beside_a_valid_twin_in_every_order_518()
 {
    assert_eq!(same_as_the_old_engine(&COMPARED_WIDTHS), 396);
}

fn drawn_codes(rows: &[(&[&str], &[Code])]) -> usize {
    let vocabulary = symphonyscript_fixtures::vocabulary();
    let new = Compiler::new(&vocabulary).expect("the vocabulary compiles");
    let mut programs = 0;
    for (items, codes) in rows {
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
    programs
}

const PLACEHOLDER_GATES: [&[&str]; 10] = [
    &["Clip c:\n    Note when q > 1\n"],
    &[
        "state s(1)\n",
        "Clip c:\n    Note when q + s > 3000000000\n",
    ],
    &["Clip c:\n    Note/60 when nope\n"],
    &["Plant boiler:\n    when nope:\n        Valve/0.3\n"],
    &["Clip c:\n    Note when (u8(if q then true else Wave::Saw))\n"],
    &["Clip c:\n    Note when (u8(if q then 1.5 else 1))\n"],
    &["Clip c:\n    Note when ((300000000000 - 1.5) % (if [1, 2] then q else i32(true)))\n"],
    &[
        "Clip c:\n    Note when (if (1 div i32(true)) then (if [1, 2] then q else q) else (q or [1, 2]))\n",
    ],
    &["Clip c:\n    Note when (if q then 1 else 2)\n"],
    &[
        "state note(60)\n",
        "Clip c:\n    Note when (if q then 300000000000 else [1, 2][5])\n",
    ],
];

#[test]
fn a_gate_holding_an_unknown_name_draws_what_the_whole_program_twins_draw_beside_a_valid_twin_in_every_order()
 {
    assert_eq!(same_as_the_old_engine(&PLACEHOLDER_GATES), 28);
}

const REFUSED_CHOICE_GATES: [(&[&str], &[Code]); 2] = [
    (
        &["Clip c:\n    Note when (if 1 then 1 else 2)\n"],
        &[Code::ConditionType, Code::ConditionType],
    ),
    (
        &["Clip c:\n    Note when (if (1 + true) then 1 else 2)\n"],
        &[Code::ConditionType, Code::OperandType],
    ),
];

#[test]
fn a_non_bool_gate_choice_beside_its_refused_condition_draws_condition_type_at_the_gate_in_every_order()
 {
    assert_eq!(drawn_codes(&REFUSED_CHOICE_GATES), 4);
}

const KEPT_TYPE_GATES: [&[&str]; 4] = [
    &["state q([]): i32[..4]\n", "Clip c:\n    Note when (q[q])\n"],
    &[
        "state q([]): i32[..4]\n",
        "Clip c:\n    Note when ([1, 2][q])\n",
    ],
    &[
        "state q([]): i32[..4]\n",
        "Clip c:\n    Note when (i32([1, 2][q]))\n",
    ],
    &[
        "state q([]): i32[..4]\n",
        "Clip c:\n    Note when ((q[[1, 2][5]..])[if true then q else 1.5])\n",
    ],
];

#[test]
fn a_gate_whose_value_an_error_leaves_of_a_known_non_bool_type_draws_condition_type_beside_a_valid_twin_in_every_order()
 {
    assert_eq!(same_as_the_old_engine(&KEPT_TYPE_GATES), 24);
}

const MEMBERS_WITH_BODIES: [&[&str]; 9] = [
    &[
        "pattern P(default x):\n    Lamp/x:\n        Lamp/0.1\n",
        "Rig r:\n    [P/0.5]\n    [P/0.6 Lamp/0.2]\n    P/0.7\n",
    ],
    &[
        "pattern P(default x):\n    [Lamp/x]\n",
        "Rig r:\n    [P/0.5]\n",
    ],
    &[
        "pattern P(default x):\n    Lamp/x\n    [Lamp/x Lamp/0.2]\n",
        "Rig r:\n    [Lamp/0.3 P/0.5]\n",
    ],
    &[
        "pattern P(default x):\n    Lamp/x\n    Lamp/0.1:\n        Lamp/0.2\n",
        "Rig r:\n    [P/0.5]\n",
    ],
    &[
        "pattern P(default x):\n    Note/1\n    Plays: b\n",
        "Clip a:\n    [P/1]\n",
        "Clip b:\n    Note/2\n",
    ],
    &[
        "pattern Q(default x):\n    Lamp/x:\n        Lamp/0.2\n",
        "pattern P(default x):\n    Q/x\n",
        "Rig r:\n    [P/0.5]\n",
    ],
    &[
        "pattern Q(default x):\n    Lamp/x\n    Lamp/x\n",
        "pattern P(default x):\n    Q/x\n",
        "Rig r:\n    [P/0.5]\n",
    ],
    &[
        "pattern P(default x):\n    for i in 0..2:\n        Lamp/x:\n            Lamp/0.1\n",
        "Rig r:\n    [P/0.5]\n",
    ],
    &[
        "pattern P(default x):\n    Lamp/x\n",
        "Rig r:\n    [P/0.5]\n    Plays: b\n",
        "Clip b:\n    Note/2\n",
    ],
];

#[test]
fn a_bundle_member_whose_pattern_supplies_a_body_a_wire_or_a_bundle_draws_bundle_member_as_the_old_engine_does_beside_a_valid_twin_in_every_order()
 {
    assert_eq!(same_as_the_old_engine(&MEMBERS_WITH_BODIES), 126);
}

const DRAWN_TOPS: [&[&str]; 4] = [
    &["use foo\n", "Clip c:\n    Note/1\n"],
    &["type T = i32\n", "Clip c:\n    Note/1\n"],
    &["type P:\n    x: i32\n", "Clip c:\n    Note/1\n"],
    &["publish foo\n", "Clip c:\n    Note/(foo)\n"],
];

#[test]
fn a_top_item_the_front_end_draws_or_skips_draws_what_the_old_engine_draws_beside_a_valid_twin_in_every_order()
 {
    assert_eq!(same_as_the_old_engine(&DRAWN_TOPS), 24);
}

const NAMES_THAT_ARE_NO_VALUES: [&[&str]; 9] = [
    &["let x = Lamp\n", "Rig r:\n    Lamp/0.1\n"],
    &["Rig r:\n    Lamp/(Lamp)\n"],
    &["Clip c:\n    Note/(sin)\n"],
    &["Synth s:\n    Osc | freq=(rand)\n"],
    &["Synth s:\n    Osc | freq=Wave\n"],
    &["Clip a:\n    Note/(Plays)\n"],
    &["Clip c:\n    Note/(sin + 1.5)\n"],
    &["state s(Lamp)\n", "Clip c:\n    Note/(s)\n"],
    &["Clip c:\n    Note when Lamp\n    Note/([Lamp, 2][5])\n"],
];

#[test]
fn a_vocabulary_name_standing_as_a_value_draws_not_a_value_beside_a_valid_twin_in_every_order() {
    assert_eq!(same_as_the_old_engine(&NAMES_THAT_ARE_NO_VALUES), 26);
    assert_eq!(
        drawn_codes(&[(
            &["Clip c:\n    Note/(Lamp + 300000000000)\n"],
            &[Code::NotAValue]
        )]),
        2
    );
}

const PI_READS: [&[&str]; 11] = [
    &["Synth s:\n    Osc | freq=pi\n"],
    &["Synth s:\n    Osc | freq=(pi * 2.0)\n"],
    &["Clip c:\n    Note/(pi)\n"],
    &["Clip c:\n    Note | velocity=pi\n"],
    &["let x = pi * 100\n", "Clip c:\n    Note | velocity=x\n"],
    &["Clip c:\n    Note when pi > 3\n"],
    &["Clip c:\n    Note/(i32(pi))\n"],
    &["state s(pi)\n", "Clip c:\n    Note | velocity=s\n"],
    &["Clip c:\n    Note/(pi + 300000000000)\n"],
    &["fn f(x): x * pi\n", "Synth s:\n    Osc | freq=f(2)\n"],
    &["Clip c:\n    Note when pi\n"],
];

#[test]
fn pi_is_a_flexible_float_drawing_what_the_old_engine_draws_beside_a_valid_twin_in_every_order() {
    assert_eq!(same_as_the_old_engine(&PI_READS), 34);
}

const STRING_ATTRIBUTES: [&[&str]; 12] = [
    &["Track t | label=1\n"],
    &["Track t | label=true\n"],
    &["Track t | label=1.5\n"],
    &["state s(1)\n", "Track t | label=s\n"],
    &["Track t | label=[1, 2]\n"],
    &["Track t | label=(1 + true)\n"],
    &["Kit k:\n    Sampler/3\n"],
    &["Track t | label=Wave::Saw\n"],
    &["Track t\n"],
    &["Kit k:\n    Sampler\n"],
    &["fn f(x): x\n", "Track t | label=f(1)\n"],
    &["Track t | label=(if true then 1 else 2)\n"],
];

#[test]
fn a_value_that_is_no_string_at_a_string_attribute_draws_what_the_old_engine_draws_beside_a_valid_twin_in_every_order()
 {
    assert_eq!(same_as_the_old_engine(&STRING_ATTRIBUTES), 32);
}

const INSTANCE_NAMES_AS_VALUES: [&[&str]; 9] = [
    &["Clip a:\n    Note/1\nClip c:\n    Note/(a)\n"],
    &["let d = a\nClip a:\n    Note/1\nClip c:\n    Note/(d)\n"],
    &["pattern P(default v):\n    Note/(v)\nClip c:\n    P/d\nClip d\n"],
    &["Clip c:\n    Note/(c)\n"],
    &["let x = c\n", "Clip c:\n    Note/(x)\n"],
    &["Clip c:\n    Note when c\n"],
    &["Clip c:\n    Note | velocity=c\n"],
    &["Synth s:\n    Osc | wave=s\n"],
    &[
        "Clip c:\n    Note/1\n",
        "Clip d:\n    Plays: c\n    Note/(c)\n",
    ],
];

#[test]
fn an_instance_name_standing_as_a_whole_attribute_value_or_gate_draws_what_the_old_engine_draws_beside_a_valid_twin_in_every_order()
 {
    assert_eq!(same_as_the_old_engine(&INSTANCE_NAMES_AS_VALUES), 26);
}

const REFERENCE_OPERANDS: [&[&str]; 16] = [
    &["Clip a:\n    Note/1\nClip c:\n    Note/(a + 1)\n"],
    &["let q = c\nClip c:\n    Note/(if true then q else 1.5)\n"],
    &["Clip a:\n    Note as h /1\nClip c | level=a.h:\n    Note/3\n"],
    &["Clip c:\n    Note/(c + 1)\n"],
    &["Clip c:\n    Note/(-c)\n"],
    &["Clip c:\n    Note/(i32(c))\n"],
    &["Clip c:\n    Note/(if true then c else 1)\n"],
    &["Clip c:\n    Note/([c][0])\n"],
    &["Clip c:\n    Note/(c > 1)\n"],
    &["Clip c:\n    Note/(not c)\n"],
    &["Clip c:\n    Note when (c and true)\n"],
    &["Clip c:\n    Note when (true or c)\n"],
    &["Clip c:\n    Note when c is c\n"],
    &["Clip c:\n    Note | velocity=(c * 2.0)\n"],
    &["Clip c:\n    Note/([c, c])\n    Note/([c, 1])\n"],
    &["let x = c + 1\n", "Clip c:\n    Note/(x)\n"],
];

#[test]
fn an_instance_name_as_an_operand_draws_the_operators_refusal_as_the_old_engine_does_beside_a_valid_twin_in_every_order()
 {
    assert_eq!(same_as_the_old_engine(&REFERENCE_OPERANDS), 36);
    assert_eq!(
        drawn_codes(&[(
            &["let q = c\nClip c:\n    Note | velocity=(if 1 then true else q)\n"],
            &[Code::ConditionType, Code::TypeMismatch]
        )]),
        2
    );
}

#[test]
fn an_annotated_let_holding_an_instance_name_refuses_it_and_keeps_its_type_in_every_order() {
    assert_eq!(
        same_as_the_old_engine(&[&[
            "let d: i32 = a\n",
            "Clip a:\n    Note/1\n",
            "Clip c:\n    Note/(d)\n"
        ]]),
        24
    );
    assert_eq!(
        drawn_codes(&[(
            &[
                "let d: i32 = a\n",
                "Clip a:\n    Note/1\n",
                "Clip c:\n    Note/3\n    Plays: d\n"
            ],
            &[Code::DestinationType, Code::TypeMismatch]
        )]),
        24
    );
}

const FAULTY_LINE_INSTANCES: [&[&str]; 7] = [
    &["xlet a = checksum(\"GET\")\n", "Clip c:\n    Note/a\n"],
    &[
        "state1 demand(0.5)\n",
        "Plant boiler:\n    Valve | open=demand * 0.8\n",
    ],
    &["xlet a = 1\n", "Clip c:\n    Note when a\n"],
    &["xlet a = 1\n", "Clip c:\n    Note/(a[0])\n"],
    &["Clip a:\n    Note/a\n"],
    &["Clip a:\n    Note when a\n"],
    &["Clip a:\n    Note/(a[0])\n"],
];

#[test]
fn an_instance_name_declared_on_a_line_holding_a_syntax_fault_is_a_placeholder_at_every_use_beside_a_valid_twin_in_every_order()
 {
    assert_eq!(same_as_the_old_engine(&FAULTY_LINE_INSTANCES), 30);
}

const BUNDLE_MEMBER_NAMES: [&[&str]; 4] = [
    &[
        "Rig stage:\n    Scene as dusk | hold=8:\n        Cue/0\n        Dims: wash\n    [Lamp as wash]\n",
    ],
    &[
        "Rig stage:\n    [Lamp as wash]\n    Scene as dusk | hold=8:\n        Cue/0\n        Dims: wash\n",
    ],
    &["Rig stage:\n    Scene as dusk | hold=8:\n        Dims: wash\n    [Lamp as wash Lamp]\n"],
    &[
        "Rig stage:\n    Scene as dusk | hold=8:\n        Cue/0 Cue/2\n        Dims: wash\n    [Lamp as wash\n    Lamp\n",
        "Scene night:\n    Cue/0\n    Triggers: stage.dusk\n",
    ],
];

#[test]
fn a_bundle_members_as_name_on_a_line_is_in_scope_for_its_siblings_bodies_beside_a_valid_twin_in_every_order()
 {
    assert_eq!(same_as_the_old_engine(&BUNDLE_MEMBER_NAMES), 12);
}

const FOLDS_IN_BODIES: [(&[&str], &[Code]); 6] = [
    (
        &[
            "fn f(n): for i in 0..n into a: i32[..3] from []: a + [i]\n",
            "Clip c:\n    Note/(f(5)[0])\n",
        ],
        &[Code::CapacityExceeded],
    ),
    (
        &[
            "fn f(n): for i in 0..n into a: i32[..3] from []: a + [i]\n",
            "Clip c:\n    Note/(f(2)[0])\n",
        ],
        &[],
    ),
    (
        &[
            "fn f(n): for i in 0..n into a: i32[..3] from []: a + [i]\n",
            "Clip c:\n    Note/(f(2)[0])\n    Note/(f(4)[0])\n    Note/(f(6)[0])\n",
        ],
        &[Code::CapacityExceeded, Code::CapacityExceeded],
    ),
    (
        &[
            "fn f(n): for i in 0..(n + 1) into a: i32[..3] from []: a + [i]\n",
            "fn g(m): f(m)\n",
            "Clip c:\n    Note/(g(3)[0])\n",
        ],
        &[Code::CapacityExceeded],
    ),
    (
        &[
            "fn f(n): for i in 0..n into a: i32[..3] from []: a + [i]\n",
            "fn g(m): f(m)[0] + f(m + 1)[0]\n",
            "Clip c:\n    Note/(g(2))\n    Note/(g(3))\n",
        ],
        &[Code::CapacityExceeded],
    ),
    (
        &[
            "fn f(n):\n    let k = n + 2\n    for i in 0..k into a: i32[..3] from []: a + [i]\n",
            "fn g(m): f(m)[0]\n",
            "Clip c:\n    Note/(g(2))\n    Note/(g(1))\n",
        ],
        &[Code::CapacityExceeded],
    ),
];

#[test]
fn a_compile_time_fold_in_a_function_body_draws_capacity_exceeded_per_call_with_its_arguments_bound_in_every_order_502()
 {
    assert_eq!(drawn_codes(&FOLDS_IN_BODIES), 90);
}

const NOT_VALUES_IN_BODIES: [&[&str]; 5] = [
    &["fn lowest(i): min + i\n", "Clip c:\n    Note/(lowest(0))\n"],
    &[
        "fn lowest(i): min + i\n",
        "Clip c:\n    Note/(lowest(0))\n    Note/(lowest(1))\n",
    ],
    &["fn lowest(i): min + i\n", "Clip c:\n    Note/(1)\n"],
    &[
        "state held([1, 2, 3])\n",
        "fn lowest(i):\n    let a = held[i]\n    if i >= 3 then 127 else min)(a, lowest(i + 1))\n",
        "Clip c:\n    Note/(lowest(0))\n",
    ],
    &[
        "fn lowest(i): Lamp + i\n",
        "fn twice(i): lowest(i) + lowest(i + 1)\n",
        "Clip c:\n    Note/(twice(0))\n",
    ],
];

#[test]
fn a_name_that_is_no_value_in_a_function_body_draws_not_a_value_at_each_call_beside_a_valid_twin_in_every_order()
 {
    assert_eq!(same_as_the_old_engine(&NOT_VALUES_IN_BODIES), 66);
}

const LET_CYCLES: [(&[&str], &[Code]); 5] = [
    (
        &[
            "state x(1.5)\n",
            "let t = s * x\n",
            "state s = t.past(1, 0)\n",
            "Clip k:\n    Note/(s)\n",
        ],
        &[Code::FloatToInteger, Code::HistoryTarget, Code::ReactiveLet],
    ),
    (
        &[
            "state x(1)\n",
            "let t = s * x\n",
            "state s = t.past(1, 0)\n",
            "Clip k:\n    Note/(s)\n",
        ],
        &[Code::HistoryTarget, Code::ReactiveLet],
    ),
    (
        &[
            "state x(1)\n",
            "let t = s + 3000000000\n",
            "state s = t.past(1, 0)\n",
            "Clip k:\n    Note/(s + 3000000000)\n",
        ],
        &[
            Code::DoesNotFit,
            Code::DoesNotFit,
            Code::HistoryTarget,
            Code::ReactiveLet,
        ],
    ),
    (
        &[
            "state x(1)\n",
            "let t = s + 3000000000\n",
            "state s = t.past(1, 0)\n",
            "Clip k:\n    Note/(s)\n",
        ],
        &[Code::DoesNotFit, Code::HistoryTarget, Code::ReactiveLet],
    ),
    (
        &[
            "state x(1)\n",
            "let t = s + 3\n",
            "state s = t.past(1, 0)\n",
            "Clip k:\n    Note/(s)\n",
        ],
        &[Code::HistoryTarget, Code::ReactiveLet],
    ),
];

#[test]
fn a_cell_on_a_cycle_through_a_lets_history_takes_the_type_the_repaired_cell_settles_to_in_every_order_519()
 {
    assert_eq!(drawn_codes(&LET_CYCLES), 600);
}

const CELLS_HOLDING_REFERENCES: [&[&str]; 4] = [
    &[
        "Clip a:\n    Note/1\n",
        "state s(a)\n",
        "Clip c:\n    Note/(s)\n",
    ],
    &[
        "Clip a:\n    Note/1\n",
        "state s = a\n",
        "Clip c:\n    Note/(s)\n",
    ],
    &[
        "Clip a:\n    Note/1\n",
        "state s(a)\n",
        "Clip c:\n    Note/(s + 1)\n    Note when s\n",
    ],
    &[
        "Clip a:\n    Note/1\n",
        "state s = a\n",
        "state u = s + 1\n",
        "Clip c:\n    Note/(u)\n",
    ],
];

#[test]
fn a_cell_whose_init_or_rule_is_an_instance_name_draws_state_value_and_its_readers_nothing_beside_a_valid_twin_in_every_order()
 {
    assert_eq!(same_as_the_old_engine(&CELLS_HOLDING_REFERENCES), 192);
}

const ANNOTATED_CALLS: [&[&str]; 7] = [
    &[
        "fn f(n): n * 100\n",
        "let x: u8 = f(3)\n",
        "Clip c:\n    Note/(x)\n",
    ],
    &[
        "fn f(n): n * 100\n",
        "let x: u8 = f(1)\n",
        "Clip c:\n    Note/(x)\n",
    ],
    &[
        "fn f(n): n + 0.5\n",
        "let x: i32 = f(1)\n",
        "Clip c:\n    Note/(x)\n",
    ],
    &[
        "fn f(n): [n, n]\n",
        "let x: i32[2] = f(1)\n",
        "Clip c:\n    Note/(x[0])\n",
    ],
    &[
        "fn f(n): n\n",
        "let x: u8 = f(1) + 300\n",
        "Clip c:\n    Note/(x)\n",
    ],
    &[
        "fn f(n): true\n",
        "let x: i32 = f(1)\n",
        "Clip c:\n    Note/(x)\n",
    ],
    &[
        "fn g(n): n + 0.5\n",
        "fn f(n): g(n)\n",
        "let x: i32 = f(1)\n",
        "Clip c:\n    Note/(x)\n",
    ],
];

#[test]
fn an_annotated_let_whose_init_is_a_call_checks_the_body_result_against_the_annotation_beside_a_valid_twin_in_every_order()
 {
    assert_eq!(same_as_the_old_engine(&ANNOTATED_CALLS), 264);
}

const REFERENCES_BESIDE_ARRAYS: [&[&str]; 4] = [
    &["Clip c:\n    Note/(i32(q / [1, 2]))\n", "let q = c\n"],
    &["Clip c:\n    Note/(u8([1, 2] and q))\n", "let q = c\n"],
    &["Clip c:\n    Note/(q / [1, 2])\n", "let q = c\n"],
    &["Clip c:\n    Note when q < [1, 2]\n", "let q = c\n"],
];

#[test]
fn a_reference_operand_beside_an_array_operand_draws_array_operand_beside_a_valid_twin_in_every_order()
 {
    assert_eq!(same_as_the_old_engine(&REFERENCES_BESIDE_ARRAYS), 24);
}

const FOLD_NAMES_CLASHING: [&[&str]; 3] = [
    &[
        "fn g(): 1\n",
        "let y = for g in [1] into a from 0: a + g\n",
        "Clip c:\n    Note/(y)\n",
    ],
    &["Clip c:\n    Note/(for c in [1] into a from 0: a + c)\n"],
    &[
        "Clip k:\n    Note/1\n",
        "let y = for i in [1] into k from 0: k + i\n",
        "Clip c:\n    Note/(y)\n",
    ],
];

#[test]
fn a_fold_name_equal_to_a_function_or_an_instance_draws_for_variable_clash_beside_a_valid_twin_in_every_order()
 {
    assert_eq!(same_as_the_old_engine(&FOLD_NAMES_CLASHING), 50);
}

const VOCABULARY_TYPE_OPERANDS: [&[&str]; 5] = [
    &["state s(1)\n", "Clip c:\n    Note when s is Wave\n"],
    &["state s(1)\n", "Clip c:\n    Note when s is Lamp\n"],
    &["Clip c:\n    Note when (1 + true) is Wave\n"],
    &["Clip c:\n    Note/(if 3 is not Lamp then 1 else 2)\n"],
    &["state s(Wave::Saw)\n", "Clip c:\n    Note when s is Wave\n"],
];

#[test]
fn a_vocabulary_type_after_is_draws_unbuilt_construct_and_nothing_of_its_left_operand_beside_a_valid_twin_in_every_order()
 {
    assert_eq!(same_as_the_old_engine(&VOCABULARY_TYPE_OPERANDS), 22);
}

const FOLD_NAMES_CLASHING_WITH_PATTERNS: [&[&str]; 2] = [
    &[
        "pattern P():\n    Note/1\n",
        "let y = for P in [1] into a from 0: a + P\n",
        "Clip c:\n    P\n    Note/(y)\n",
    ],
    &[
        "let k = 1\n",
        "let k = 2\n",
        "let y = for k in [1] into a from 0: a + k\n",
        "Clip c:\n    Note/(y)\n",
    ],
];

#[test]
fn a_fold_name_equal_to_a_pattern_draws_for_variable_clash_beside_a_valid_twin_in_every_order() {
    assert_eq!(
        same_as_the_old_engine(&FOLD_NAMES_CLASHING_WITH_PATTERNS),
        144
    );
}

const REFERENCE_ELEMENT_READS: [&[&str]; 8] = [
    &["Clip c:\n    Note/1\n", "Clip k:\n    Note/([c, c][5])\n"],
    &["Clip c:\n    Note/1\n", "Clip k:\n    Note/([c, c][1])\n"],
    &["Clip c:\n    Note/1\n", "Clip k:\n    Note/([1, 2][5])\n"],
    &[
        "Clip c:\n    Note/1\n",
        "Clip k:\n    Note when ([c, c][5])\n",
    ],
    &[
        "Clip c:\n    Note/1\n",
        "Clip k:\n    Note when ([c, c][0])\n",
    ],
    &[
        "Clip c:\n    Note/1\n",
        "let q = c\n",
        "Clip k:\n    Note/([q, q][5])\n",
    ],
    &[
        "Clip c:\n    Note/1\n",
        "Clip k:\n    Note/(1 + [c, c][5])\n",
    ],
    &[
        "Clip c:\n    Note/1\n",
        "Clip k:\n    Note/(i32([c, c][5]))\n",
    ],
];

#[test]
fn a_reference_arrays_element_read_outside_it_draws_index_out_of_range_beside_its_type_fault_beside_a_valid_twin_in_every_order()
 {
    assert_eq!(same_as_the_old_engine(&REFERENCE_ELEMENT_READS), 66);
}

const REFERENCE_OPERATORS: [&[&str]; 10] = [
    &["Clip c:\n    Note/1\n", "Clip k:\n    Note/(c or c)\n"],
    &["Clip c:\n    Note/1\n", "Clip k:\n    Note/(c or true)\n"],
    &[
        "Clip c:\n    Note/1\n",
        "Clip k:\n    Note/(true or true)\n",
    ],
    &[
        "Clip c:\n    Note/1\n",
        "let q = c\n",
        "Clip k:\n    Note/(q or 1)\n",
    ],
    &["Clip c:\n    Note/1\n", "Clip k:\n    Note/(c - 1.5)\n"],
    &["Clip c:\n    Note/1\n", "Clip k:\n    Note/(c - true)\n"],
    &["Clip c:\n    Note/1\n", "Clip k:\n    Note/(c + [1, 2])\n"],
    &[
        "Clip c:\n    Note/1\n",
        "Clip k:\n    Note/(u8(c + [1, 2]))\n",
    ],
    &["Clip c:\n    Note/1\n", "Clip k:\n    Note when (u8(c))\n"],
    &[
        "Clip c:\n    Note/1\n",
        "Clip k:\n    Note when (c and c)\n",
    ],
];

#[test]
fn a_reference_operand_draws_what_a_number_in_its_place_draws_at_and_or_a_join_and_a_conversion_beside_a_valid_twin_in_every_order()
 {
    assert_eq!(same_as_the_old_engine(&REFERENCE_OPERATORS), 78);
}

const CLASHES_ACROSS_FILES: [(&str, &str); 4] = [
    (
        "fn f(x): x + 1\n",
        "let y = for f in [1, 2] into a from 0: a + f\n",
    ),
    (
        "Clip k:\n    Note/1\n",
        "let y = for k in [1, 2] into a from 0: a + k\n",
    ),
    (
        "pattern P(v):\n    Note/(v)\n",
        "let y = for P in [1, 2] into a from 0: a + P\n",
    ),
    (
        "fn g(x): x\n",
        "let y = for f in [1, 2] into a from 0: a + f\n",
    ),
];

#[test]
fn a_fold_name_equal_to_a_name_another_file_declares_draws_for_variable_clash_at_that_declaration_as_the_old_engine_does()
 {
    let vocabulary = symphonyscript_fixtures::vocabulary();
    let old = Compiler::new(&vocabulary)
        .expect("the vocabulary compiles")
        .on_the_old_engine();
    let new = Compiler::new(&vocabulary).expect("the vocabulary compiles");
    let mut programs = 0;
    for (declared, folded) in CLASHES_ACROSS_FILES {
        for (first, second) in [("a.sym", "main.sym"), ("main.sym", "a.sym")] {
            let sources = [
                crate::Source {
                    path: first.into(),
                    text: declared.into(),
                },
                crate::Source {
                    path: second.into(),
                    text: format!("{folded}Clip c:\n    Note/(y)\n{TWIN}"),
                },
            ];
            let found = new
                .checked_by_new_core(&sources)
                .unwrap_or_else(|| panic!("the new core takes {declared}{folded}"));
            assert_eq!(found, old.check(&sources), "{declared}{folded}");
            programs += 1;
        }
    }
    assert_eq!(programs, 8);
}

const REACTIVE_REFERENCES_IN_BODIES: [&[&str]; 9] = [
    &[
        "state s(0)\nClip a:\n    Note/1\nClip b:\n    Note/2\n",
        "fn pick(t): if t > 0 then a else b\n",
        "Clip c:\n    Note when pick(s)\n",
    ],
    &[
        "state s(0)\nClip a:\n    Note/1\nClip b:\n    Note/2\n",
        "fn pick(t): if t > 0 then a else b\n",
        "Clip c:\n    Note/(pick(s))\n",
    ],
    &[
        "state s(0)\nClip a:\n    Note/1\nClip b:\n    Note/2\n",
        "fn pick(t): if t > 0 then a else b\n",
        "Clip c:\n    Note when (pick(s) and true)\n",
    ],
    &[
        "state s(0)\nClip a:\n    Note/1\nClip b:\n    Note/2\n",
        "fn pick(t): if t > 0 then a else b\nfn twice(t): pick(t)\n",
        "Clip c:\n    Note when twice(s)\n",
    ],
    &[
        "state s(0)\nClip a:\n    Note/1\nClip b:\n    Note/2\n",
        "fn pick(t): [a, b][t]\n",
        "Clip c:\n    Note when (not pick(s))\n",
    ],
    &[
        "state s(0)\nClip a:\n    Note/1\nClip b:\n    Note/2\n",
        "fn pick(k):\n    let t = [a, b][k]\n    k\n",
        "Clip c:\n    Note/(pick(s))\n",
    ],
    &[
        "state s(0)\nClip a:\n    Note/1\nClip b:\n    Note/2\n",
        "fn pick(k):\n    let t = [a, b][k]\n    t\n",
        "Clip c:\n    Note when pick(s)\n",
    ],
    &[
        "Clip a:\n    Note/1\nClip b:\n    Note/2\n",
        "fn pick(k):\n    let t = [a, b][k]\n    k\n",
        "Clip c:\n    Note/(pick(1))\n",
    ],
    &[
        "state s(0)\nClip a:\n    Note/1\nClip b:\n    Note/2\n",
        "pattern P(default t):\n    Note | velocity=(if t > 0 then a else b)\n",
        "Clip c:\n    P/(s)\n",
    ],
];

#[test]
fn a_reference_that_reads_a_cell_in_a_body_draws_what_the_old_engine_draws_and_a_body_let_holding_one_draws_reactive_node_reference_beside_a_valid_twin_in_every_order()
 {
    assert_eq!(same_as_the_old_engine(&REACTIVE_REFERENCES_IN_BODIES), 216);
}

const PATHS_THROUGH_LETS: [&[&str]; 8] = [
    &["let d = a\n", "Clip c | level=d.level:\n    Note/3\n"],
    &["let d = a\n", "Clip c | level=d.nope:\n    Note/3\n"],
    &["let d = [a, b][1]\n", "Clip c:\n    Note/(d.level)\n"],
    &[
        "let d = if true then a else b\n",
        "Clip c | level=d.level:\n    Note/3\n",
    ],
    &[
        "state s(0)\nlet d = [a, b][s]\n",
        "Clip c | level=d.level:\n    Note/3\n",
    ],
    &[
        "let d = a\nlet e = d\n",
        "Clip c | level=e.level:\n    Note/3\n",
    ],
    &["let d = c\n", "Clip c | level=d.level:\n    Note/3\n"],
    &["let d = 1 + true\n", "Clip c:\n    Note/(d.level)\n"],
];

#[test]
fn a_path_through_a_let_holding_a_reference_reads_the_instance_it_holds_and_through_any_other_let_draws_what_the_old_engine_draws_beside_a_valid_twin_in_every_order()
 {
    let rows: Vec<Vec<&str>> = PATHS_THROUGH_LETS
        .iter()
        .map(|items| {
            let mut row =
                vec!["Clip a | level=2:\n    Note as h /1\nClip b | level=3:\n    Note/2\n"];
            row.extend(items.iter());
            row
        })
        .collect();
    let rows: Vec<&[&str]> = rows.iter().map(Vec::as_slice).collect();
    assert_eq!(same_as_the_old_engine(&rows), 192);
}

const LET_ARRAY_MEMBERS: [&[&str]; 16] = [
    &["Clip c:\n    Note/(xs.size)\n"],
    &["Clip c:\n    Note/(xs.capacity + 300)\n"],
    &["Clip c:\n    Note/([1, 2][xs.size])\n"],
    &["Clip c:\n    Note/(ys.size)\n"],
    &["Clip c:\n    Note/(k.size)\n"],
    &["Clip c:\n    Note/(k.capacity)\n"],
    &["Clip c:\n    Note/(xs.size.size)\n"],
    &["Clip c:\n    Note/(ys.size + 1.5)\n"],
    &["Clip c:\n    Note when xs.size\n"],
    &["Clip c:\n    Note/((for i in 0..xs.size into a from 0: a + i))\n"],
    &[
        "let n = xs.size\nlet m: u8 = n * 200\n",
        "Clip c:\n    Note/(m)\n",
    ],
    &[
        "let n = xs.size\nlet zs = [n, n, n]\n",
        "Clip c:\n    Note/(zs[n + 1])\n",
    ],
    &["Clip c:\n    Note/(ys[0].capacity)\n"],
    &["Clip c:\n    Note/(ys[2].size + 300)\n"],
    &["Clip c:\n    Note/([1, 2][ys[1].size])\n"],
    &["Clip c | level=xs.size:\n    Note/1\n"],
];

#[test]
fn a_let_arrays_size_and_capacity_are_its_length_known_at_compile_time_and_a_scalar_lets_draw_unknown_attribute_beside_a_valid_twin_in_every_order()
 {
    let rows: Vec<Vec<&str>> = LET_ARRAY_MEMBERS
        .iter()
        .map(|items| {
            let mut row = vec!["let xs = [5, 6]\nlet ys = [[1, 2], [3, 4], [5, 6]]\nlet k = 2\n"];
            row.extend(items.iter());
            row
        })
        .collect();
    let rows: Vec<&[&str]> = rows.iter().map(Vec::as_slice).collect();
    assert_eq!(same_as_the_old_engine(&rows), 132);
}

const CELL_CAPACITIES: [&[&str]; 6] = [
    &["Clip c:\n    Note/(held.capacity)\n"],
    &["Clip c:\n    Note/(held.capacity + 300)\n"],
    &["Clip c:\n    Note/([1, 2][held.capacity])\n"],
    &["Clip c:\n    Note/(held.capacity.size)\n"],
    &[
        "let n = held.capacity\nlet m: u8 = n * 100\n",
        "Clip c:\n    Note/(m)\n",
    ],
    &["state r = held.capacity + q\n", "Clip c:\n    Note/(r)\n"],
];

#[test]
fn a_variable_cells_capacity_is_its_annotated_capacity_known_at_compile_time_reading_no_cell_beside_a_valid_twin_in_every_order()
 {
    let rows: Vec<Vec<&str>> = CELL_CAPACITIES
        .iter()
        .map(|items| {
            let mut row = vec!["state held([]): i32[..4]\nstate q(2)\n"];
            row.extend(items.iter());
            row
        })
        .collect();
    let rows: Vec<&[&str]> = rows.iter().map(Vec::as_slice).collect();
    assert_eq!(same_as_the_old_engine(&rows), 72);
}

const LITERAL_ARRAY_MEMBERS: [&[&str]; 7] = [
    &["Clip c:\n    Note/([1, 2.5].size)\n"],
    &["Clip c:\n    Note/([1, 2.5, 3000000000].capacity)\n"],
    &["Clip c:\n    Note/([5, 6].size)\n"],
    &["Clip c:\n    Note/([5, 6].capacity + 300)\n"],
    &["Clip c:\n    Note/([1, 2][[5, 6].size])\n"],
    &["Clip c:\n    Note/([1.5, 2.5].size)\n"],
    &["Clip c:\n    Note/([5, 6].size.size)\n"],
];

#[test]
fn a_literal_arrays_size_and_capacity_are_its_length_known_at_compile_time_beside_a_valid_twin_in_every_order()
 {
    assert_eq!(same_as_the_old_engine(&LITERAL_ARRAY_MEMBERS), 14);
}

const ARRAY_ATTRIBUTE_VALUES: [&[&str]; 10] = [
    &["Clip c | trio=[1, 2, 3, 4][0..3]:\n    Note/1\n"],
    &["let xs = [1, 2, 3]\n", "Clip c | trio=xs:\n    Note/1\n"],
    &["let xs = [1, 2]\n", "Clip c | trio=xs:\n    Note/1\n"],
    &[
        "let xs = [1, 3000000000, 2]\n",
        "Clip c | trio=xs:\n    Note/1\n",
    ],
    &["let xs = [1, 2.5, 2]\n", "Clip c | trio=xs:\n    Note/1\n"],
    &[
        "let xs = [1, 2, 3]\nlet ys = xs\n",
        "Clip c | trio=ys:\n    Note/1\n",
    ],
    &[
        "let xs = [[1, 2], [3, 4]]\n",
        "Clip c | trio=xs[0]:\n    Note/1\n",
    ],
    &[
        "let xs = [1, 2, 3]\n",
        "Clip c | trio=(if true then xs else [4, 5, 6]):\n    Note/1\n",
    ],
    &[
        "let xs = [1, 2, 3000000000]\n",
        "Clip c | trio=xs[0..]:\n    Note/1\n",
    ],
    &[
        "let xs = [1, 2, 3]\n",
        "Clip c | trio=xs[1..]:\n    Note/1\n",
    ],
];

#[test]
fn an_integer_array_attribute_whose_value_is_a_compile_time_array_draws_its_shape_and_fit_at_the_value_as_the_old_engine_does()
 {
    let vocabulary = symphonyscript_fixtures::native_vocabulary();
    let old = Compiler::new(&vocabulary)
        .expect("the vocabulary compiles")
        .on_the_old_engine();
    let new = Compiler::new(&vocabulary).expect("the vocabulary compiles");
    let mut programs = 0;
    for items in ARRAY_ATTRIBUTE_VALUES {
        for order in Orders::Every.of(items) {
            let text = order.concat();
            let sources = [source(&text)];
            let found = new
                .checked_by_new_core(&sources)
                .unwrap_or_else(|| panic!("the new core takes {text}"));
            assert_eq!(found, old.check(&sources), "{text}");
            programs += 1;
        }
    }
    assert_eq!(programs, 19);
}

const UNKNOWN_MEMBER_KINDS: [&[&str]; 14] = [
    &["Plant fill: Valve/0 Valvex/0.5 (Controller/0.4 | gain=0.5)\n"],
    &["Plant fill: Valve/0 Valve/0.5 (Controller/0.4 | gain=0.5)bounded\n"],
    &["Plant fill: Valve/0 Valve/0.5 (Controllerbounded /0.4 | gain=0.5)\n"],
    &["Plant fill: Valve/0 Valve/0.5 x(Controller/0.4 | gain=0.5)\n"],
    &["Clip c: Nope/1\n"],
    &["Clip c:\n    [Note/1 Nope/2]\n"],
    &["Clip c:\n    Note/1 Nope/2\n"],
    &["Clip c:\n    [Nope/(1 + true) Note/2]\n"],
    &["Clip c:\n    Note/1 (Nope/2 | gain=x)\n"],
    &["Clip c:\n    Note as n /1 Nope/2\n"],
    &["Clip c:\n    Note/1 Note/2 Nope/3\n"],
    &["Clip c:\n    Nope/1 Note/2\n"],
    &["Clip c:\n    [Note/1 Lamp/2]\n"],
    &["Clip c:\n    Note/1 Lamp/2\n"],
];

#[test]
fn a_member_of_an_unknown_kind_draws_unknown_kind_with_the_old_engines_naming_hint_beside_a_valid_twin_in_every_order()
 {
    assert_eq!(same_as_the_old_engine(&UNKNOWN_MEMBER_KINDS), 28);
}

const DIVIDED_INTEGER_LITERALS: [&[&str]; 16] = [
    &[
        "state q(0): i32\n",
        "Clip c:\n    Note/([1, 2][i32(300000000000 / q)])\n",
    ],
    &[
        "state q(0): i32\n",
        "Clip c:\n    Note/(if 300000000000 / q > 1 then 1 else 2)\n",
    ],
    &[
        "state q(0): u8\n",
        "Clip c:\n    Note | accent=(300000000000 / q > 1.5)\n",
    ],
    &[
        "let x = 300000000000 / 7\n",
        "Clip c:\n    Note/(if x > 1 then 1 else 2)\n",
    ],
    &[
        "let x = 300000000000 / 7\n",
        "Clip c:\n    Note/(i32(x + 1))\n",
    ],
    &[
        "let x = 300000000000 / 7\n",
        "Clip c:\n    Note | velocity=x\n",
    ],
    &[
        "let x = 300000000000 / 7\n",
        "Clip c:\n    Note/(i32(f64(x)))\n",
    ],
    &["Clip c:\n    Note/(i32(f32(300000000000 / 7)))\n"],
    &["Clip c:\n    Note/(i32(f64(300000000000 / 7)))\n"],
    &["Clip c:\n    Note/(i32(300000000000 / 7))\n"],
    &[
        "state q(2)\n",
        "Clip c:\n    Note/(if 3 / q > 1 then 1 else 2)\n",
    ],
    &["state q(2)\n", "Clip c:\n    Note/(i32(3 / q))\n"],
    &[
        "state q(2)\n",
        "Clip c:\n    Note/(if (q + 0.5) * 2 > 1 then 1 else 2)\n",
    ],
    &[
        "state q(2)\n",
        "Clip c:\n    Note/(if q > 1.5 then 1 else 2)\n",
    ],
    &[
        "state q(2)\n",
        "Clip c:\n    Note/(if f32(q) > 1 then 1 else 2)\n",
    ],
    &[
        "state q(2)\n",
        "Clip c:\n    Note | velocity=(3 / q)\n    Note/(if 3 / q > 1 then 1 else 2)\n",
    ],
];

#[test]
fn an_integer_literal_divided_states_its_fault_and_a_cell_divided_its_float_use_as_the_old_engine_does_beside_a_valid_twin_in_every_order()
 {
    assert_eq!(same_as_the_old_engine(&DIVIDED_INTEGER_LITERALS), 84);
}

const CELL_SIZES: [&[&str]; 16] = [
    &[
        "state held([]): i32[..10]\nstate step(0)\nstate note(60)\n",
        "state played = held + [note]\n",
        "Clip arp:\n    Note/(played[step % played.size])\n",
    ],
    &[
        "state held([]): i32[..10]\n",
        "Clip c:\n    Note/(held.size + 3000000000)\n",
    ],
    &[
        "state held([]): i32[..10]\n",
        "Clip c:\n    Note | velocity=(held.size / 2)\n",
    ],
    &[
        "state held([]): i32[..10]\nlet n = held.size\n",
        "Clip c:\n    Note/n\n",
    ],
    &[
        "state held([]): i32[..10]\nfn f(): held.size\n",
        "Clip c:\n    Note/(f())\n",
    ],
    &[
        "state steps([true, false, true])\n",
        "Clip c:\n    Note/(steps.size)\n",
    ],
    &[
        "state steps([1, 2, 3])\nstate a = steps.size\n",
        "Clip c:\n    Note/a\n",
    ],
    &[
        "state a = b.size\n",
        "state b = [a]\n",
        "Clip c:\n    Note/a\n",
    ],
    &[
        "state held([]): i32[..10]\n",
        "Clip c:\n    Note/(held.size.size)\n",
    ],
    &[
        "state held([]): i32[..10]\n",
        "Clip c:\n    Note | accent=(held.size > 2)\n",
    ],
    &["state x(1)\n", "Clip c:\n    Note/(x.size)\n"],
    &[
        "state xs([1, 2])\n",
        "Clip c:\n    Note/(xs.capacity + xs.size + 3000000000)\n",
    ],
    &[
        "state note(60)\nstate ys = [note, note]\n",
        "Clip c:\n    Note/(ys.size)\n",
    ],
    &[
        "state held([]): i32[..10]\n",
        "state recent = (recent.past(1, []) + held)[^4..]\n",
        "Clip c:\n    Note/(recent.size)\n",
    ],
    &[
        "state xs([1.5, 2.5])\n",
        "Clip c:\n    Note | velocity=(xs[0])\n    Note/(xs.size)\n",
    ],
    &[
        "state a([]): i32[..5]\n",
        "state probe: i32[..3] = a[..a.size - 1]\n",
        "Clip c:\n    Note/(probe[0])\n",
    ],
];

#[test]
fn a_cells_size_is_its_length_known_at_compile_time_or_a_runtime_i32_bounded_by_its_capacity_beside_a_valid_twin_in_every_order()
 {
    assert_eq!(same_as_the_old_engine(&CELL_SIZES), 168);
}

const FLEXIBLE_RULES: [&[&str]; 10] = [
    &["state n = 2 + 3000000000\n", "Clip c:\n    Note/n\n"],
    &[
        "state xs([1, 2])\nstate n = xs.size + 3000000000\n",
        "Clip c:\n    Note/n\n",
    ],
    &[
        "state n = 3000000000 - 3000000000\n",
        "Clip c:\n    Note/n\n",
    ],
    &[
        "state n = [1, 2][5] + 3000000000\n",
        "Clip c:\n    Note/n\n",
    ],
    &["state n = 2 + 3000000000\n", "Clip c:\n    Note/(u8(n))\n"],
    &[
        "state n = 2 + 3000000000\nlet m = 3\n",
        "Clip c:\n    Note/(n + m)\n",
    ],
    &[
        "state n = 2 + 30\n",
        "state k = n + 3000000000\n",
        "Clip c:\n    Note/k\n",
    ],
    &[
        "state n = 2 + 3000000000\n",
        "Clip c:\n    Note | velocity=n\n",
    ],
    &[
        "state n = 3000000000\nstate m = n\n",
        "Clip c:\n    Note/(u8(m))\n",
    ],
    &["state n = 3000000000\n", "Clip c:\n    Note/(i64(n))\n"],
];

#[test]
fn a_derived_cells_flexible_compile_time_rule_fits_its_width_whole_as_the_old_engine_does_beside_a_valid_twin_in_every_order()
 {
    assert_eq!(same_as_the_old_engine(&FLEXIBLE_RULES), 78);
}

const DOT_CALLS: [&[&str]; 12] = [
    &["fn plus(a, b): a + b\n", "Clip c:\n    Note/(3.plus(4))\n"],
    &[
        "fn plus(a, b): a + b\nlet x = 3\n",
        "Clip c:\n    Note/(x.plus(4))\n",
    ],
    &["fn plus(a, b): a + b\n", "Clip c:\n    Note/(3.plus())\n"],
    &[
        "fn plus(a, b): a + b\n",
        "Clip c:\n    Note/(3.plus(4, 5))\n",
    ],
    &[
        "fn plus(a, b): a + b\n",
        "Clip c:\n    Note/(3.plus(true))\n",
    ],
    &[
        "fn first(xs): xs[0]\n",
        "Clip c:\n    Note/([1, 2].first())\n",
    ],
    &[
        "fn plus(a, b): a + b\n",
        "Clip c:\n    Note/(3.plus(3000000000))\n",
    ],
    &[
        "fn plus(a, b): a + b\n",
        "Clip c:\n    Note/(3.plus(4).plus(5))\n",
    ],
    &[
        "fn plus(a, b): a + b\nfn twice(x): x.plus(x)\n",
        "Clip c:\n    Note/(twice(3000000000))\n",
    ],
    &[
        "fn plus(a, b): a + b\nstate q(1)\n",
        "Clip c:\n    Note/(q.plus(1))\n    Note | velocity=(q.plus(0.5))\n",
    ],
    &[
        "fn at(xs, i): xs[i]\n",
        "Clip c:\n    Note/([1, 2, 3].at(5))\n",
    ],
    &[
        "fn plus(a, b): a + b\n",
        "Clip c:\n    for i in 0..3:\n        Note/(i.plus(60))\n",
    ],
];

#[test]
fn a_dot_call_of_a_source_function_is_the_call_with_its_receiver_first_beside_a_valid_twin_in_every_order()
 {
    assert_eq!(same_as_the_old_engine(&DOT_CALLS), 72);
}

const FUNCTION_NAMES_AND_NON_FUNCTIONS: [&[&str]; 19] = [
    &["pattern P(x):\n    Note/x\n", "Clip c:\n    Note/(P(1))\n"],
    &[
        "pattern P(x):\n    Note/x\n",
        "Clip c:\n    Note/(P(1 + true))\n    P/2\n",
    ],
    &[
        "pattern P(x):\n    Note/x\n",
        "Clip c:\n    Note/(P(x=1))\n",
    ],
    &["fn f(): 1\n", "Clip c:\n    Note/(f + 1)\n"],
    &["fn f(): 1\n", "Clip c:\n    Note/f\n"],
    &["fn f(): 1\n", "Clip c:\n    Note/([f, 2][0])\n"],
    &["fn f(): 1\n", "Clip c:\n    Note/(if true then f else 2)\n"],
    &["fn f(): 1\nstate s = f\n", "Clip c:\n    Note/s\n"],
    &["fn f(): 1\nlet g = f\n", "Clip c:\n    Note/(g())\n"],
    &["fn f(): 1\n", "Clip c:\n    Note | accent=(f is f)\n"],
    &["fn f(): 1\nfn h(x): x + 1\n", "Clip c:\n    Note/(h(f))\n"],
    &["fn f(): 1\nfn k(): f\n", "Clip c:\n    Note/(k())\n"],
    &["fn f(): 1\n", "Clip c:\n    Note/(i32(f))\n"],
    &["let g = 3\n", "Clip c:\n    Note/(g(1 + true))\n"],
    &[
        "state g(1)\n",
        "Clip c:\n    Note | velocity=(g(1) + 0.5)\n",
    ],
    &["Osc as o\n", "Clip c:\n    Note/(o())\n"],
    &["let g = 3\nlet h = g(1)\n", "Clip c:\n    Note/h\n"],
    &["let g = 3\n", "Clip c:\n    Note/(g(zz))\n"],
    &["let g = 3\n", "Clip c:\n    Note/(g(a=1))\n"],
];

#[test]
fn a_function_name_outside_a_callee_is_not_a_value_and_a_call_of_a_value_is_not_callable_beside_a_valid_twin_in_every_order()
 {
    assert_eq!(
        same_as_the_old_engine(&FUNCTION_NAMES_AND_NON_FUNCTIONS),
        114
    );
}

const ABSOLUTE_VALUES: [&[&str]; 20] = [
    &["Clip c:\n    Note/(abs(-3))\n"],
    &["Clip c:\n    Note/(abs(-2147483648))\n"],
    &["Clip c:\n    Note/(abs(-3000000000) - 3000000000)\n"],
    &["Clip c:\n    Note | velocity=(abs(-0.5))\n"],
    &["Clip c:\n    Note/(abs(true))\n"],
    &["Clip c:\n    Note/(abs([1, 2]))\n"],
    &["Clip c:\n    Note/(abs([true]))\n"],
    &["state q(1)\n", "Clip c:\n    Note/(abs(q) + 3000000000)\n"],
    &["state q(0.5)\n", "Clip c:\n    Note/(abs(q))\n"],
    &["let x = abs(-5)\n", "Clip c:\n    Note/([1, 2][x])\n"],
    &[
        "fn f(a): abs(a) + 1\n",
        "Clip c:\n    Note/(f(-3))\n    Note | velocity=(f(-0.25))\n",
    ],
    &["Clip c:\n    Note/(abs(zz))\n"],
    &["Clip c:\n    Note/(abs(i32(-2147483648)))\n"],
    &["Clip c:\n    Note/(abs(u8(3)) + 300)\n"],
    &[
        "state q(1)\nstate r = abs(q) * 2\n",
        "Clip c:\n    Note/r\n",
    ],
    &["Clip c:\n    Note/(abs(i64(-5)) + 3000000000)\n"],
    &["Clip c:\n    Note/(abs(u8(200)))\n"],
    &["state q(1): i32\n", "Clip c:\n    Note/(abs(q))\n"],
    &["Clip c:\n    Note/(abs(if true then -3 else 2.5))\n"],
    &["Clip c:\n    Note/(abs(-3000000000))\n"],
];

#[test]
fn abs_of_one_number_is_its_magnitude_in_the_operands_type_saturating_beside_a_valid_twin_in_every_order()
 {
    assert_eq!(same_as_the_old_engine(&ABSOLUTE_VALUES), 64);
}

const LEAST_AND_GREATEST: [&[&str]; 18] = [
    &["Clip c:\n    Note/(min(3, 5))\n"],
    &["Clip c:\n    Note/(max(3, 3000000000))\n"],
    &["Clip c:\n    Note/(min(3, 3000000000))\n"],
    &["Clip c:\n    Note/(clamp(70, 0, 64))\n"],
    &["Clip c:\n    Note/(min(1.5, 2))\n"],
    &["Clip c:\n    Note | velocity=(max(0.25, 2))\n"],
    &["Clip c:\n    Note/(min(true, 2))\n"],
    &["Clip c:\n    Note/(max([1], 2))\n"],
    &["state q(1)\n", "Clip c:\n    Note/(min(q, 3000000000))\n"],
    &["state q(1)\n", "Clip c:\n    Note/(clamp(q, 0, 64))\n"],
    &["Clip c:\n    Note/(min(i32(3), u8(4)))\n"],
    &["Clip c:\n    Note/(max(u8(3), 300))\n"],
    &["Clip c:\n    Note/(min(i64(3), 2) + 3000000000)\n"],
    &[
        "let x = clamp(9, 0, 5)\n",
        "Clip c:\n    Note/([1, 2][x])\n",
    ],
    &["Clip c:\n    Note | velocity=(min(-0.0, 0.0))\n"],
    &["Clip c:\n    Note/(min(zz, 2))\n"],
    &[
        "fn f(a, b): max(a, b) + 1\n",
        "Clip c:\n    Note/(f(1, 2))\n    Note | velocity=(f(0.5, 1))\n",
    ],
    &["Clip c:\n    Note/(min(abs(-70), max(1, 64)))\n"],
];

#[test]
fn min_max_and_clamp_compute_in_their_operands_combined_type_beside_a_valid_twin_in_every_order() {
    assert_eq!(same_as_the_old_engine(&LEAST_AND_GREATEST), 52);
}

#[test]
fn a_math_function_keeps_its_operands_unit_family_as_the_old_engine_does() {
    let vocabulary = symphonyscript_fixtures::envelope_vocabulary();
    let old = Compiler::new(&vocabulary)
        .expect("the vocabulary compiles")
        .on_the_old_engine();
    let new = Compiler::new(&vocabulary).expect("the vocabulary compiles");
    let mut programs = 0;
    for text in [
        "Env e | attack=(min(1s, 3))\n",
        "Env e | attack=(max(1s, 2ms))\n",
        "Env e | attack=(clamp(3, 1s, 5s))\n",
        "Env e | attack=(abs(-3ms) + 1)\n",
        "let d = min(2, 1s)\nEnv e | attack=d\n",
        "Env e | attack=(min(1s, 3) / 2)\n",
        "Env e | attack=(max(3i32, 1s))\n",
        "Env e | attack=(floor(1.5s))\n",
        "Env e | attack=(sqrt(4s))\n",
        "Env e | attack=(round(3))\n",
        "Env e | attack=(lerp(1s, 2s, 0.5))\n",
        "Env e | attack=(lerp(1s, 2, 0.5))\n",
        "Env e | attack=(lerp(1, 2, 1s))\n",
    ] {
        let sources = [source(text)];
        let found = new
            .checked_by_new_core(&sources)
            .unwrap_or_else(|| panic!("the new core takes {text}"));
        assert_eq!(found, old.check(&sources), "{text}");
        programs += 1;
    }
    assert_eq!(programs, 13);
}

const FLOAT_FUNCTIONS: [&[&str]; 19] = [
    &["Clip c:\n    Note | velocity=(floor(2.7))\n"],
    &["Clip c:\n    Note | velocity=(sqrt(2))\n"],
    &["Clip c:\n    Note/(floor(2.5))\n"],
    &["Clip c:\n    Note/(i32(round(2.5)))\n"],
    &["Clip c:\n    Note/(i32(sqrt(300000000001)))\n"],
    &["Clip c:\n    Note | velocity=(sin(i32(3)))\n"],
    &["Clip c:\n    Note | velocity=(log(true))\n"],
    &["Clip c:\n    Note | velocity=(exp([1.0]))\n"],
    &["state q(0.5)\n", "Clip c:\n    Note | velocity=(cos(q))\n"],
    &["state q(2)\n", "Clip c:\n    Note | velocity=(sqrt(q))\n"],
    &["Clip c:\n    Note | velocity=(exp(1000))\n"],
    &["Clip c:\n    Note | accent=(ceil(2.1) > 2)\n"],
    &["let x = floor(2.7)\n", "Clip c:\n    Note | velocity=x\n"],
    &[
        "fn f(a): sqrt(a) + 1\n",
        "Clip c:\n    Note | velocity=(f(4))\n",
    ],
    &["Clip c:\n    Note | velocity=(tan(1e300))\n"],
    &["Clip c:\n    Note/([1, 2][i32(floor(1.5))])\n"],
    &[
        "state q(2)\n",
        "Clip c:\n    Note/(if sqrt(q) > 1 then 1 else 2)\n",
    ],
    &[
        "state q(2)\n",
        "Clip c:\n    Note/(if floor(q / 2) > 1 then 1 else 2)\n",
    ],
    &[
        "state q(2)\n",
        "Clip c:\n    Note | velocity=(min(q, 0.5))\n",
    ],
];

#[test]
fn a_float_math_function_converts_an_integer_as_division_does_and_yields_a_float_beside_a_valid_twin_in_every_order()
 {
    assert_eq!(same_as_the_old_engine(&FLOAT_FUNCTIONS), 66);
}

const LERPS: [&[&str]; 9] = [
    &["Clip c:\n    Note | velocity=(lerp(0, 1, 0.25))\n"],
    &["Clip c:\n    Note/(lerp(0, 10, 0.5))\n"],
    &["Clip c:\n    Note/(i32(lerp(0, 10, 0.5)))\n"],
    &["Clip c:\n    Note | velocity=(lerp(true, 1, 0.5))\n"],
    &[
        "state q(0.5)\n",
        "Clip c:\n    Note | velocity=(lerp(0.2, 0.8, q))\n",
    ],
    &[
        "state q(1)\n",
        "Clip c:\n    Note | velocity=(lerp(0, q, 0.5))\n",
    ],
    &["Clip c:\n    Note | velocity=(lerp(0, 300000000001, 0.5))\n"],
    &["Clip c:\n    Note | velocity=(lerp(i64(0), 3, 0.5))\n"],
    &[
        "let x = lerp(2, 4, 0.5)\n",
        "Clip c:\n    Note/([1, 2, 3, 4][i32(x)])\n",
    ],
];

#[test]
fn lerp_is_a_float_between_its_first_two_arguments_beside_a_valid_twin_in_every_order() {
    assert_eq!(same_as_the_old_engine(&LERPS), 30);
}

const RANDS: [&[&str]; 9] = [
    &["Clip c:\n    Note | velocity=(rand(1, 2))\n"],
    &["Clip c:\n    Note/([1, 2, 3][i32(rand(1, 2) * 3)])\n"],
    &["Clip c:\n    Note | velocity=(rand(1.5, 2))\n"],
    &["Clip c:\n    Note | velocity=(rand(true, 2))\n"],
    &["Clip c:\n    Note | velocity=(rand(9223372036854775808, 2))\n"],
    &[
        "state q(1)\n",
        "Clip c:\n    Note | velocity=(rand(q, 2))\n",
    ],
    &["Clip c:\n    Note | velocity=(rand(2 ** 70, 2))\n"],
    &["Clip c:\n    Note/(rand(1, 2))\n"],
    &["Clip c:\n    Note | velocity=(rand(i32(3), u8(2)) + 1)\n"],
];

#[test]
fn rand_is_an_f32_fixed_by_two_integers_each_fitting_i64_beside_a_valid_twin_in_every_order() {
    assert_eq!(same_as_the_old_engine(&RANDS), 22);
}

const MATH_COUNTS: [&[&str]; 10] = [
    &["Clip c:\n    Note/(abs(x=1 + true))\n"],
    &["Clip c:\n    Note/(min(a=1, b=zz))\n"],
    &["fn f(a): abs(x=a)\n", "Clip c:\n    Note/(f(1))\n"],
    &["let y = max(a=1, b=2)\n", "Clip c:\n    Note/y\n"],
    &["Clip c:\n    Note/(abs(1, 2))\n"],
    &["Clip c:\n    Note/(min(1))\n"],
    &["Clip c:\n    Note/(clamp(1, 2))\n"],
    &["Clip c:\n    Note | velocity=(sqrt())\n"],
    &["Clip c:\n    Note/(min(1, 2, 3 + true))\n"],
    &[
        "state q(1)\n",
        "Clip c:\n    Note | velocity=(rand(q) + 3000000000)\n",
    ],
];

#[test]
fn a_math_call_of_another_count_draws_argument_count_and_checks_its_arguments_beside_a_valid_twin_in_every_order()
 {
    assert_eq!(same_as_the_old_engine(&MATH_COUNTS), 32);
}

const PARAMETER_FITS: [&[&str]; 16] = [
    &[
        "fn plus(a, b): a + b\nstate q(1)\n",
        "Clip c:\n    Note/(plus(q, 30))\n",
    ],
    &[
        "fn plus(a, b): a + b\nstate q(1)\n",
        "Clip c:\n    Note/(plus(q, 3000000000))\n",
    ],
    &[
        "fn plus(a, b): a + b\nstate q(1)\n",
        "Clip c:\n    Note/(q.plus(3000000000))\n",
    ],
    &[
        "fn plus(a, b): a + b\nstate q(1)\n",
        "Clip c:\n    Note/(plus(q, 3000000000 + 1))\n",
    ],
    &[
        "fn plus(a, b): a + b\nstate q(1)\nlet z = 3000000000\n",
        "Clip c:\n    Note/(plus(q, z))\n",
    ],
    &[
        "fn plus(a, b): a + b\n",
        "Clip c:\n    Note/(plus(1, 3000000000))\n",
    ],
    &[
        "fn two(b): (u8(1) + b) + i64(b)\n",
        "Clip c:\n    Note/(two(300))\n",
    ],
    &[
        "fn two(b): i64(b) + (u8(1) + b)\n",
        "Clip c:\n    Note/(two(300))\n",
    ],
    &[
        "fn two(b): (u8(1) + b) + i64(b)\n",
        "Clip c:\n    Note/(two(30))\n",
    ],
    &[
        "fn two(a, b): (a + b) + i64(b)\nstate q(1)\n",
        "Clip c:\n    Note/(two(q, 3000000000))\n",
    ],
    &[
        "fn two(a, b): (a + b) + i64(b)\nstate q(1)\n",
        "Clip c:\n    Note/(two(q, 30))\n",
    ],
    &[
        "fn f(b, q): b + q\nfn g(b): f(b, q) + i64(b)\nstate q(1)\n",
        "Clip c:\n    Note/(g(3000000000))\n",
    ],
    &[
        "fn two(a): (a + 1) + i64(1)\nstate q(1)\n",
        "Clip c:\n    Note/(two(q))\n",
    ],
    &[
        "fn f(a, b): a\nstate q(1)\n",
        "Clip c:\n    Note/(f(q, 3000000000))\n",
    ],
    &["fn f(a): u8(a)\n", "Clip c:\n    Note/(f(3000000000))\n"],
    &[
        "fn f(a, b): u8(a) + b\nstate q(1)\n",
        "Clip c:\n    Note/(f(3000000000, q))\n",
    ],
];

#[test]
fn a_flexible_argument_fits_at_each_use_of_its_parameter_and_a_reactive_body_refuses_its_own_operand_beside_a_valid_twin_in_every_order()
 {
    assert_eq!(same_as_the_old_engine(&PARAMETER_FITS), 96);
}

const NOT_CALLABLE_ANYWHERE: [&[&str]; 12] = [
    &["state k(1)\nlet j = k(1)\n", "Clip c:\n    Note/1\n"],
    &["let k = 1\nlet j = k(1)\n", "Clip c:\n    Note/1\n"],
    &["let k = 1\nfn g(): k(1)\n", "Clip c:\n    Note/1\n"],
    &[
        "state k(1)\nfn g(): k(1)\n",
        "Clip c:\n    Note/(g() + g())\n",
    ],
    &["state k(1)\nlet j = k(1) +\n", "Clip c:\n    Note/1\n"],
    &[
        "state k(1)\nlet j = k(1)\nlet m = j + 1\n",
        "Clip c:\n    Note/m\n",
    ],
    &["fn h(p): p(1)\n", "Clip c:\n    Note/(h(2))\n"],
    &["fn h(p): p(1 + true)\n", "Clip c:\n    Note/(h(2))\n"],
    &["fn h(p): p(1)\n", "Clip c:\n    Note/1\n"],
    &["fn h(p): p\n", "Clip c:\n    Note/(h(2))\n"],
    &["fn h(p): p(zz)\n", "Clip c:\n    Note/(h(2) + h(3))\n"],
    &["fn h(abs): abs(1)\n", "Clip c:\n    Note/(h(2))\n"],
];

#[test]
fn a_callee_naming_a_value_or_a_parameter_draws_not_callable_once_outside_every_call_and_at_each_call_inside_one_beside_a_valid_twin_in_every_order()
 {
    assert_eq!(same_as_the_old_engine(&NOT_CALLABLE_ANYWHERE), 72);
}

const CLASH_WITH_A_FAULTY_PLACEMENT: [&[&str]; 6] = [
    &[
        "state rises = for i in 1..3 into n from 0: n + 1\nstate n\n(4)\n",
        "Clip c:\n    Note/rises\n",
    ],
    &[
        "state rises = for i in 1..3 into n from 0: n + 1\nstate n\n",
        "Clip c:\n    Note/rises\n",
    ],
    &[
        "state rises = for i in 1..3 into n from 0: n + 1\nxstate n(4)\n",
        "Clip c:\n    Note/rises\n",
    ],
    &[
        "state rises = for i in 1..3 into n from 0: n + 1\nClip n(4)\n",
        "Clip c:\n    Note/rises\n",
    ],
    &[
        "state rises = for i in 1..3 into n from 0: n + 1\nstate n(4) +\n",
        "Clip c:\n    Note/rises\n",
    ],
    &[
        "state rises = for i in 1..3 into n from 0: n + 1\nlet n = 4\nlet n = 5\n",
        "Clip c:\n    Note/rises\n",
    ],
];

#[test]
fn a_for_name_equal_to_a_placement_on_a_faulty_line_draws_its_clash_with_that_name_as_related_beside_a_valid_twin_in_every_order()
 {
    assert_eq!(same_as_the_old_engine(&CLASH_WITH_A_FAULTY_PLACEMENT), 36);
}

const NAMED_DOT_CALLS: [&[&str]; 11] = [
    &["fn f(a, b): a + b\n", "Clip c:\n    Note/(1.f(b=2))\n"],
    &["fn f(a, b): a + b\n", "Clip c:\n    Note/(1.f(a=2))\n"],
    &["fn f(a, b): a + b\n", "Clip c:\n    Note/(1.f(c=2))\n"],
    &[
        "fn f(a, b, c): a + b + c\n",
        "Clip c:\n    Note/(1.f(c=2, b=3))\n",
    ],
    &["fn f(a): a\n", "Clip c:\n    Note/(1.f(b=2))\n"],
    &[
        "fn f(a, b): a + b\nstate q(1)\n",
        "Clip c:\n    Note/(q.f(b=3000000000))\n",
    ],
    &["fn f(a, b): a + b\n", "Clip c:\n    Note/(1.f(b=2, b=3))\n"],
    &["fn f(a, b): a + b\n", "Clip c:\n    Note/(1.f(b=zz))\n"],
    &["fn f(a, b): a + b\n", "Clip c:\n    Note/([1, 2].f(b=2))\n"],
    &[
        "fn f(a, b): a - b\nlet x = 5\n",
        "Clip c:\n    Note/(x.f(b=2) + x.f(2))\n",
    ],
    &["fn f(a, b): a - b\n", "Clip c:\n    Note/(1.f(b=true))\n"],
];

#[test]
fn a_named_dot_call_binds_its_receiver_to_the_first_parameter_and_its_names_to_the_rest_beside_a_valid_twin_in_every_order()
 {
    assert_eq!(same_as_the_old_engine(&NAMED_DOT_CALLS), 66);
}

const UNKNOWN_CALLEES_AND_MATH_DOT_CALLS: [&[&str]; 13] = [
    &["Clip c:\n    Note/(g(1 + true))\n"],
    &["Clip c:\n    Note/(g(zz))\n"],
    &["Clip c:\n    Note/(g(x=1 + true))\n"],
    &["let k = g(1)\nlet m = k + 1\n", "Clip c:\n    Note/m\n"],
    &["fn h(): g(1)\n", "Clip c:\n    Note/(h() + h())\n"],
    &["let x = g(x)\n", "Clip c:\n    Note/x\n"],
    &[
        "state q(1)\n",
        "Clip c:\n    Note | velocity=(g(q) + q / 2)\n",
    ],
    &["Clip c:\n    Note/((-3).abs())\n"],
    &["Clip c:\n    Note/(3.min(4))\n"],
    &["Clip c:\n    Note | velocity=(2.sqrt())\n"],
    &["Clip c:\n    Note/(1.abs(2))\n"],
    &["Clip c:\n    Note/(1.abs(x=2))\n"],
    &[
        "let y = 4\n",
        "Clip c:\n    Note/(y.max(5) + y.clamp(1, 3))\n",
    ],
];

#[test]
fn an_unknown_callee_draws_its_name_alone_and_a_math_dot_call_is_the_math_call_beside_a_valid_twin_in_every_order()
 {
    assert_eq!(
        same_as_the_old_engine(&UNKNOWN_CALLEES_AND_MATH_DOT_CALLS),
        46
    );
}

const NATIVE_FAULTS: [&[&str]; 9] = [
    &["Clip c:\n    Note/(checksum())\n"],
    &["Clip c:\n    Note/(checksum([u8(1)], 1 + true))\n"],
    &["Clip c:\n    Note/(pair_sum(zz, 1))\n"],
    &["Clip c:\n    Note/(checksum(x=1))\n"],
    &["Clip c:\n    Note/(checksum(x=1 + true))\n"],
    &["Clip c:\n    Note/(checksum(x=zz))\n"],
    &["let k = checksum()\n", "Clip c:\n    Note/1\n"],
    &["fn f(): mirror()\n", "Clip c:\n    Note/(f() + f())\n"],
    &["state q(1)\n", "Clip c:\n    Note/(three(q, q))\n"],
];

#[test]
fn a_native_call_of_another_count_or_with_named_arguments_draws_as_a_math_call_does_beside_a_valid_twin_in_every_order()
 {
    assert_eq!(
        same_in(
            &symphonyscript_fixtures::native_vocabulary(),
            &NATIVE_FAULTS
        ),
        30
    );
}

const BODY_LET_MEMBERS: [&[&str]; 4] = [
    &[
        "fn f(n):\n    let xs = [1, 2, 3]\n    xs.size + n\n",
        "Clip c:\n    Note/(f(3))\n",
    ],
    &[
        "fn f(n):\n    let xs = [1, 2.5, 3]\n    xs.size + xs.capacity + n\n",
        "Clip c:\n    Note/(f(3) + f(4))\n",
    ],
    &[
        "fn f(n):\n    let xs = [1, 2, 3]\n    u8(xs.size) + 300 + n\n",
        "Clip c:\n    Note/(f(3))\n",
    ],
    &[
        "fn f(n):\n    let xs = [1, 2, 3]\n    xs.size\n",
        "Clip c:\n    Note/1\n",
    ],
];

#[test]
fn a_member_of_a_body_let_holding_an_array_literal_is_its_length_beside_a_valid_twin_in_every_order()
 {
    assert_eq!(same_as_the_old_engine(&BODY_LET_MEMBERS), 24);
}

const LOOSE_FLOAT_ARGUMENTS: [&[&str]; 5] = [
    &[
        "state q(1.5)\nfn f(a, b): a + b\n",
        "Clip c:\n    Note/(f(q, 1e400))\n",
    ],
    &[
        "state q(1.5)\nfn f(a, b): a + b\n",
        "Clip c:\n    Note | velocity=(f(q, 1e400))\n",
    ],
    &[
        "state q(1.5)\nfn f(a, b): a + b\n",
        "Clip c:\n    Note/(f(q, 1e39))\n",
    ],
    &["fn f(a, b): a + b\n", "Clip c:\n    Note/(f(1.5, 1e400))\n"],
    &[
        "state q(1.5)\nfn f(a, b): a + b\n",
        "Clip c:\n    Note | velocity=(f(q, 0.5))\n",
    ],
];

#[test]
fn a_loose_float_argument_beside_a_refused_call_is_read_in_its_bodys_float_width_beside_a_valid_twin_in_every_order()
 {
    assert_eq!(same_as_the_old_engine(&LOOSE_FLOAT_ARGUMENTS), 30);
}

const MATH_OVER_ARRAYS: [&[&str]; 7] = [
    &[
        "state tw([]): f64[..4]\n",
        "Clip c:\n    Note/(abs((tw + [])))\n",
    ],
    &["state v0([]): i32[..4]\n", "Clip c:\n    Note/(abs(v0))\n"],
    &[
        "state v0([]): i32[..4]\n",
        "Clip c:\n    Note | velocity=(abs(v0))\n",
    ],
    &[
        "state v0([1.5])\n",
        "Clip c:\n    Note | velocity=(sqrt(v0) + min(v0, 1))\n",
    ],
    &["Clip c:\n    Note/(abs([1.5, 2]))\n"],
    &["let xs = [1.5, 2]\n", "Clip c:\n    Note/(abs(xs))\n"],
    &[
        "state n0(false)\n",
        "Clip c:\n    Note/(abs(if true then (if n0 then [1] else [1]) else [1.5]))\n",
    ],
];

#[test]
fn a_math_call_over_an_array_draws_array_operand_and_one_reading_an_array_cell_keeps_its_element_type_beside_a_valid_twin_in_every_order()
 {
    assert_eq!(same_as_the_old_engine(&MATH_OVER_ARRAYS), 38);
}

const LEN_AND_BYTES_CALLEES: [&[&str]; 4] = [
    &["Clip c:\n    Note/(len([1, 2]))\n"],
    &["Clip c:\n    Note/(bytes(1 + true))\n"],
    &[
        "let xs = [1, 2]\n",
        "Clip c:\n    Note/(len(xs) + lenx(xs))\n",
    ],
    &["state q(1)\n", "Clip c:\n    Note/(1 + bytes (q + 1))\n"],
];

#[test]
fn an_unknown_len_or_bytes_callee_draws_its_unknown_name_with_the_member_help_beside_a_valid_twin_in_every_order()
 {
    assert_eq!(same_as_the_old_engine(&LEN_AND_BYTES_CALLEES), 16);
}

const FUNCTION_VALUES_IN_UNCHECKED_ROOTS: [&[&str]; 7] = [
    &[
        "fn fib(n): if n < 2 then n else fib\n",
        "let lengths = [fib(3), fib(4), fib(5), fib(6)]\n",
    ],
    &[
        "fn fib(n): if n < 2 then n else fib\n",
        "let l = fib(3) + fib(4)\n",
    ],
    &[
        "fn fib(n): if n < 2 then n else fib\n",
        "let x = fib(3)\n",
        "let y = fib(4)\n",
    ],
    &["fn fib(n): if n < 2 then n else fib\n", "fn g(): fib(1)\n"],
    &[
        "fn fib(n): if n < 2 then n else fib\n",
        "fn g(): fib(1)\n",
        "let x = g() + fib(2)\n",
    ],
    &[
        "fn fib(n): if n < 2 then n else fib\n",
        "let x = fib(3)\n",
        "Clip c:\n    Note/(x)\n",
    ],
    &["fn fib(n): if n < 2 then n else 1\n", "let x = fib(3)\n"],
];

#[test]
fn a_function_read_as_a_value_in_a_body_only_unchecked_roots_reach_names_the_last_call_of_the_first_such_root_beside_a_valid_twin_in_every_order()
 {
    assert_eq!(
        same_as_the_old_engine(&FUNCTION_VALUES_IN_UNCHECKED_ROOTS),
        96
    );
}

const FLOAT_WIDTHS_BESIDE_A_REFUSAL: [&[&str]; 8] = [
    &[
        "state q(1.5)\n",
        "fn f(a, b): a * 1e300 + b\n",
        "Clip c:\n    Note/(f(q, 1e400))\n",
    ],
    &[
        "state q(1.5)\n",
        "fn f(a, b): a * 1e300 + b\n",
        "Clip c:\n    Note/(i32(f(q, 1e400)))\n",
    ],
    &[
        "state q(1.5)\n",
        "fn f(a, b): a * 1e3 + b\n",
        "Clip c:\n    Note/(i32(f(q, 1.5)))\n",
    ],
    &[
        "state q(1.5)\n",
        "fn f(a, b): a * 1e300 + b\n",
        "Clip c:\n    Note | velocity=(f(q, 1))\n",
    ],
    &[
        "state q(1.5)\n",
        "let b = 1e400\n",
        "Clip c:\n    Note/(q + b)\n",
    ],
    &[
        "state q(1.5)\n",
        "let b = 1e400\n",
        "Clip c:\n    Note/(i32(q + b))\n",
    ],
    &[
        "state q(1.5)\n",
        "let b = 1e300\n",
        "Clip c:\n    Note/(i32(q + b))\n",
    ],
    &["let b = 1e400\n", "Clip c:\n    Note/(i32(b))\n"],
];

#[test]
fn a_call_reading_an_argument_cell_computes_in_f32_and_a_lets_float_literal_keeps_its_own_width_beside_a_refusal_and_its_repaired_twin_in_every_order_523()
 {
    assert_eq!(same_as_the_old_engine(&FLOAT_WIDTHS_BESIDE_A_REFUSAL), 174);
}

const FLAG_ATTRIBUTES: [&[&str]; 9] = [
    &["Rig r:\n    Lamp | strobe\n"],
    &["Rig r:\n    Lamp | !strobe level=0.5\n"],
    &["Rig r:\n    Lamp | strobe=true\n"],
    &["Rig r:\n    Lamp | hue\n", "Rig s:\n    Lamp | !hue\n"],
    &["Rig r:\n    Lamp | zap strobe\n"],
    &[
        "Rig r:\n    Lamp | strobe strobe\n",
        "Rig s:\n    Lamp | strobe=true !strobe\n",
    ],
    &["Rig r:\n    Lamp | strobe level=(1 + true)\n"],
    &[
        "state intensity(0.5)\n",
        "state glow = intensity * 0.8\n",
        "Rig stage:\n    Lamp | level=0.7\n           hue=glow\n           strobe\n",
    ],
    &["Rig r:\n    Lamp as a | !strobe level=0.5\n    Lamp | level=a.level\n"],
];

#[test]
fn a_flag_attribute_sets_a_bool_attribute_and_draws_type_mismatch_unknown_or_duplicate_attribute_as_a_value_does_beside_a_valid_twin_in_every_order()
 {
    assert_eq!(same_as_the_old_engine(&FLAG_ATTRIBUTES), 48);
}

const PLACEMENT_FLAGS: [&[&str]; 6] = [
    &["Rig r:\n    Lamp*\n", "Rig s:\n    Lamp\n"],
    &["Rig r:\n    Lamp**\n"],
    &["Rig r:\n    Lamp~\n"],
    &["Rig r:\n    Lamp* | strobe\n"],
    &["Rig r:\n    Lamp*/0.5 | !strobe\n"],
    &["Rig r:\n    Lamp* as a | level=0.5\n    Lamp | level=a.level\n"],
];

#[test]
fn a_placement_flag_sets_its_bool_attribute_and_draws_unknown_flag_or_duplicate_attribute_as_the_old_engine_does_beside_a_valid_twin_in_every_order()
 {
    assert_eq!(same_as_the_old_engine(&PLACEMENT_FLAGS), 16);
}

const MEMBER_DOT_CALLS: [&[&str]; 11] = [
    &["let xs = [1, 2]\n", "Clip c:\n    Note/(xs.size())\n"],
    &["let xs = [1, 2]\n", "Clip c:\n    Note/((xs).capacity())\n"],
    &["Clip c:\n    Note/([1, 2][0].size())\n"],
    &["let xs = [1, 2]\n", "Clip c:\n    Note/(xs.keys())\n"],
    &["state q([]): i32[..4]\n", "Clip c:\n    Note/(q.size())\n"],
    &["Clip c:\n    Note/([1, 2].size() + [1, 2][5])\n"],
    &[
        "fn f(v): v.values()\n",
        "Clip c:\n    Note/(f(1) + f([1]))\n",
    ],
    &["fn f(v): v.size()\n", "let x = f([1])\n"],
    &["Clip c:\n    Note/(1.5.bytes() + true.keys())\n"],
    &[
        "let xs = [[1], [2]]\n",
        "Clip c | gain=xs[0].capacity():\n    Note\n",
    ],
    &["let xs = [1, 2]\n", "Clip c:\n    Note/(xs.size)\n"],
];

#[test]
fn a_member_called_with_no_arguments_draws_not_callable_on_an_array_and_unknown_name_otherwise_beside_a_valid_twin_in_every_order()
 {
    assert_eq!(same_as_the_old_engine(&MEMBER_DOT_CALLS), 54);
}

const MEMBER_CALLS_IN_USED_LETS: [&[&str]; 6] = [
    &[
        "let xs = [1, 2]\n",
        "let n = xs.size()\n",
        "Clip c:\n    Note/(n)\n",
    ],
    &[
        "let xs = [1, 2]\n",
        "let n = xs.keys()\n",
        "let m = n + 1\n",
        "Clip c:\n    Note/(m)\n",
    ],
    &["let n = [1, 2][0].keys()\n", "Clip c:\n    Note/(n)\n"],
    &["let n = [1, 2].map(1)\n", "Clip c | gain=n:\n    Note\n"],
    &[
        "let xs = [1, 2]\n",
        "let n = xs.capacity(1 + true)\n",
        "Clip c:\n    Note/(n)\n",
    ],
    &["let n = 1.5.bytes()\n", "Clip c:\n    Note/(n + 1)\n"],
];

#[test]
fn a_member_called_in_a_used_lets_value_draws_what_one_in_a_placement_draws_beside_a_valid_twin_in_every_order()
 {
    assert_eq!(same_as_the_old_engine(&MEMBER_CALLS_IN_USED_LETS), 186);
}

const MEMBER_CALLS_IN_UNUSED_LETS: [(&[&str], &[Code]); 7] = [
    (&["let xs = [1, 2]\n", "let n = xs.size()\n"], &[]),
    (&["let n = [1, 2][0].keys()\n"], &[]),
    (&["let n = [1, 2].map(1)\n"], &[]),
    (
        &["let xs = [1, 2]\n", "let n = xs.capacity(1 + true)\n"],
        &[],
    ),
    (
        &[
            "let xs = [1, 2]\n",
            "let n = xs.size()\n",
            "let m = n + 1\n",
        ],
        &[],
    ),
    (&["fn f(): [1, 2].size()\n"], &[]),
    (
        &[
            "let xs = [1, 2]\n",
            "let n = xs.size()\n",
            "Clip c:\n    Note/(xs[5])\n",
        ],
        &[Code::IndexOutOfRange],
    ),
];

#[test]
fn a_member_called_in_a_value_nothing_uses_draws_nothing_beside_a_valid_twin_in_every_order() {
    assert_eq!(drawn_codes(&MEMBER_CALLS_IN_UNUSED_LETS), 66);
}

const MEMBER_DOT_CALLS_WITH_ARGUMENTS: [&[&str]; 4] = [
    &[
        "let xs = [1, 2]\n",
        "Clip c:\n    Note/(xs.capacity(1 + true))\n",
    ],
    &["let x = 1\n", "Clip c:\n    Note/(x.size([1][4]))\n"],
    &["let xs = [1, 2]\n", "Clip c:\n    Note/(xs.keys(q))\n"],
    &[
        "let xs = [1, 2]\n",
        "Clip c:\n    Note/(xs.size(xs.size()))\n",
    ],
];

#[test]
fn a_member_called_with_arguments_leaves_them_unread_beside_a_valid_twin_in_every_order() {
    assert_eq!(same_as_the_old_engine(&MEMBER_DOT_CALLS_WITH_ARGUMENTS), 24);
}

const ARRAY_MAPS: [&[&str]; 4] = [
    &["let xs = [1, 2]\n", "Clip c:\n    Note/(xs.map(1))\n"],
    &["let x = 1\n", "Clip c:\n    Note/(x.map(1 + true))\n"],
    &[
        "let xs = [1, 2]\n",
        "Clip c:\n    Note/((xs).map(xs.size()) + 1)\n",
    ],
    &["fn f(v): v.map(1)\n", "Clip c:\n    Note/(f([1]))\n"],
];

#[test]
fn an_arrays_map_draws_unbuilt_construct_at_the_call_and_a_scalars_draws_unknown_name_beside_a_valid_twin_in_every_order()
 {
    assert_eq!(same_as_the_old_engine(&ARRAY_MAPS), 24);
}

const ARRAY_LET_ATTRIBUTES: [&[&str]; 6] = [
    &["let xs = [1, 2]\n", "Clip c:\n    Note/(xs.foo)\n"],
    &["let xs = [1, 2]\n", "Clip c:\n    Note/(xs.keys + 1)\n"],
    &["let xs = [[1], [2]]\n", "Clip c:\n    Note/(xs.foo[0])\n"],
    &[
        "let xs = [1, 2]\n",
        "let y = xs.values\n",
        "Clip c:\n    Note/(y)\n",
    ],
    &["let xs = [1, 2]\n", "Clip c | gain=xs.foo:\n    Note\n"],
    &[
        "let xs = [1, 2]\n",
        "let foo = 1\n",
        "Clip c:\n    Note/(xs.foo + xs.size)\n",
    ],
];

#[test]
fn an_array_lets_attribute_other_than_its_size_or_capacity_draws_unknown_attribute_beside_a_valid_twin_in_every_order()
 {
    assert_eq!(same_as_the_old_engine(&ARRAY_LET_ATTRIBUTES), 72);
}

const LET_MEMBER_READS: [&[&str]; 7] = [
    &[
        "let x = 1\n",
        "Clip c:\n    Note/(x.past + x.map + x.bytes)\n",
    ],
    &["let xs = [1, 2]\n", "Clip c:\n    Note/(xs.dropped)\n"],
    &[
        "let x = 1.5\n",
        "Clip c:\n    Note/(i32(x.dropped) + x.kind)\n",
    ],
    &[
        "let xs = [1, 2]\n",
        "Clip c:\n    Note/(xs.kind + xs.past)\n",
    ],
    &["let xs = [1, 2]\n", "Clip c:\n    Note/(xs.map)\n"],
    &["let x = 1\n", "Clip c:\n    Note/(x.foo)\n"],
    &["let x = 1\n", "Clip c | gain=x.bar:\n    Note/(x)\n"],
];

#[test]
fn a_lets_member_read_draws_unknown_attribute_or_an_arrays_map_unbuilt_construct_beside_a_valid_twin_in_every_order()
 {
    assert_eq!(same_as_the_old_engine(&LET_MEMBER_READS), 42);
}

const PATHS_FROM_NAMES_OF_NO_INSTANCE: [&[&str]; 9] = [
    &["Clip c:\n    Note/(stage.wash)\n"],
    &["Clip c | gain=stage.wash:\n    Note\n"],
    &["Scene dusk:\n    Cue/0 Cue/1\n    Dims | depth=0.5: stage.wash\n"],
    &["Clip c:\n    Note/(foo.bar + 1)\n"],
    &["fn f(): 1\n", "Clip c:\n    Note/(f.bar)\n"],
    &["let a = 1\n", "let a = 2\n", "Clip c:\n    Note/(a.bar)\n"],
    &["Clip c:\n    Note/(stage.wash.x[1] + stage.size)\n"],
    &["let x = stage.wash\n", "Clip c:\n    Note/(x)\n"],
    &["fn g(): stage.wash + true\n", "Clip c:\n    Note/(g())\n"],
];

#[test]
fn a_path_from_a_name_of_no_instance_is_a_placeholder_beside_its_names_fault_and_a_valid_twin_in_every_order()
 {
    assert_eq!(same_as_the_old_engine(&PATHS_FROM_NAMES_OF_NO_INSTANCE), 52);
}

const FLAGGED_PATTERN_USES: [&[&str]; 7] = [
    &["pattern P():\n    Lamp\n", "Rig r:\n    P*\n"],
    &[
        "pattern P(default v):\n    Lamp/v\n",
        "Rig r:\n    P* /0.5\n    Lamp*\n",
    ],
    &["pattern P():\n    Lamp\n", "Rig r:\n    P*~ | x=1\n"],
    &[
        "pattern P():\n    Lamp\n",
        "Rig r:\n    Lamp as a\n    P*\n    Dims*: a\n",
    ],
    &["Rig r:\n    Lamp as a\n    Dims*~: a\n    Dims: a\n"],
    &[
        "pattern P():\n    Lamp\n",
        "Rig r:\n    for i in 0..2:\n        P*\n",
    ],
    &[
        "pattern P():\n    Lamp\n",
        "Rig r:\n    P*\n",
        "Rig s:\n    Lamp* | strobe=true\n",
    ],
];

#[test]
fn a_flag_on_a_pattern_use_draws_use_form_and_one_on_a_wire_unknown_flag_beside_a_flagged_placement_and_a_valid_twin_in_every_order_525()
 {
    assert_eq!(same_as_the_old_engine(&FLAGGED_PATTERN_USES), 56);
}

const CELL_CAPACITIES_AND_CLOCKS: [&[&str]; 12] = [
    &[
        "channel[16] hits(0)\n",
        "Rig r:\n    for i in 0..4:\n        Lamp/(0.2 * i) when hits > i\n",
    ],
    &["stream[4] s(0.5)\n", "Rig r:\n    Lamp/s\n"],
    &["state[4] s(0.5)\n", "Rig r:\n    Lamp/s\n"],
    &["stream[4] s = 1 + 2\n", "Rig r:\n    Lamp/s\n"],
    &["channel[3000000000] s(0)\n", "Rig r:\n    Lamp/s\n"],
    &["channel[0] s(0)\n", "Rig r:\n    Lamp/s\n"],
    &[
        "state a(1)\n",
        "state b on zz = a + 1\n",
        "Rig r:\n    Lamp/b\n",
    ],
    &[
        "state a(1)\n",
        "state b on a = a + true\n",
        "Rig r:\n    Lamp/b\n",
    ],
    &[
        "state b(0) on a = b.past(1, 0) + 1\n",
        "Rig r:\n    Lamp/b\n",
    ],
    &[
        "let x = 1 + true\n",
        "state a(1)\n",
        "state b on (a, x) = a + 1\n",
        "Rig r:\n    Lamp/b\n",
    ],
    &[
        "state a(1)\n",
        "state b(0) on a = c.past(1, 0) + 1\n",
        "state c(0) = b.past(1, 0)\n",
        "Rig r:\n    Lamp/(b + c)\n",
    ],
    &[
        "state a(1)\n",
        "state b on (1 +) = a + 1\n",
        "Rig r:\n    Lamp/b\n",
    ],
];

#[test]
fn a_cells_capacity_draws_its_misplacement_or_size_and_a_clock_its_unbuilt_construct_reading_nothing_beside_a_valid_twin_in_every_order()
 {
    assert_eq!(same_as_the_old_engine(&CELL_CAPACITIES_AND_CLOCKS), 354);
}

const BUNDLE_LOOPS: [&[&str]; 8] = [
    &["Rig r:\n    [for i in 0..3: Lamp/(i + true)]\n"],
    &["Rig r:\n    [for i in 0..3: Lamp/(i * 0.3) Lamp as x]\n"],
    &[
        "let washes = [0.2, 0.5, 0.8]\n",
        "Rig stage:\n    [Lamp/0.2 Lamp/0.5 Lamp/0.8]\n    [for i in 0..3: Lamp/(0.2 + i * 0.3)]\n    [for i, p in washes: (Lamp/p | hue=1.0 - i * 0.2)]\n",
    ],
    &[
        "Rig r:\n    [for i in 0..3: Lamp/(0.2 + i * 0.3) Lamp/i]\n    for i in 0..3:\n        [for j in 0..i: Lamp/(j + true)]\n",
    ],
    &["Rig r:\n    Lamp [for i in 0..3: Lamp/i]\n"],
    &[
        "pattern P(default n):\n    [for i in 0..n: Lamp/(i * 0.1)]\n",
        "Rig r:\n    P/0\n    P/2\n    P/0\n",
    ],
    &[
        "pattern Q():\n    Lamp\n",
        "Rig r:\n    [for i in 0..2: Q]\n",
    ],
    &["Rig r:\n    [for i in [1, 2][5..]: Lamp/i]\n"],
];

#[test]
fn a_bundle_for_expands_as_a_for_over_its_members_and_draws_empty_bundle_where_it_runs_no_iteration_beside_a_valid_twin_in_every_order()
 {
    assert_eq!(same_as_the_old_engine(&BUNDLE_LOOPS), 28);
}

const UNRUN_BODY_WIRES: [&[&str]; 9] = [
    &["Clip intro:\n    for s in 0..0:\n        Starts | foo=1: s\n"],
    &["Clip intro:\n    for s in 0..0:\n        Starts | gain=1 + true: s\n"],
    &["Clip intro:\n    for s in 0..0:\n        Starts/(1 + true) when 2 + true: s\n"],
    &["Clip intro:\n    for s in 0..0:\n        Starts* | foo=1: s\n"],
    &["Clip intro:\n    for s in 0..0:\n        Starts*~: intro\n"],
    &["Rig r:\n    for i in 0..0:\n        Lamp*~ | foo=1 strobe\n"],
    &[
        "pattern P():\n    Lamp\n",
        "Rig r:\n    for i in 0..0:\n        P*\n",
    ],
    &[
        "state section(0)\n",
        "let sections = [verse, chorus, bridge]\n",
        "Clip intro:\n    for s in sections:\n        Starts: s\n",
        "Clip outro:\n    Plays: sections[section]\n",
    ],
    &["Clip intro:\n    for s in 0..0:\n        Starts | foo=zz: s\n"],
];

#[test]
fn a_wire_in_a_body_no_iteration_runs_draws_its_unknown_flag_and_attribute_names_whatever_its_destination_beside_a_valid_twin_in_every_order()
 {
    assert_eq!(same_as_the_old_engine(&UNRUN_BODY_WIRES), 140);
}

const NAMED_WIRES: [&[&str]; 10] = [
    &[
        "Rig stage:\n    Lamp as wash\n",
        "Scene night:\n    Dims as soften: stage.wash\n",
        "Scene dawn | hold=night.soften.depth * 10:\n    Cue/0\n",
    ],
    &[
        "Rig stage:\n    Lamp as wash\n",
        "Scene night:\n    Dims as soften | depth=0.4: stage.wash\n    Dims as soften: stage.wash\n    Cue as soften\n",
    ],
    &[
        "Rig stage:\n    Lamp as wash\n",
        "Scene night:\n    Cue as soften\n    Dims as soften: stage.wash\n",
    ],
    &[
        "Rig stage:\n    Lamp as wash\n",
        "Scene night:\n    Dims as hold: stage.wash\n",
    ],
    &[
        "Rig stage:\n    Lamp as wash\n",
        "Scene night:\n    Dims as soften | depth=0.4: stage.wash\n",
        "Scene dawn | hold=night.soften.depth.x:\n    Cue/0\n",
    ],
    &[
        "Rig stage:\n    Lamp as wash\n",
        "Scene night:\n    Dims as soften | depth=true: stage.wash\n",
        "Scene dawn | hold=night.soften.depth:\n    Cue/0\n",
    ],
    &[
        "Rig stage:\n    Lamp as wash\n",
        "Scene night:\n    Dims as soften | depth=night.soften.depth: stage.wash\n",
    ],
    &[
        "Rig stage:\n    Lamp as wash\n",
        "Scene night:\n    Dims as soften | depth=0.4: stage.wash\n",
        "Scene dawn | hold=night.soften.depth + true:\n    Cue/0\n",
    ],
    &["Rig stage:\n    Lamp as soften\n    Scene:\n        Dims as soften: stage.soften\n"],
    &[
        "Rig stage:\n    Lamp as wash\n",
        "Scene night:\n    Dims as soften | depth=0.4: stage.wash\n    Dims as soften: stage.wash\n",
        "Scene dawn | hold=night.soften.foo:\n    Cue/0\n",
    ],
];

#[test]
fn a_named_wire_is_a_child_name_whose_attributes_a_path_reads_and_clashes_with_its_siblings_beside_a_valid_twin_in_every_order()
 {
    assert_eq!(same_as_the_old_engine(&NAMED_WIRES), 146);
}

const UNIT_ATTRIBUTE_READS: [&[&str]; 10] = [
    &[
        "Scene dusk | hold=8:\n    Cue as first /1 | fade=2\n",
        "Scene night | hold=dusk.hold * 1.5:\n    Cue/0 | fade=dusk.first.fade\n",
    ],
    &[
        "Scene dusk | hold=500ms:\n    Cue/0\n",
        "Scene night | hold=dusk.hold + 1:\n    Cue/0\n",
    ],
    &[
        "Scene dusk | hold=500ms:\n    Cue/0\n",
        "Scene night | hold=dusk.hold + 1s:\n    Cue/0\n",
    ],
    &[
        "Scene dusk | hold=500ms:\n    Cue/0\n",
        "Clip c:\n    Note/(dusk.hold)\n",
    ],
    &[
        "Scene dusk:\n    Cue/0\n",
        "Scene night | hold=dusk.hold * 2:\n    Cue/0\n",
    ],
    &[
        "Scene dusk | hold=500ms:\n    Cue/0\n",
        "let x = dusk.hold\n",
        "Scene night | hold=x:\n    Cue/0\n",
    ],
    &[
        "Scene dusk | hold=500ms:\n    Cue/0\n",
        "Scene night | hold=dusk.hold / 1ms:\n    Cue/0\n",
    ],
    &[
        "Scene dusk | hold=night.hold:\n    Cue/0\n",
        "Scene night | hold=dusk.hold:\n    Cue/0\n",
    ],
    &[
        "Scene dusk | hold=500ms:\n    Cue/0\n",
        "Clip c:\n    Note/(i32(dusk.hold / 1ms))\n",
    ],
    &[
        "Rig stage:\n    Lamp as wash\n",
        "Scene dusk | hold=8:\n    Cue as first /1 | fade=2\n",
        "Scene night | hold=dusk.hold * 1.5:\n    Cue/0 | fade=dusk.first.fade\n    Dims as soften | depth=0.4: stage.wash\n",
        "Scene dawn | hold=night.soften.depth * 10:\n    Cue/0\n",
    ],
];

#[test]
fn a_path_read_of_a_unit_attribute_is_a_value_of_its_family_beside_a_valid_twin_in_every_order() {
    assert_eq!(same_as_the_old_engine(&UNIT_ATTRIBUTE_READS), 192);
}

const FLAG_SET_READS: [&[&str]; 6] = [
    &[
        "Rig r:\n    Lamp* as a | level=0.5\n    Lamp | level=a.level hue=(if a.strobe then 1 else 0)\n",
    ],
    &["Rig r:\n    Lamp as a | !strobe\n    Lamp | hue=(if a.strobe then 1 else 0)\n"],
    &["Rig r:\n    Lamp as a | strobe\n    Lamp | hue=a.strobe\n"],
    &["Rig r:\n    Lamp* as a | strobe=false\n    Lamp | hue=(if a.strobe then 1 else 0)\n"],
    &["Rig r:\n    Lamp* as a\n    Lamp/(a.strobe + 1)\n"],
    &["Rig r:\n    Lamp as a | hue\n    Lamp/(a.hue)\n"],
];

#[test]
fn a_path_read_of_an_attribute_a_flag_sets_is_the_flags_bool_beside_a_valid_twin_in_every_order() {
    assert_eq!(same_as_the_old_engine(&FLAG_SET_READS), 12);
}
