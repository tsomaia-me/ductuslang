use symphonyscript_core::Vocabulary;

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

const CHOSEN_BOUNDED_WIRES: [&str; 8] = [
    "state sw(true)\nRoot r:\n    Item\n    Link: if sw then r else r\n",
    "Root r:\n    Item\n    Link: if true then r else r\n",
    "state sw(true)\nRoot r:\n    Item\n    Link: if sw then r else z\n",
    "state k(0)\nRoot r:\n    Item\n    Link: [r, r][k]\n    Link: r\n",
    "Root r:\n    Item\n    Link: if 1 then r else r\n",
    "Root r:\n    Item\n    Link: [r, r][2.5]\n",
    "Root r:\n    Item\n    Link: [r, r][7]\n",
    "state k(1.5)\nRoot r:\n    Item\n    Link: if k then r else r\n",
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
            &[Code::ConditionType],
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

#[test]
fn a_chosen_destination_beside_a_to_path_or_holding_a_value_stays_with_the_old_engine() {
    let vocabulary = symphonyscript_fixtures::vocabulary();
    let new = Compiler::new(&vocabulary).expect("the vocabulary compiles");
    for text in [
        "state look(0)\nClip a | gain=0.5:\n    Note/1\nClip b | gain=0.7:\n    Note/2\nClip c:\n    Note/3\n    Plays | gain=to.gain: [a, b][look]\n",
        "state look(0)\nClip a:\n    Note/1\nClip c:\n    Note/3\n    Plays: [a, 1][look]\n",
        "let d = a\nClip a:\n    Note/1\nClip c:\n    Note/3\n    Plays: d\n",
    ] {
        assert!(new.checked_by_new_core(&[source(text)]).is_none(), "{text}");
    }
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
