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
fn a_path_from_a_let_or_a_member_to_an_instance_or_through_a_function_cycle_stays_with_the_old_engine()
 {
    let vocabulary = symphonyscript_fixtures::vocabulary();
    let new = Compiler::new(&vocabulary).expect("the vocabulary compiles");
    for text in [
        "let xs = [1, 2]\nClip c:\n    Note/xs.size\n",
        "let n = f()\nfn f(): c.gain\nClip c | gain=n:\n    Note/n\n",
        "let n = f()\nfn f(): m\nlet m = c.gain\nClip c | gain=n:\n    Note/n\n",
        "Clip c:\n    Note as h /60\nClip d:\n    Note/c.h.length\n",
    ] {
        assert!(new.checked_by_new_core(&[source(text)]).is_none(), "{text}");
    }
}

const BODY_PATHS: [&str; 9] = [
    "fn f(): c.gain\nClip c:\n    Note/1\n",
    "Clip c | gain=0.5:\n    Note/60\nfn f(x): x + c.gain\nClip k:\n    Note/(f(1))\n",
    "Clip c | gain=0.5:\n    Note/60\nfn f(x): x * c.gain\nClip k | gain=f(2.0):\n    Note/1\n",
    "Clip c:\n    Note as h /60\nfn f(): c.h.pitch + 1\nClip k:\n    Note/(f())\n    Note/(f())\n",
    "Clip c:\n    Note/60\nfn f(x): x + c.nope\nClip k:\n    Note/(f(1))\n    Note/(f(2))\n",
    "Clip c:\n    Note/60\nfn f(x): x + c.nope\nClip k:\n    Note/1\n",
    "Clip c:\n    Note/60\nfn g(x): x + c.gain.zz\nfn f(x): g(x) + 1\nClip k:\n    Note/(f(1))\n",
    "Clip c | gain=0.5:\n    Note/60\nlet a = for v in [1, 2] into s from 0.0: s + c.gain\nClip k | gain=a:\n    Note/1\n",
    "Clip c:\n    Note/60\nfn f(x): for v in [1, 2] into s from x: s + c.nope.q\nClip k:\n    Note/(f(1))\n    Note/(f(2))\n",
];

#[test]
fn a_path_in_a_function_or_fold_body_reads_the_attribute_as_the_old_engine_does() {
    let vocabulary = symphonyscript_fixtures::vocabulary();
    let old = Compiler::new(&vocabulary)
        .expect("the vocabulary compiles")
        .on_the_old_engine();
    let new = Compiler::new(&vocabulary).expect("the vocabulary compiles");
    for text in BODY_PATHS {
        let sources = [source(text)];
        let found = new
            .checked_by_new_core(&sources)
            .unwrap_or_else(|| panic!("the new core takes {text}"));
        assert_eq!(found, old.check(&sources), "{text}");
    }
}

#[test]
fn a_path_fault_in_a_function_body_draws_at_each_call_beside_its_valid_twin() {
    let cases: [(&str, usize, usize); 4] = [
        (
            "Clip c:\n    Note/60\nfn f(x): x + c.level\nClip k | gain=f(0.5) + f(0.25):\n    Note/1\n",
            0,
            0,
        ),
        (
            "Clip c:\n    Note/60\nfn f(x): x + c.lvel\nClip k | gain=f(0.5) + f(0.25):\n    Note/1\n",
            2,
            1,
        ),
        (
            "Clip c:\n    Note/60\nfn f(x): x + c.level.x\nfn g(x): f(x)\nClip k | gain=g(0.5):\n    Note/1\n",
            1,
            2,
        ),
        (
            "Clip c:\n    Note/60\nfn f(x): x + c.lvel\nClip k:\n    Note/1\n",
            1,
            0,
        ),
    ];
    let vocabulary = symphonyscript_fixtures::vocabulary();
    let new = Compiler::new(&vocabulary).expect("the vocabulary compiles");
    for (text, drawn, chain) in cases {
        let found = new
            .checked_by_new_core(&[source(text)])
            .unwrap_or_else(|| panic!("the new core takes {text}"));
        let faults: Vec<_> = found
            .iter()
            .filter(|diagnostic| diagnostic.code == Code::UnknownAttribute)
            .collect();
        assert_eq!(faults.len(), drawn, "{text}");
        assert!(
            faults
                .iter()
                .all(|diagnostic| diagnostic.related.len() == chain
                    && diagnostic
                        .related
                        .iter()
                        .all(|related| related.label == "in this call")),
            "{text}"
        );
        let others = found
            .iter()
            .filter(|diagnostic| {
                !matches!(
                    diagnostic.code,
                    Code::UnknownAttribute | Code::NoEntryInstance | Code::UnreachableInstance
                )
            })
            .count();
        assert_eq!(others, 0, "{text}");
    }
}

const CHILD_NAMES: [&str; 8] = [
    "Plant p:\n    Valve as v\n    Controller:\n        Drives: v\n",
    "Clip c:\n    Note as h /60\n    Note/h.pitch\n",
    "Clip c:\n    Note as h /60\nlet x = h\n",
    "let h = 3\nClip c:\n    Note as h /60\n    Note/(h.pitch + 1)\n",
    "Plant p:\n    Valve as v\n    Controller as k:\n        Drives: v\n    Controller:\n        Drives: k\n",
    "Clip c:\n    Note as h /60 | velocity=0.5\n    Note/62 | velocity=h.velocity\n",
    "Plant p:\n    Valve as v\n    Controller:\n        Drives: w\n",
    "Clip c:\n    Note as h /60\n    Note/h.nope\n",
];

#[test]
fn a_name_in_a_body_reads_a_child_of_its_body_or_an_enclosing_one_as_the_old_engine_does() {
    let vocabulary = symphonyscript_fixtures::vocabulary();
    let old = Compiler::new(&vocabulary)
        .expect("the vocabulary compiles")
        .on_the_old_engine();
    let new = Compiler::new(&vocabulary).expect("the vocabulary compiles");
    for text in CHILD_NAMES {
        let sources = [source(text)];
        let found = new
            .checked_by_new_core(&sources)
            .unwrap_or_else(|| panic!("the new core takes {text}"));
        assert_eq!(found, old.check(&sources), "{text}");
    }
}

const REACTIVE_PATHS: [&str; 17] = [
    "state s0(0.5)\nClip c | gain=s0:\n    Note/(c.gain)\n",
    "state s0(0.5)\nClip c | gain=s0:\n    Note/60\nlet a = c.gain + 1.0\nClip d | gain=a:\n    Note/1\n",
    "state s0(1)\nClip c | gain=s0:\n    Note/(c.gain)\n",
    "state s0(1)\nClip c | level=s0:\n    Note/60\nClip d | level=c.level:\n    Note/1\n",
    "state s0(1)\nClip c | gain=s0 + 1:\n    Note/60\nClip d | gain=c.gain * 2:\n    Note/1\n",
    "state s0(0.5)\nClip c | gain=s0:\n    Note as h | velocity=s0\nClip d:\n    Note/(c.h.velocity)\n",
    "state s0(1): u8\nClip c | level=s0:\n    Note/(c.level + 300)\n",
    "state s0(0.5)\nClip c | gain=s0:\n    Note/60\nfn f(): c.gain\nClip d:\n    Note/(f())\n",
    "state s0(1)\nClip c | level=s0:\n    Note/60\nClip d | level=c.level:\n    Note/1\nClip e | level=d.level + 1:\n    Note/1\n",
    "state s0(1)\nClip c | level=s0:\n    Note/60\nstate q = c.level + 1\nClip d | level=q:\n    Note/1\n",
    "state s0(1)\nClip c | gain=s0 + 0.5:\n    Note/(c.gain)\n",
    "state s0(3000000000)\nClip c:\n    Note as h /s0\nClip d:\n    Note/(c.h.pitch)\n",
    "state s0(3000000000)\nClip c:\n    Note as h /s0\nClip d | gain=c.h.pitch:\n    Note/1\n",
    "state s0(0.5)\nClip c | gain=s0:\n    Note/(c.gain + 1)\n",
    "state s0(1)\nClip c | gain=s0:\n    Note/60\nlet a = [c.gain]\n",
    "state s0(1)\nClip c | gain=s0:\n    Note/(c.gain + true)\n",
    "state s0(0.5)\nClip c | gain=s0:\n    Note/([1, 2][c.gain])\n",
];

#[test]
fn a_path_whose_attribute_reaches_a_cell_reads_the_cell_as_the_old_engine_does() {
    let vocabulary = symphonyscript_fixtures::vocabulary();
    let old = Compiler::new(&vocabulary)
        .expect("the vocabulary compiles")
        .on_the_old_engine();
    let new = Compiler::new(&vocabulary).expect("the vocabulary compiles");
    for text in REACTIVE_PATHS {
        let sources = [source(text)];
        let found = new
            .checked_by_new_core(&sources)
            .unwrap_or_else(|| panic!("the new core takes {text}"));
        assert_eq!(found, old.check(&sources), "{text}");
    }
}

#[test]
fn a_path_reaching_a_cell_draws_its_cell_faults_beside_its_valid_twin() {
    let cases: [(&str, &[Code]); 11] = [
        (
            "state s0(0.5)\nClip c | gain=s0:\n    Note/(c.gain)\n",
            &[Code::FloatToInteger],
        ),
        (
            "state s0(0.5)\nClip c | gain=s0:\n    Note/(i32(c.gain))\n",
            &[],
        ),
        (
            "state s0(1)\nlet w = s0\nClip c | gain=w:\n    Note/60\nClip d:\n    Note/(c.gain)\n",
            &[Code::ReactiveLet, Code::TypeMismatch],
        ),
        (
            "let w = 1\nClip c | gain=w:\n    Note/60\nClip d:\n    Note/(c.gain)\n",
            &[Code::TypeMismatch],
        ),
        (
            "state s0(1)\nlet w = s0\nClip c | gain=w:\n    Note/60\nClip d:\n    Note/(i32(c.gain))\n",
            &[Code::ReactiveLet],
        ),
        (
            "state s0(3000000000)\nClip c:\n    Note as h /s0\nClip d:\n    Note/(c.h.pitch)\n",
            &[Code::DoesNotFit, Code::DoesNotFit],
        ),
        (
            "state s0(3)\nClip c:\n    Note as h /s0\nClip d:\n    Note/(c.h.pitch)\n",
            &[],
        ),
        (
            "state s0(1)\nClip c | level=s0:\n    Note/60\nlet x = if s0 > 0 then 1 else 2\nClip d | level=x:\n    Note/1\n",
            &[Code::ReactiveLet],
        ),
        (
            "state s0(1)\nClip c | level=s0:\n    Note/60\nlet x = if c.level > 0 then 1 else 2\nClip d | level=x:\n    Note/1\n",
            &[Code::ReactiveLet],
        ),
        (
            "state s0(1)\nClip c | level=s0:\n    Note/60\nlet x = 1\nClip d | level=x:\n    Note/1\n",
            &[Code::IntegerCell],
        ),
        (
            "state s0(1)\nClip c | level=s0:\n    Note/60\nstate x = if s0 > 0 then 1 else 2\nClip d | level=x:\n    Note/1\n",
            &[],
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
