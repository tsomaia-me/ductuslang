use symphonyscript_core::{Count, Vocabulary};

use crate::compiler::Compiler;
use crate::diagnostics::Code;
use crate::syntax::front_end_goldens::source;

const FIXTURE_WIRES: [&str; 14] = [
    "Clip a:\n    Note/60\n    Plays: b\nClip b:\n    Note/62\n",
    "Clip a:\n    Plays | gain=0.5 transpose=2: b\nClip b:\n    Note/1\n",
    "Clip a:\n    Plays/2: b\nClip b:\n    Note/1\n",
    "Clip a:\n    Plays/true: b\nClip b:\n    Note/1\n",
    "Clip a:\n    Plays | volume=1: b\nClip b:\n    Note/1\n",
    "Clip a:\n    Plays | gain=1 gain=2: b\nClip b:\n    Note/1\n",
    "Clip a:\n    Triggers: b\nClip b:\n    Note/1\n",
    "Clip a:\n    Starts: nowhere\n",
    "Clip a:\n    Plays: a\n",
    "let g = 0.25\nClip a:\n    Note/60\n    Plays/g: b\n    Note/61\nClip b:\n    Plays/(g + 1): a\n",
    "Scene a:\n    Triggers: b\nScene b:\n    Triggers: a\n",
    "state q(0.5)\nClip a:\n    Plays/q: b\nClip b:\n    Note/1\n",
    "Filter f:\n    Feeds: g\nFilter g:\n    Feeds: f\n",
    "Filter f:\n    Feeds: f\nFilter g:\n    Feeds: g\n    Feeds: f\n",
];

const BOUNDED_WIRES: [&str; 6] = [
    "Root r:\n    Item\n    Link: r\n",
    "Root r:\n    Item\n",
    "Root r:\n    Item\n    Link: r\n    Link: r\n",
    "Root r:\n    Item\n    Link: r\n    Pick | level=2: a\nA a\n",
    "Root r:\n    Item\n    Link: r\n    Pick: z\n",
    "Root r:\n    Item\n    Pick: b\nB b\nA a\n",
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
fn a_wire_to_a_named_instance_checks_its_attributes_endpoints_counts_and_cycles_as_the_old_engine_does()
 {
    assert_engines_agree(&symphonyscript_fixtures::vocabulary(), &FIXTURE_WIRES);
    assert_engines_agree(
        &symphonyscript_fixtures::bounded_vocabulary(),
        &BOUNDED_WIRES,
    );
}

const CHOSEN_WIRES: [&str; 22] = [
    "state look(0)\nClip a:\n    Note/1\nClip b:\n    Note/2\nClip c:\n    Note/3\n    Plays: [a, b][look]\n",
    "state f(false)\nClip a:\n    Note/1\nClip b:\n    Note/2\nClip c:\n    Note/3\n    Plays: if f then a else b\n",
    "Clip a:\n    Note/1\nClip b:\n    Note/2\nClip c:\n    Note/3\n    Plays: if true then a else b\n",
    "Clip a:\n    Note/1\nClip b:\n    Note/2\nClip c:\n    Note/3\n    Plays: [a, b][1]\n",
    "state look(0.5)\nClip a:\n    Note/1\nClip b:\n    Note/2\nClip c:\n    Note/3\n    Plays: [a, b][look]\n",
    "Clip a:\n    Note/1\nClip c:\n    Note/3\n    Plays: [a, a][2]\n",
    "state f(1)\nClip a:\n    Note/1\nClip b:\n    Note/2\nClip c:\n    Note/3\n    Plays: if f then a else b\n",
    "state look(0)\nClip a:\n    Note/1\nClip c:\n    Note/3\n    Plays: [a, c][look]\n",
    "state look(0)\nClip a:\n    Note/1\nClip c:\n    Note/3\n    Plays: [a, nope][look]\n",
    "state look(0)\nFilter f:\n    Feeds: [g, h][look]\nFilter g:\n    Feeds: f\nFilter h\n",
    "Filter f:\n    Feeds: [g, h][1]\nFilter g:\n    Feeds: f\nFilter h\n",
    "Filter f:\n    Feeds: [g, h][0]\nFilter g:\n    Feeds: f\nFilter h\n",
    "state look(0)\nClip a:\n    Triggers: [b, c][look]\nClip b:\n    Note/1\nClip c:\n    Note/1\n",
    "state look(0)\nClip a:\n    Note/1\n    Plays: if look > 0 then [a, b][look] else c\nClip b:\n    Note/2\nClip c:\n    Note/3\n",
    "Clip a:\n    Note/1\n    Plays: if false then [a, b][5] else c\nClip b:\n    Note/2\nClip c:\n    Note/3\n",
    "Clip a:\n    Note/1\n    Plays: [b, c][-1]\nClip b:\n    Note/2\nClip c:\n    Note/3\n",
    "let i = 1\nClip a:\n    Note/1\n    Plays: [b, c][i + 300000000000]\nClip b:\n    Note/2\nClip c:\n    Note/3\n",
    "Clip a | level=1:\n    Note/1\n    Plays: [b, c][a.level]\nClip b:\n    Note/2\nClip c:\n    Note/3\n",
    "state look(0)\nClip a:\n    Note/1\n    Plays: [b, c][[1, 2]]\nClip b:\n    Note/2\nClip c:\n    Note/3\n",
    "Clip a:\n    Note/1\n    Plays: if [true] then b else c\nClip b:\n    Note/2\nClip c:\n    Note/3\n",
    "fn pick(x): x - 1\nClip a:\n    Note/1\n    Plays: [b, c][pick(1)]\nClip b:\n    Note/2\nClip c:\n    Note/3\n",
    "state look(0)\nlet w = look\nClip a:\n    Note/1\n    Plays: [b, c][w]\nClip b:\n    Note/2\nClip c:\n    Note/3\n",
];

const CHOSEN_BOUNDED_WIRES: [&str; 7] = [
    "state sw(true)\nRoot r:\n    Item\n    Link: if sw then r else r\n",
    "Root r:\n    Item\n    Link: if true then r else r\n",
    "state sw(true)\nRoot r:\n    Item\n    Link: if sw then r else z\n",
    "state k(0)\nRoot r:\n    Item\n    Link: [r, r][k]\n    Link: r\n",
    "Root r:\n    Item\n    Link: if 1 then r else r\n",
    "Root r:\n    Item\n    Link: [r, r][2.5]\n",
    "Root r:\n    Item\n    Link: [r, r][7]\n",
];

#[test]
fn a_destination_chosen_by_an_if_or_an_index_checks_its_candidates_as_the_old_engine_does() {
    assert_engines_agree(&symphonyscript_fixtures::vocabulary(), &CHOSEN_WIRES);
    assert_engines_agree(
        &symphonyscript_fixtures::bounded_vocabulary(),
        &CHOSEN_BOUNDED_WIRES,
    );
}

#[test]
fn a_chosen_destination_counts_its_candidates_beside_its_valid_twin() {
    let cases: [(&str, &[Code]); 6] = [
        (
            "state k(0)\nRoot r:\n    Item\n    Link: [r, r][k]\n",
            &[Code::CountBelowMinimum],
        ),
        ("Root r:\n    Item\n    Link: [r, r][1]\n", &[]),
        (
            "state sw(true)\nRoot r:\n    Item\n    Link: [r, r][sw]\n",
            &[Code::IndexType],
        ),
        (
            "state k(1.5)\nRoot r:\n    Item\n    Link: if k then r else r\n",
            &[Code::CountBelowMinimum, Code::ConditionType],
        ),
        (
            "state k(0)\nRoot r:\n    Item\n    Link: if k > 0 then r else r\n",
            &[Code::CountBelowMinimum],
        ),
        ("Root r:\n    Item\n    Link: if 1 > 0 then r else r\n", &[]),
    ];
    let vocabulary = symphonyscript_fixtures::bounded_vocabulary();
    let new = Compiler::new(&vocabulary).expect("the vocabulary compiles");
    for (text, codes) in cases {
        let found = new
            .checked_by_new_core(&[source(text)])
            .unwrap_or_else(|| panic!("the new core takes {text}"));
        let found: Vec<Code> = found.iter().map(|diagnostic| diagnostic.code).collect();
        assert_eq!(found, codes, "{text}");
    }
}

const CHOSEN_TO_PATHS: [&str; 14] = [
    "state look(0)\nClip a | gain=0.5:\n    Note/1\nClip b | gain=0.7:\n    Note/2\nClip c:\n    Note/3\n    Plays | gain=to.gain: [a, b][look]\n",
    "state look(0)\nClip a:\n    Note as h /1\nClip b:\n    Note/2\nClip c:\n    Note/3\n    Plays | gain=to.h.velocity: [a, b][look]\n",
    "state look(0)\nClip a:\n    Note as h /1 | velocity=0.5\nClip b:\n    Note as h /2\nClip c:\n    Note/3\n    Plays | gain=to.h.velocity: [a, b][look]\n",
    "Clip a:\n    Note as h /1\nClip b:\n    Note/2\nClip c:\n    Note/3\n    Plays | gain=to.h.velocity: if true then a else b\n",
    "Clip a:\n    Note as h /1\nClip b:\n    Note/2\nClip c:\n    Note/3\n    Plays | gain=to.h.velocity: if false then a else b\n",
    "state look(0)\nClip a:\n    Note/1\nClip b:\n    Note/2\nClip c:\n    Note/3\n    Plays | gain=to.nope: [a, b][look]\n",
    "state look(0)\nClip a:\n    Note/1\nClip b:\n    Note/2\nClip c:\n    Note/3\n    Plays | gain=to.gain * 2: [a, b][look]\n",
    "state look(0)\nClip a:\n    Note/1\nClip b:\n    Note/2\nClip c:\n    Note/3\n    Plays | gain=to.level: [a, b][look]\n",
    "state look(0)\nClip a:\n    Note/1\nFilter b:\n    Note/2\nClip c:\n    Note/3\n    Plays | gain=to.gain: [a, b][look]\n",
    "state look(0)\nClip a:\n    Note/1\nClip c:\n    Note/3\n    Plays | gain=to.gain: [a, a][look]\n",
    "state look(0)\nClip a:\n    Note as h /1\nClip c:\n    Note/3\n    Plays | gain=to.h: [a, a][look]\n",
    "state look(0)\nClip a | gain=0.5:\n    Note/1\nClip b | gain=0.7:\n    Note/2\nClip c:\n    Note/3\n    Plays/to.gain: [a, b][look]\n",
    "state look(0)\nClip a:\n    Note/1\nClip b:\n    Note/2\nClip c:\n    Note/3\n    Plays | gain=to.gain: [a, nope][look]\n",
    "state look(0)\nClip a | gain=0.5:\n    Note/1\nClip b | gain=0.7:\n    Note/2\nClip c:\n    Note/3\n    Plays | gain=to.gain + 300000000000: [a, b][look]\n",
];

#[test]
fn a_to_path_over_a_chosen_destination_reads_its_candidates_attributes_as_the_old_engine_does() {
    assert_engines_agree(&symphonyscript_fixtures::vocabulary(), &CHOSEN_TO_PATHS);
}

#[test]
fn a_to_path_over_several_candidates_draws_its_faults_beside_its_valid_twin() {
    let cases: [(&str, &[Code]); 4] = [
        (
            "state look(0)\nClip a:\n    Note as h /1\nClip b:\n    Note as h /2\nClip c:\n    Note/3\n    Plays | gain=to.h.velocity: [a, b][look]\n",
            &[],
        ),
        (
            "state look(0)\nClip a:\n    Note as h /1\nClip b:\n    Note/2\nClip c:\n    Note/3\n    Plays | gain=to.h.velocity: [a, b][look]\n",
            &[Code::ToCandidates],
        ),
        (
            "state look(0)\nClip a:\n    Note/1\nClip b:\n    Note/2\nClip c:\n    Note/3\n    Plays | gain=to.gain: [a, b][look]\n",
            &[],
        ),
        (
            "state look(0)\nClip a:\n    Note/1\nClip b:\n    Note/2\nClip c:\n    Note/3\n    Plays | transpose=to.gain: [a, b][look]\n",
            &[Code::TypeMismatch],
        ),
    ];
    let vocabulary = symphonyscript_fixtures::vocabulary();
    let new = Compiler::new(&vocabulary).expect("the vocabulary compiles");
    for (text, codes) in cases {
        let found = new
            .checked_by_new_core(&[source(text)])
            .unwrap_or_else(|| panic!("the new core takes {text}"));
        let found: Vec<Code> = found
            .iter()
            .map(|diagnostic| diagnostic.code)
            .filter(|code| *code != Code::NoEntryInstance)
            .collect();
        assert_eq!(found, codes, "{text}");
    }
}

#[test]
fn a_reference_read_where_a_value_is_expected_stays_with_the_old_engine() {
    let vocabulary = symphonyscript_fixtures::vocabulary();
    let new = Compiler::new(&vocabulary).expect("the vocabulary compiles");
    for text in [
        "state i(0)\nfn pick(k):\n    let t = [a, b][k]\n    k\nClip a:\n    Note/1\nClip b:\n    Note/2\nClip c:\n    Note/(pick(i))\n",
        "let d: i32 = a\nClip a:\n    Note/1\nClip c:\n    Note/3\n    Plays: d\n",
        "let d = a\nClip a | level=2:\n    Note/1\nClip c | level=d.level:\n    Note/3\n",
    ] {
        assert!(new.checked_by_new_core(&[source(text)]).is_none(), "{text}");
    }
}

const REFERENCE_WIRES: [&str; 26] = [
    "let d = a\nClip a:\n    Note/1\nClip c:\n    Note/3\n    Plays: d\n",
    "state look(0)\nlet xs = [a, b]\nClip a:\n    Note/1\nClip b:\n    Note/2\nClip c:\n    Note/3\n    Plays: xs[look]\n",
    "let xs = [a, b]\nClip a:\n    Note/1\nClip b:\n    Note/2\nClip c:\n    Note/3\n    Plays: xs[1]\n",
    "state look(0)\nClip a:\n    Note/1\nClip c:\n    Note/3\n    Plays: [a, 1][look]\n",
    "Clip a:\n    Note/1\nClip b:\n    Note/2\nClip c:\n    Note/3\n    Plays: [a, b]\n",
    "Clip a:\n    Note/1\nClip c:\n    Note/3\n    Plays: 1\n",
    "Clip a:\n    Note/1\nClip c:\n    Note/3\n    Plays: true\n",
    "let d = 1\nClip a:\n    Note/1\nClip c:\n    Note/3\n    Plays: d\n",
    "state look(0)\nClip a:\n    Note/1\nClip c:\n    Note/3\n    Plays: look\n",
    "let d = a\nClip a:\n    Note/1\n",
    "let d = if true then a else b\nClip a:\n    Note/1\nClip b:\n    Note/2\nClip c:\n    Note/3\n    Plays: d\n",
    "state f(false)\nlet d = if f then a else b\nClip a:\n    Note/1\nClip b:\n    Note/2\nClip c:\n    Note/3\n    Plays: d\n",
    "let xs = [a, b]\nClip a:\n    Note/1\nClip b:\n    Note/2\nClip c:\n    Note/3\n    Plays: xs\n",
    "let d = a\nlet e = d\nClip a:\n    Note/1\nClip c:\n    Note/3\n    Plays: e\n",
    "let d = a\nClip a:\n    Note/1\nClip c:\n    Note/3\n    Plays | gain=to.gain: d\n",
    "Clip a:\n    Note/1\nClip c:\n    Note/3\n    Plays: if true then a else 1\n",
    "let d = a.h\nClip a:\n    Note as h /1\n",
    "Clip a:\n    Note as h /1\nClip c:\n    Note/3\n    Plays: a.h\n",
    "let xs = [a, b]\nClip a:\n    Note/1\nClip b:\n    Note/2\nClip c:\n    Note/3\n    Plays: xs[5]\n",
    "state look(0.5)\nlet xs = [a, b]\nClip a:\n    Note/1\nClip b:\n    Note/2\nClip c:\n    Note/3\n    Plays: xs[look]\n",
    "state look(0)\nlet d = [a, b][look]\nClip a:\n    Note/1\nClip b:\n    Note/2\nClip c:\n    Note/3\n    Plays: d\n",
    "let d = e\nlet e = d\nClip a:\n    Note/1\nClip c:\n    Note/3\n    Plays: d\n",
    "state look(0)\nlet d = a\nClip a:\n    Note/1\nClip b:\n    Note/2\nClip c:\n    Note/3\n    Plays: if look > 0 then d else b\n",
    "let d = f\nClip a:\n    Note/1\n    Plays: d\nFilter f\n",
    "let xs = [a, b]\nlet ys = xs\nClip a:\n    Note/1\nClip b:\n    Note/2\nClip c:\n    Note/3\n    Plays: ys[0]\n",
    "Clip a:\n    Note/1\nClip c:\n    Note/3\n    Plays: [a][0][0]\n",
];

const REFERENCE_BOUNDED_WIRES: [&str; 2] = [
    "state k(0)\nlet xs = [r, r]\nRoot r:\n    Item\n    Link: xs[k]\n",
    "let xs = [r, r]\nRoot r:\n    Item\n    Link: xs[1]\n",
];

#[test]
fn a_destination_through_lets_paths_and_values_checks_as_the_old_engine_does() {
    assert_engines_agree(&symphonyscript_fixtures::vocabulary(), &REFERENCE_WIRES);
    assert_engines_agree(
        &symphonyscript_fixtures::bounded_vocabulary(),
        &REFERENCE_BOUNDED_WIRES,
    );
}

fn new_codes(vocabulary: &Vocabulary, text: &str) -> Vec<Code> {
    let new = Compiler::new(vocabulary).expect("the vocabulary compiles");
    new.checked_by_new_core(&[source(text)])
        .unwrap_or_else(|| panic!("the new core takes {text}"))
        .iter()
        .map(|diagnostic| diagnostic.code)
        .filter(|code| *code != Code::NoEntryInstance)
        .collect()
}

fn old_codes(vocabulary: &Vocabulary, text: &str) -> Vec<Code> {
    let old = Compiler::new(vocabulary)
        .expect("the vocabulary compiles")
        .on_the_old_engine();
    old.check(&[source(text)])
        .iter()
        .map(|diagnostic| diagnostic.code)
        .filter(|code| *code != Code::NoEntryInstance)
        .collect()
}

#[test]
fn a_destination_draws_destination_type_beside_its_valid_twin() {
    let vocabulary = symphonyscript_fixtures::vocabulary();
    let cases: [(&str, &[Code]); 6] = [
        (
            "let d = a\nClip a:\n    Note/1\nClip c:\n    Note/3\n    Plays: d\n",
            &[],
        ),
        (
            "let d = 1\nClip a:\n    Note/1\nClip c:\n    Note/3\n    Plays: d\n",
            &[Code::DestinationType],
        ),
        (
            "let xs = [a, b]\nClip a:\n    Note/1\nClip b:\n    Note/2\nClip c:\n    Note/3\n    Plays: xs[0]\n",
            &[],
        ),
        (
            "let xs = [a, b]\nClip a:\n    Note/1\nClip b:\n    Note/2\nClip c:\n    Note/3\n    Plays: xs\n",
            &[Code::DestinationType],
        ),
        (
            "Clip a:\n    Note as h /1\nClip c:\n    Note/3\n    Plays: a\n",
            &[],
        ),
        (
            "Clip a:\n    Note as h /1\nClip c:\n    Note/3\n    Plays: a.h\n",
            &[Code::WireEndpoint],
        ),
    ];
    for (text, codes) in cases {
        assert_eq!(new_codes(&vocabulary, text), codes, "{text}");
    }
}

#[test]
fn a_bool_index_and_a_runtime_to_read_keep_the_new_cores_sets_476() {
    let bounded = symphonyscript_fixtures::bounded_vocabulary();
    let index = "state sw(true)\nRoot r:\n    Item\n    Link: [r, r][sw]\n";
    assert_eq!(new_codes(&bounded, index), [Code::IndexType]);
    assert_eq!(
        old_codes(&bounded, index),
        [Code::CountBelowMinimum, Code::IndexType]
    );
    let vocabulary = symphonyscript_fixtures::vocabulary();
    for destination in ["[a, b][look]", "xs[look]"] {
        let read = format!(
            "state look(0)\nlet xs = [a, b]\nClip a | gain=0.5:\n    Note/1\nClip b | gain=0.7:\n    Note/2\nClip c:\n    Note/3\n    Plays | transpose=to.gain: {destination}\n"
        );
        assert_eq!(new_codes(&vocabulary, &read), [Code::TypeMismatch]);
        assert_eq!(old_codes(&vocabulary, &read), [Code::FloatToInteger]);
    }
}

#[test]
fn a_compile_time_choice_in_a_runtime_destination_counts_its_taken_arm_alone_476() {
    let vocabulary = symphonyscript_fixtures::vocabulary();
    let program = |arm: &str| {
        format!(
            "state look(0)\nClip a:\n    Note/1\nClip c:\n    Note/3\n    Plays: [if {arm} then a else f, c][look]\nFilter f\n"
        )
    };
    let taken = program("true");
    assert_eq!(new_codes(&vocabulary, &taken), []);
    assert_eq!(old_codes(&vocabulary, &taken), [Code::WireEndpoint]);
    for arm in ["false", "look > 0"] {
        let text = program(arm);
        assert_eq!(new_codes(&vocabulary, &text), [Code::WireEndpoint]);
        assert_eq!(old_codes(&vocabulary, &text), [Code::WireEndpoint]);
    }
}

fn differing_vocabulary() -> Vocabulary {
    let mut v = Vocabulary::builder("differing");
    let desk = v.node("Desk");
    let lamp = v.node("Lamp");
    let dial = v.node("Dial");
    let feeds = v.wire("Feeds");
    v.kind(desk).top_level(Count::Exactly(1)).entry();
    v.kind(lamp)
        .top_level(Count::Any)
        .attr::<i32>("level")
        .default(0);
    v.kind(dial)
        .top_level(Count::Any)
        .attr::<f32>("level")
        .default(0.0);
    v.wire_kind(feeds)
        .connects(desk, lamp)
        .connects(desk, dial)
        .attr::<f32>("gain")
        .default(1.0);
    v.build().expect("the differing vocabulary builds")
}

#[test]
fn a_to_path_over_candidates_whose_attribute_types_differ_draws_to_candidates() {
    let vocabulary = differing_vocabulary();
    let program = |second: &str| {
        format!("state k(0)\nDesk d:\n    Feeds | gain=to.level: [a, b][k]\nDial a\n{second} b\n")
    };
    let differing = program("Lamp");
    assert_eq!(new_codes(&vocabulary, &differing), [Code::ToCandidates]);
    assert_eq!(old_codes(&vocabulary, &differing), [Code::ToCandidates]);
    let alike = program("Dial");
    assert_eq!(new_codes(&vocabulary, &alike), []);
    assert_eq!(old_codes(&vocabulary, &alike), []);
}

#[test]
fn a_wire_draws_its_endpoint_count_and_cycle_faults_beside_its_valid_twin() {
    let cases: [(&str, &[Code]); 7] = [
        ("Clip a:\n    Plays: b\nClip b:\n    Note/1\n", &[]),
        (
            "Clip a:\n    Triggers: b\nClip b:\n    Note/1\n",
            &[Code::WireEndpoint],
        ),
        ("Root r:\n    Item\n    Link: r\n", &[]),
        ("Filter f:\n    Feeds: g\nFilter g\n", &[]),
        (
            "Filter f:\n    Feeds: g\nFilter g:\n    Feeds: f\n",
            &[Code::WireCycle],
        ),
        ("Root r:\n    Item\n", &[Code::CountBelowMinimum]),
        (
            "Root r:\n    Item\n    Link: r\n    Link: r\n",
            &[Code::CountExceeded],
        ),
    ];
    for (text, codes) in cases {
        let vocabulary = if text.starts_with("Root") {
            symphonyscript_fixtures::bounded_vocabulary()
        } else {
            symphonyscript_fixtures::vocabulary()
        };
        let new = Compiler::new(&vocabulary).expect("the vocabulary compiles");
        let found = new
            .checked_by_new_core(&[source(text)])
            .unwrap_or_else(|| panic!("the new core takes {text}"));
        let found: Vec<Code> = found
            .iter()
            .map(|diagnostic| diagnostic.code)
            .filter(|code| *code != Code::NoEntryInstance && *code != Code::UnreachableInstance)
            .collect();
        assert_eq!(found, codes, "{text}");
    }
}

#[test]
fn a_to_path_through_a_child_one_candidate_lacks_draws_to_candidates_alone_476() {
    let vocabulary = symphonyscript_fixtures::vocabulary();
    let program = |child: &str, end: &str| {
        format!(
            "state sec(0)\nClip b:\n    Note/1\nClip c:\n    Note{child}/2\nClip d:\n    Note/3\n    Plays | gain=to.seq.{end}: [b, c][sec]\n"
        )
    };
    for child in [" as seq", ""] {
        let text = program(child, "size");
        assert_eq!(new_codes(&vocabulary, &text), [Code::ToCandidates]);
        assert_eq!(
            old_codes(&vocabulary, &text),
            [Code::UnknownAttribute, Code::ToCandidates]
        );
    }
    let twin = program(" as seq", "pitch");
    assert_eq!(new_codes(&vocabulary, &twin), [Code::ToCandidates]);
    assert_eq!(old_codes(&vocabulary, &twin), new_codes(&vocabulary, &twin));
}
