use crate::compiler::Compiler;
use crate::diagnostics::Code;
use crate::syntax::front_end_goldens::source;

const PATHS: [&str; 21] = [
    "Clip c:\n    Note as gain\n    Note as gain\n",
    "Clip c | level=0.5:\n    Note/60\nlet n = c.level\nClip d | gain=n:\n    Note/1\n",
    "let n = c.level\nClip c | level=n:\n    Note/1\n",
    "Clip c | level=n:\n    Note/1\nlet n = c.level\n",
    "Clip c:\n    Note as h /60 | velocity=0.9\nClip d | gain=c.h.velocity:\n    Note/1\n",
    "Clip c:\n    Note/60\nClip d | gain=c.level:\n    Note/c.gain\n",
    "Clip c:\n    Note/60\nClip d | gain=c.nothing:\n    Note/1\n",
    "Clip c | level=d.gain:\n    Note/60\nClip d | gain=c.level:\n    Note/1\n",
    "Clip c | level=2:\n    Note as h /c.level\n",
    "Clip c | level=0.5:\n    Note/60\nClip d:\n    Note/c.level\n",
    "Clip c:\n    Note as h /60\n    Note as h /62\n",
    "Clip c:\n    Note/60\nClip d | gain=c.level.x:\n    Note/1\n",
    "Clip c:\n    Note as h /c.h.pitch\n",
    "Clip c:\n    Note as h /60\n    Note as k /c.h.pitch\n",
    "Clip c:\n    Note as h | velocity=c.level\nClip d | gain=c.h.velocity:\n    Note/1\n",
    "Clip c | level=c.h.velocity:\n    Note as h | velocity=c.level\n",
    "Clip c:\n    Note as h /60\nClip d:\n    Note/c.h.nope\n",
    "Clip c:\n    Note as h /60\nlet p = c.h.pitch + 1\nClip d:\n    Note/p\n",
    "Clip c | gain=1e39:\n    Note/60\nClip d | level=c.gain:\n    Note/1\n",
    "Session s:\n    Track as t:\n        Piano as t\n",
    "Session s:\n    Track as t:\n        Piano as p\n    Track as p\n",
];

#[test]
fn a_path_through_named_instances_and_their_children_reads_the_attribute_as_the_old_engine_does() {
    let vocabulary = symphonyscript_fixtures::vocabulary();
    let old = Compiler::new(&vocabulary)
        .expect("the vocabulary compiles")
        .on_the_old_engine();
    let new = Compiler::new(&vocabulary).expect("the vocabulary compiles");
    for text in PATHS {
        let sources = [source(text)];
        let found = new
            .checked_by_new_core(&sources)
            .unwrap_or_else(|| panic!("the new core takes {text}"));
        assert_eq!(found, old.check(&sources), "{text}");
    }
}

#[test]
fn a_path_draws_its_unknown_attribute_cycle_and_type_faults_beside_its_valid_twin() {
    let cases: [(&str, &[Code]); 11] = [
        (
            "Clip c | level=2:\n    Note as h /60\nClip d | gain=c.level:\n    Note/c.h.pitch\n",
            &[],
        ),
        (
            "Clip c | level=2:\n    Note/c.level\n",
            &[Code::TypeMismatch],
        ),
        (
            "Clip c:\n    Note as h /60\nClip d:\n    Note/c.h.pitch\n",
            &[],
        ),
        (
            "Clip c:\n    Note as h /60\nClip d:\n    Note/c.h.pith\n",
            &[Code::UnknownAttribute],
        ),
        ("let n = c.gain\nClip c | level=n:\n    Note/1\n", &[]),
        (
            "let n = c.level\nClip c | level=n:\n    Note/1\n",
            &[Code::DeclarationCycle],
        ),
        ("Clip c:\n    Note as h /60\n    Note as k /61\n", &[]),
        (
            "Clip c:\n    Note as h /60\n    Note as h /61\n",
            &[Code::ChildNameClash],
        ),
        ("Clip a | gain=b.gain:\n    Note\nClip b:\n    Note\n", &[]),
        (
            "Clip a | gain=b.sequence:\n    Note\nClip b:\n    Note\n",
            &[Code::UnknownAttribute],
        ),
        (
            "Clip a | seq=b.sequence:\n    Note\nClip b:\n    Note\n",
            &[Code::UnknownAttribute, Code::UnknownAttribute],
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
fn a_path_from_a_let_a_member_or_a_body_or_to_an_instance_stays_with_the_old_engine() {
    let vocabulary = symphonyscript_fixtures::vocabulary();
    let new = Compiler::new(&vocabulary).expect("the vocabulary compiles");
    for text in [
        "let xs = [1, 2]\nClip c:\n    Note/xs.size\n",
        "Clip c:\n    Note as h /60\nlet x = c.h\n",
        "fn f(): c.gain\nClip c:\n    Note/1\n",
        "state s(0.5)\nClip c | level=s:\n    Note/60\nlet n = c.level\n",
        "Clip c:\n    Note as h /60\nClip d:\n    Note/c.h.length\n",
        "Plant p:\n    Valve as v\n    Controller:\n        Drives: v\n",
    ] {
        assert!(new.checked_by_new_core(&[source(text)]).is_none(), "{text}");
    }
}
