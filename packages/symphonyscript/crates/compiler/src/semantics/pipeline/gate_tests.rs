use crate::compiler::Compiler;
use crate::syntax::front_end_goldens::source;
use symphonyscript_fixtures::Orders;

const TWIN: &str = "Clip twin:\n    Note/61 when true\n";

const GATES: [&[&str]; 29] = [
    &["state live(true)\n", "Clip c:\n    Note/60 when live\n"],
    &["Clip c:\n    Note/60 when 1\n"],
    &["Clip c:\n    Note/60 when [true, false]\n"],
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
    assert_eq!(programs, 198);
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

const DECLINED: [&str; 5] = [
    "Clip c:\n    Note/60 when nope\n",
    "Plant boiler:\n    when nope:\n        Valve/0.3\n",
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

const WHENS: [&[&str]; 19] = [
    &[
        "state alarm(false)\n",
        "Plant boiler:\n    Valve/0.1\n    when alarm:\n        Valve/0.3 Valve/0.5\n    else:\n        Valve/0.3\n",
    ],
    &[
        "state phase(0)\n",
        "Plant boiler:\n    when:\n        phase is 0: Valve/0.6\n        phase is 1: Valve/0.7\n        else: Valve/0.9\n",
    ],
    &["Plant boiler:\n    when 1:\n        Valve/0.3\n"],
    &["Plant boiler:\n    when [true]:\n        Valve/0.3\n"],
    &["Plant boiler:\n    when true:\n        Valve/2\n"],
    &["Plant boiler:\n    when true:\n        Note/1\n"],
    &["Clip c:\n    when true:\n        Note/1\n    else:\n        Note/(1 + true)\n"],
    &[
        "state n(3)\n",
        "Clip c:\n    when:\n        n > 2: Note/1\n        n: Note/2\n",
    ],
    &["Clip c:\n    when [1][3] > 0:\n        Note/1\n"],
    &["Plant boiler:\n    when true:\n        when false:\n            Valve/0.3\n"],
    &[
        "Clip a:\n    when true:\n        Plays: b\n",
        "Clip b:\n    Note/1\n",
    ],
    &["Clip c:\n    when true:\n        Note/1 when 2\n"],
    &[
        "Clip c:\n    when:\n        true: Note/1 Note/(2 + true)\n        1: Note/3\n        else: Note/4\n",
    ],
    &["Clip c:\n    when true: Note/1\n"],
    &["Clip c:\n    when true: Note/1\n    else: Note/2\n"],
    &["Plant p:\n    when:\n        true: Note/1\n"],
    &[
        "Clip c:\n    when:\n        [1][4] > 1: Note/1\n        else:\n            Note/2\n            Note/3\n",
    ],
    &[
        "let k = 2\n",
        "Clip c:\n    when k / 0 > 1:\n        Note/1\n",
    ],
    &[
        "state live(true)\n",
        "Clip c:\n    when live:\n        Note/1 when not live\n    Note/2\n",
    ],
];

#[test]
fn a_when_block_places_every_arm_and_its_guards_are_gates_drawing_what_the_old_engine_draws_beside_a_valid_twin_in_every_order()
 {
    let vocabulary = symphonyscript_fixtures::vocabulary();
    let old = Compiler::new(&vocabulary)
        .expect("the vocabulary compiles")
        .on_the_old_engine();
    let new = Compiler::new(&vocabulary).expect("the vocabulary compiles");
    let mut programs = 0;
    for items in WHENS {
        for text in twins(items) {
            let sources = [source(&text)];
            let found = new
                .checked_by_new_core(&sources)
                .unwrap_or_else(|| panic!("the new core takes {text}"));
            assert_eq!(found, old.check(&sources), "{text}");
            programs += 1;
        }
    }
    assert_eq!(programs, 62);
    let unrun = "Clip c:\n    for x in q:\n        Note/(x)\n    when true:\n        Note/(1)\n";
    assert!(
        new.checked_by_new_core(&[source(unrun)]).is_none(),
        "{unrun}"
    );
}

const PARENS: [&[&str]; 8] = [
    &["Plant fill:\n    Valve/0 (Valve/0.5 | open=2) (Controller/0.4 | gain=true)\n"],
    &["Plant fill:\n    (Valve/0.5 when 1) Valve/0.1\n"],
    &[
        "state live(true)\n",
        "Plant fill:\n    (Valve/0.5 when live) Valve/0.1\n",
    ],
    &["Clip c:\n    (Note/60 | velocity=0.5) (Note/61 | pitch=2)\n"],
    &["Clip c:\n    (Note/60 | nope=1) Note/2\n"],
    &["Clip c:\n    when true:\n        (Note/60 | velocity=0.5) Note/2\n"],
    &["Clip c:\n    (Note/(1 + true) | velocity=0.5)\n"],
    &["Clip c:\n    (Valve/1 | open=0.5)\n"],
];

#[test]
fn a_parenthesized_placement_on_a_line_is_one_child_drawing_what_the_old_engine_draws_beside_a_valid_twin_in_every_order()
 {
    let vocabulary = symphonyscript_fixtures::vocabulary();
    let old = Compiler::new(&vocabulary)
        .expect("the vocabulary compiles")
        .on_the_old_engine();
    let new = Compiler::new(&vocabulary).expect("the vocabulary compiles");
    let mut programs = 0;
    for items in PARENS {
        for text in twins(items) {
            let sources = [source(&text)];
            let found = new
                .checked_by_new_core(&sources)
                .unwrap_or_else(|| panic!("the new core takes {text}"));
            assert_eq!(found, old.check(&sources), "{text}");
            programs += 1;
        }
    }
    assert_eq!(programs, 20);
}

const LOOP_FLOATS: [&[&str]; 6] = [
    &[
        "let ramp = [0.2, 0.5, 0.8]\n",
        "Rig fade_in:\n    for i, p in ramp:\n        Lamp/p | hue=1.0/ - i * 0.2\n",
        "Rig twin_in:\n    for i, p in ramp:\n        Lamp/p | hue=1.0 - i * 0.2\n",
    ],
    &[
        "Rig fade_in:\n    for i in 0..3:\n        Lamp/0.5 | hue=0.0 / (i * 0)\n",
        "Rig twin_in:\n    for i in 0..3:\n        Lamp/0.5 | hue=1.0 - i * 0.2\n",
    ],
    &[
        "Rig fade_in:\n    for i in 0..3:\n        Lamp/0.5 | hue=1.0 / i\n",
        "Rig twin_in:\n    for i in 1..3:\n        Lamp/0.5 | hue=1.0 / i\n",
    ],
    &[
        "Rig fade_in:\n    for i in 0..3:\n        Lamp/0.5 | hue=1.0 / -i\n",
        "Rig twin_in:\n    for i in 0..3:\n        Lamp/0.5 | hue=1.0 / (i + 1)\n",
    ],
    &[
        "Rig fade_in:\n    for i in 16777215..16777218:\n        Lamp/0.5 | hue=i\n",
        "Rig twin_in:\n    for i in 0..3:\n        Lamp/0.5 | hue=i\n",
    ],
    &[
        "let ramp = [0.2, 0.5, 0.8]\n",
        "Rig fade_in:\n    for i, p in ramp:\n        Lamp/p | hue=p / i\n",
    ],
];

#[test]
fn an_integer_loop_name_read_as_a_float_converts_and_a_value_that_does_not_fit_draws_at_each_use_as_the_old_engine_draws_512()
 {
    let vocabulary = symphonyscript_fixtures::vocabulary();
    let old = Compiler::new(&vocabulary)
        .expect("the vocabulary compiles")
        .on_the_old_engine();
    let new = Compiler::new(&vocabulary).expect("the vocabulary compiles");
    let mut programs = 0;
    for items in LOOP_FLOATS {
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
    assert_eq!(programs, 16);
}

const BUNDLES: [&[&str]; 9] = [
    &["Rig flash:\n    [Lamp/0.2 Lamp/0.5 Lamp/0.8]\n"],
    &["Rig flash:\n    [Lamp/0.2 Lamp/(1 + true)]\n"],
    &["Rig flash:\n    [Lamp/0.2 Note/1]\n"],
    &["Rig flash:\n    [Lamp/0.2 (Lamp/0.5 | hue=true)]\n"],
    &["Rig flash:\n    [Lamp/0.2 Nope/1]\n"],
    &["Rig flash:\n    [Lamp/0.2]\n    [Lamp/0.4 Lamp/0.6]\n"],
    &["Clip c:\n    [Note/1 Note/2 Note/3]\n    Note/4\n"],
    &[
        "state lit(0.5)\n",
        "Rig flash:\n    [Lamp/lit (Lamp/0.5 | hue=lit)]\n",
    ],
    &["Rig flash:\n    when true:\n        [Lamp/0.2 Lamp/2]\n"],
];

#[test]
fn a_bundles_members_are_children_of_its_parent_drawing_what_the_old_engine_draws_beside_a_valid_twin_in_every_order()
 {
    let vocabulary = symphonyscript_fixtures::vocabulary();
    let old = Compiler::new(&vocabulary)
        .expect("the vocabulary compiles")
        .on_the_old_engine();
    let new = Compiler::new(&vocabulary).expect("the vocabulary compiles");
    let mut programs = 0;
    for items in BUNDLES {
        for text in twins(items) {
            let sources = [source(&text)];
            let found = new
                .checked_by_new_core(&sources)
                .unwrap_or_else(|| panic!("the new core takes {text}"));
            assert_eq!(found, old.check(&sources), "{text}");
            programs += 1;
        }
    }
    assert_eq!(programs, 22);
}
