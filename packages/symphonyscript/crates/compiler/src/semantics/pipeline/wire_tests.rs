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
