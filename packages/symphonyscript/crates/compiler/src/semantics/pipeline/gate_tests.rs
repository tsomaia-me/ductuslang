use crate::compiler::Compiler;
use crate::syntax::front_end_goldens::source;
use symphonyscript_fixtures::Orders;

const TWIN: &str = "Clip twin:\n    Note/61 when true\n";

const GATES: [&[&str]; 30] = [
    &["state live(true)\n", "Clip c:\n    Note/60 when live\n"],
    &["Clip c:\n    Note/60 when 1\n"],
    &["Clip c:\n    Note/60 when [true, false]\n"],
    &["Clip c:\n    Note/60 when nope\n"],
    &["state n(3)\n", "Clip c:\n    Note/60 when n > 2\n"],
    &["state n(3)\n", "Clip c:\n    Note/60 when n\n"],
    &[
        "let a = [true, false]\n",
        "Clip c:\n    Note/60 when a[5]\n",
    ],
    &["Clip c:\n    Note/60 when (if 1 then true else false)\n"],
    &["Clip c:\n    Note/60 when Wave::Saw\n"],
    &["Clip c:\n    Note/60 when Saw\n"],
    &["fn f(x): x > 1\n", "Clip c:\n    Note/60 when f(2)\n"],
    &["fn f(x): x + 1\n", "Clip c:\n    Note/60 when f(2)\n"],
    &["Clip c:\n    Note/60 when 1 + true\n"],
    &["Clip c:\n    Note/(1 + true) when 2\n"],
    &[
        "state live(true)\n",
        "let g = live\n",
        "Clip c:\n    Note/60 when g\n",
    ],
    &[
        "state v(1.5)\n",
        "Clip c:\n    Note/60 when v > 300000000000\n",
    ],
    &["Clip c:\n    for i in 0..3:\n        Note/60 when i > 1\n"],
    &["Clip c:\n    for i in 0..3:\n        Note/60 when [true, false][i]\n"],
    &["Clip c:\n    for i in 0..3:\n        Note/60 when i\n"],
    &["Clip c:\n    Note/60 when 1bars\n"],
    &[
        "state live(true)\n",
        "Clip a:\n    Note/60\n    Plays when live: b\n",
        "Clip b:\n    Note/62\n",
    ],
    &["Clip a:\n    Plays when 3: b\n", "Clip b:\n    Note/62\n"],
    &[
        "Clip a:\n    Plays/2 when [1][4] > 0: b\n",
        "Clip b:\n    Note/62\n",
    ],
    &[
        "pattern Arp(default v):\n    Note/60 when v > 1\n",
        "Clip c:\n    Arp/1\n    Arp/5\n",
    ],
    &[
        "pattern Arp(default v):\n    Note/60 when v\n",
        "Clip c:\n    Arp/1\n    Arp/true\n",
    ],
    &[
        "state live(true)\n",
        "pattern Arp(default v):\n    Note/60 when live\n",
        "Clip c:\n    Arp/1\n",
    ],
    &[
        "state live(true)\n",
        "state other = not live\n",
        "Clip c:\n    Note/60 when other\n",
    ],
    &["let x = 2\n", "Clip c:\n    Note/60 when x / 0 > 1\n"],
    &["Clip c:\n    Note/60 when (for i in 0..3 into a from false: a or i > 1)\n"],
    &[
        "state live(true)\n",
        "Clip c:\n    Note/60 when live.past(1, false)\n",
    ],
];

const COMPARED: [&str; 26] = [
    "1e30 * 1e30 > 1",
    "1e200 * 1e200 > 1",
    "2 / 0 > 1",
    "0 / 0 > 1",
    "-(2 / 0) < 1",
    "2.0 / 0 is 1.0",
    "1 < 2 / 0",
    "(2 / 0) * 0 > 1",
    "1e30 * 1e30 * 0 > 1",
    "f32(2 / 0) > 1",
    "2 / 0 > 1.0f32",
    "2 / 0 > 1i64",
    "[2 / 0][0] > 1",
    "(if true then 2 / 0 else 1) > 1",
    "1e39 > 1",
    "2 ** 2000 > 1.0",
    "16777217 > 1.5",
    "16777217 + 0.5 > 1.5",
    "0.1 + 0.2 is 0.3",
    "1e39 * 0 > 1",
    "(1e30 * 1e30) / 1e30 > 1e29",
    "2 / 0 > 2 / 0",
    "[1.5, 2 / 0][1] > 1",
    "2 / 0 > 1.5f64",
    "f64(1) / 0 > 1",
    "2 ** 200 > 1.5",
];

fn twins(items: &[&str]) -> Vec<String> {
    let mut all: Vec<&str> = items.to_vec();
    all.push(TWIN);
    Orders::Every
        .of(&all)
        .into_iter()
        .map(|order| order.concat())
        .collect()
}

#[test]
fn a_gate_on_a_node_or_a_wire_is_a_bool_that_may_read_cells_and_draws_what_the_old_engine_draws_beside_a_valid_twin_in_every_order()
 {
    let vocabulary = symphonyscript_fixtures::vocabulary();
    let old = Compiler::new(&vocabulary)
        .expect("the vocabulary compiles")
        .on_the_old_engine();
    let new = Compiler::new(&vocabulary).expect("the vocabulary compiles");
    let mut programs = 0;
    for items in GATES {
        for text in twins(items) {
            let sources = [source(&text)];
            let found = new
                .checked_by_new_core(&sources)
                .unwrap_or_else(|| panic!("the new core takes {text}"));
            assert_eq!(found, old.check(&sources), "{text}");
            programs += 1;
        }
    }
    assert_eq!(programs, 200);
}

#[test]
fn a_flexible_comparison_operand_takes_its_default_float_type_and_a_value_that_does_not_fit_it_draws_as_the_old_engine_draws()
 {
    let vocabulary = symphonyscript_fixtures::vocabulary();
    let old = Compiler::new(&vocabulary)
        .expect("the vocabulary compiles")
        .on_the_old_engine();
    let new = Compiler::new(&vocabulary).expect("the vocabulary compiles");
    for compared in COMPARED {
        for text in [
            format!("Clip c:\n    Note/60 | accent=({compared})\n"),
            format!("Clip c:\n    Note/(if {compared} then 1 else 2)\n"),
            format!("let x = {compared}\nClip c:\n    Note/60 | accent=x\n"),
            format!("state v(1.0)\nClip c:\n    Note/60 | accent=(v > 1 and {compared})\n"),
        ] {
            let sources = [source(&text)];
            let found = new
                .checked_by_new_core(&sources)
                .unwrap_or_else(|| panic!("the new core takes {text}"));
            assert_eq!(found, old.check(&sources), "{text}");
        }
    }
}

const DECLINED: [&str; 3] = [
    "state big(1)\npattern Gated(default active):\n    Note when active\nClip c:\n    Gated/(big > u8(255) + 4)\n",
    "state note(60)\nClip c:\n    Note when (if q then 300000000000 else [1, 2][5])\n",
    "Clip a:\n    Plays when to.gain > 0.5: b\nClip b:\n    Note/1\n",
];

#[test]
fn a_gate_reading_a_reactive_pattern_parameter_or_a_choice_beside_its_refused_condition_stays_with_the_old_engine()
 {
    let vocabulary = symphonyscript_fixtures::vocabulary();
    let new = Compiler::new(&vocabulary).expect("the vocabulary compiles");
    for text in DECLINED {
        assert!(new.checked_by_new_core(&[source(text)]).is_none(), "{text}");
    }
}
