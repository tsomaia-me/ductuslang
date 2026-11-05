use crate::compiler::Compiler;
use crate::diagnostics::Code;
use crate::syntax::front_end_goldens::source;

const TO_PATHS: [&str; 21] = [
    "fn f(): to.gain\nClip a:\n    Plays | gain=f(): b\nClip b:\n    Note/1\n",
    "Clip verse:\n    Plays | gain=to.hook.velocity: chorus\nClip chorus:\n    Note as hook /72 | velocity=0.9\n",
    "Clip a:\n    Plays | gain=to.gain: b\nClip b | gain=0.25:\n    Note/1\n",
    "Clip a:\n    Plays | gain=to.level: b\nClip b:\n    Note/1\n",
    "Clip a:\n    Plays/to.gain: b\nClip b | gain=2:\n    Note/1\n",
    "Clip a:\n    Plays | gain=to.nope: b\nClip b:\n    Note/1\n",
    "Clip a:\n    Plays | gain=to.h.velocity: b\nClip b:\n    Note/1\n",
    "Clip a:\n    Plays | gain=to.h: b\nClip b:\n    Note as h /60\n",
    "Clip a:\n    Plays | gain=to: b\nClip b:\n    Note/1\n",
    "let x = to.gain\nClip b:\n    Note/x\n",
    "Clip b:\n    Note/to.gain\n",
    "Clip a:\n    Plays | gain=to.gain: nowhere\n",
    "Clip a:\n    Plays | transpose=to.gain: b\nClip b | gain=0.5:\n    Note/1\n",
    "Clip a:\n    Plays | gain=to.gain.x: b\nClip b:\n    Note/1\n",
    "Clip a | gain=b.gain:\n    Plays | gain=to.gain: b\nClip b | gain=1:\n    Note/1\n",
    "let g = 0.5\nClip a:\n    Plays | gain=to.gain + g: b\nClip b | gain=g:\n    Note/1\n",
    "Clip a:\n    Plays | gain=to.h.velocity * 2: b\nClip b:\n    Note as h | velocity=0.25\n",
    "Clip a | gain=0.5:\n    Plays | gain=to.gain: a\n",
    "fn f(): to\nClip a:\n    Plays | gain=f(): b\nClip b:\n    Note/1\n",
    "Clip a:\n    Plays | transpose=to.h.pitch: b\nClip b:\n    Note as h /72\n",
    "Clip a:\n    Plays | gain=to.h.velocity: b\nClip b:\n    Note as h /60 | velocity=to.gain\n",
];

#[test]
fn a_to_path_reads_the_attribute_of_its_wires_destination_as_the_old_engine_does() {
    let vocabulary = symphonyscript_fixtures::vocabulary();
    let old = Compiler::new(&vocabulary)
        .expect("the vocabulary compiles")
        .on_the_old_engine();
    let new = Compiler::new(&vocabulary).expect("the vocabulary compiles");
    for text in TO_PATHS {
        let sources = [source(text)];
        let found = new
            .checked_by_new_core(&sources)
            .unwrap_or_else(|| panic!("the new core takes {text}"));
        assert_eq!(found, old.check(&sources), "{text}");
    }
}

#[test]
fn a_to_path_draws_its_unknown_attribute_and_misplaced_to_faults_beside_its_valid_twin() {
    let cases: [(&str, &[Code]); 10] = [
        (
            "Clip a:\n    Plays | gain=to.gain: b\nClip b | gain=0.25:\n    Note/1\n",
            &[],
        ),
        (
            "Clip a:\n    Plays | gain=to.gian: b\nClip b | gain=0.25:\n    Note/1\n",
            &[Code::UnknownAttribute],
        ),
        (
            "Clip a:\n    Plays | gain=to.h.velocity: b\nClip b:\n    Note as h /60\n",
            &[],
        ),
        (
            "Clip a:\n    Plays | gain=to.k.velocity: b\nClip b:\n    Note as h /60\n",
            &[Code::UnknownAttribute],
        ),
        (
            "Clip a:\n    Plays | gain=to.h: b\nClip b:\n    Note as h /60\n",
            &[Code::MisplacedTo],
        ),
        (
            "Clip a:\n    Plays | gain=to.gain: b\nClip b:\n    Note/1\n",
            &[],
        ),
        (
            "Clip a:\n    Plays | gain=to: b\nClip b:\n    Note/1\n",
            &[Code::MisplacedTo],
        ),
        (
            "Clip a:\n    Plays | gain=to.gain: b\nClip b:\n    Note/b.gain\n",
            &[Code::TypeMismatch],
        ),
        (
            "Clip a:\n    Plays | gain=to.gain: b\nClip b:\n    Note/to.gain\n",
            &[Code::MisplacedTo],
        ),
        (
            "Clip a:\n    Plays | transpose=to.gain: b\nClip b | gain=0.5:\n    Note/1\n",
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
            .filter(|code| *code != Code::NoEntryInstance && *code != Code::UnreachableInstance)
            .collect();
        assert_eq!(found, codes, "{text}");
    }
}

#[test]
fn a_to_path_in_a_gate_or_a_named_wire_stays_with_the_old_engine() {
    let vocabulary = symphonyscript_fixtures::vocabulary();
    let new = Compiler::new(&vocabulary).expect("the vocabulary compiles");
    for text in [
        "Clip a:\n    Plays when to.gain > 0.5: b\nClip b:\n    Note/1\n",
        "Clip a:\n    Plays as w | gain=to.gain: b\nClip b:\n    Note/1\n",
    ] {
        assert!(new.checked_by_new_core(&[source(text)]).is_none(), "{text}");
    }
}

const PAIR: &str = "pattern Pair(default v):\n    Note as a /v\n    Note as b /v | velocity=0.5\n";
const HOOK: &str = "pattern Hook(default v):\n    Note as hook /v\n    Note/v\n";
const WRAP: &str = "pattern Wrap(default v):\n    Pair as p /v\n    Note as c /v\n";

fn through_uses(head: &str, body: &str) -> String {
    format!("{head}{body}")
}

#[test]
fn a_to_path_steps_through_the_names_inside_a_named_pattern_use_as_the_old_engine_does_486() {
    let pair_wrap = format!("{PAIR}{WRAP}");
    let cases: [(String, &[Code]); 14] = [
        (
            through_uses(
                PAIR,
                "Clip chorus:\n    Pair as p /60\nClip verse:\n    Plays | gain=to.p.a.velocity: chorus\n",
            ),
            &[],
        ),
        (
            through_uses(
                PAIR,
                "Clip chorus:\n    Pair as p /60\nClip verse:\n    Plays | gain=to.p.b.velocity: chorus\n",
            ),
            &[],
        ),
        (
            through_uses(
                PAIR,
                "Clip chorus:\n    Pair as p /60\nClip verse:\n    Plays | gain=to.p.a: chorus\n",
            ),
            &[Code::MisplacedTo],
        ),
        (
            through_uses(
                PAIR,
                "Clip chorus:\n    Pair as p /60\nClip verse:\n    Plays | gain=to.p: chorus\n",
            ),
            &[Code::MisplacedTo],
        ),
        (
            through_uses(
                PAIR,
                "Clip chorus:\n    Pair as p /60\nClip verse:\n    Plays | gain=to.p.c.velocity: chorus\n",
            ),
            &[Code::UnknownAttribute],
        ),
        (
            through_uses(
                PAIR,
                "Clip chorus:\n    Pair as p /60\nClip verse:\n    Plays | gain=to.p.velocity: chorus\n",
            ),
            &[Code::UnknownAttribute],
        ),
        (
            through_uses(
                PAIR,
                "Clip chorus:\n    Pair as p /60\nClip verse:\n    Plays | gain=to.p.b.nope: chorus\n",
            ),
            &[Code::UnknownAttribute],
        ),
        (
            through_uses(
                PAIR,
                "Clip chorus:\n    Pair as p /60\nClip verse:\n    Plays | gain=to.a.velocity: chorus\n",
            ),
            &[Code::UnknownAttribute],
        ),
        (
            through_uses(
                HOOK,
                "Clip lk:\n    Hook as lk /60\nClip verse:\n    Plays | gain=to.lk.hook.velocity: lk\n",
            ),
            &[],
        ),
        (
            through_uses(
                HOOK,
                "Clip lk:\n    Hook as lk /60\nClip verse:\n    Plays | gain=to.lk.hook: lk\n",
            ),
            &[Code::MisplacedTo],
        ),
        (
            through_uses(
                &pair_wrap,
                "Clip chorus:\n    Wrap as w /60\nClip verse:\n    Plays | gain=to.w.p.a.velocity: chorus\n    Plays | gain=to.w.c.velocity: chorus\n",
            ),
            &[],
        ),
        (
            through_uses(
                &pair_wrap,
                "Clip chorus:\n    Wrap as w /60\nClip verse:\n    Plays | gain=to.w.p: chorus\n",
            ),
            &[Code::MisplacedTo],
        ),
        (
            through_uses(
                &pair_wrap,
                "Clip chorus:\n    Wrap as w /60\nClip verse:\n    Plays | gain=to.w.p.zz: chorus\n",
            ),
            &[Code::UnknownAttribute],
        ),
        (
            through_uses(
                PAIR,
                "state sec(0)\nClip chorus:\n    Pair as p /60\nClip other:\n    Note as z /1\nClip verse:\n    Plays | gain=to.p.b.velocity: [chorus, other][sec]\n",
            ),
            &[Code::ToCandidates],
        ),
    ];
    let vocabulary = symphonyscript_fixtures::vocabulary();
    let old = Compiler::new(&vocabulary)
        .expect("the vocabulary compiles")
        .on_the_old_engine();
    let new = Compiler::new(&vocabulary).expect("the vocabulary compiles");
    for (text, codes) in cases {
        let sources = [source(&text)];
        let found = new
            .checked_by_new_core(&sources)
            .unwrap_or_else(|| panic!("the new core takes {text}"));
        assert_eq!(found, old.check(&sources), "{text}");
        let found: Vec<Code> = found
            .iter()
            .map(|diagnostic| diagnostic.code)
            .filter(|code| *code != Code::NoEntryInstance && *code != Code::UnreachableInstance)
            .collect();
        assert_eq!(found, codes, "{text}");
    }
}
