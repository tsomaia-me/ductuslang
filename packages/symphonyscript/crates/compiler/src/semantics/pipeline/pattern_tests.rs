use symphonyscript_core::Vocabulary;

use crate::compiler::Compiler;
use crate::diagnostics::Code;
use crate::semantics::counted_walk::CountedWalk;
use crate::syntax::front_end_goldens::source;

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

const VALID_USES: [&str; 4] = [
    "pattern Arp(default root, steps = 4, gap = 4):\n    for i in 0..steps:\n        Note/(root + i * gap)\nClip verse:\n    Arp/48\n    Arp/60 | steps=8 gap=3\n",
    "pattern Arp(default root):\n    Note/root\n    Plays: verse\nClip verse:\n    Arp/48\n",
    "pattern Arp(default v):\n    Note/60 | velocity=v\nClip verse:\n    Arp/1\n",
    "pattern Arp(default v, n = 2):\n    for i in 0..n:\n        Note/(v + i)\nClip verse:\n    for k in 0..3:\n        Arp/k | n=k\n",
];

const FAULTY_USES: [&str; 11] = [
    "pattern Arp(default root, steps = 4):\n    for i in 0..steps:\n        Note/(root + i * 1.5)\nClip verse:\n    Arp/48\n",
    "pattern Arp(root):\n    Note/root\nClip verse:\n    Arp/48 | root=1\n",
    "pattern Arp(default root, steps):\n    Note/root\n    Note/steps\n    Note/(steps + 1)\nClip verse:\n    Arp/48 | bogus=1 root=2\n    Arp | steps=1 steps=2\n",
    "pattern Arp(default root):\n    Note/(root + true)\nClip verse:\n    Arp/48\n    Arp/50\n",
    "pattern Arp(default root):\n    Nope/root\n    Note/root | velo=1\nClip verse:\n    Arp/48\n    Arp/50\n",
    "pattern Arp(default v):\n    Note/([1, 2][v])\nClip verse:\n    Arp/1\n    Arp/5\n",
    "pattern Arp(default v):\n    Note/v\nClip verse:\n    Arp/300000000000\n",
    "pattern Arp(default v, n = 2):\n    for i in 0..n:\n        Note/(v + i)\n        Clip:\n            Note/i\nClip verse:\n    for k in 0..3:\n        Arp/k | n=k\n",
    "pattern Arp(default v):\n    Note/v\nClip verse:\n    Arp/(1 + true)\n",
    "pattern Arp(default a, default b):\n    Note/a\n",
    "pattern Arp(default root):\n    Note/(bogus)\n    Nope/1\n    Note/1 | velo=2\n",
];

const UNUSED: [&str; 4] = [
    "pattern Arp(default root, root):\n    Note/root\n",
    "pattern Arp(default root):\n    Note/(root + true)\n    for i in 0..root:\n        Note/i\n",
    "pattern Arp(default root):\n    Note/root\nlet Arp = 1\n",
    "pattern Clip(x):\n    Note/x\n",
];

#[test]
fn a_pattern_use_binds_its_parameters_as_a_call_and_its_faults_point_at_the_use_and_the_line() {
    let vocabulary = symphonyscript_fixtures::vocabulary();
    assert_engines_agree(&vocabulary, &VALID_USES);
    assert_engines_agree(&vocabulary, &FAULTY_USES);
    assert_engines_agree(&vocabulary, &UNUSED);
    let new = Compiler::new(&vocabulary).expect("the vocabulary compiles");
    let found = new
        .checked_by_new_core(&[source(FAULTY_USES[3])])
        .expect("the new core takes two uses of one faulty body");
    let chains: Vec<Vec<std::ops::Range<u32>>> = found
        .iter()
        .filter(|diagnostic| diagnostic.code == Code::OperandType)
        .map(|diagnostic| {
            diagnostic
                .related
                .iter()
                .map(|related| related.span.range.clone())
                .collect()
        })
        .collect();
    assert_eq!(chains, vec![vec![66..72], vec![77..83]]);
}

fn uses(count: usize, fixed: bool) -> String {
    let mut text = String::from(
        "pattern Arp(default root, steps = 4, gap = 4):\n    for i in 0..steps:\n        Note/(root + i * gap)\nClip c:\n",
    );
    for at in 0..count {
        text.push_str(&format!("    Arp/{} | steps=2\n", 36 + at % 24));
    }
    if fixed {
        text.push_str("    Arp/(i32(40))\n");
    }
    text
}

#[test]
fn uses_with_one_key_type_the_pattern_body_once_and_a_fixed_argument_types_a_second() {
    let vocabulary = symphonyscript_fixtures::vocabulary();
    assert_engines_agree(&vocabulary, &[&uses(3, false), &uses(3, true)]);
    let new = Compiler::new(&vocabulary).expect("the vocabulary compiles");
    for (fixed, typed) in [(false, 1), (true, 2)] {
        let text = uses(2_000, fixed);
        let bodies = CountedWalk::CorePatternBody.steps();
        let used = CountedWalk::CorePatternUse.steps();
        new.checked_by_new_core(&[source(&text)])
            .unwrap_or_else(|| panic!("the new core takes 2,000 uses, fixed: {fixed}"));
        assert_eq!(
            CountedWalk::CorePatternBody.steps() - bodies,
            typed,
            "fixed: {fixed}"
        );
        assert_eq!(
            CountedWalk::CorePatternUse.steps() - used,
            2_000 + u64::from(fixed),
            "fixed: {fixed}"
        );
    }
}

#[test]
fn a_use_in_a_pattern_body_a_named_use_and_a_top_level_use_stay_with_the_old_engine() {
    let vocabulary = symphonyscript_fixtures::vocabulary();
    let new = Compiler::new(&vocabulary).expect("the vocabulary compiles");
    for text in [
        "pattern P0():\n    Note/1\npattern P1():\n    P0\nClip c:\n    P1\n",
        "pattern P(default v):\n    Note/v\nClip c:\n    P as p /1\n",
        "pattern P(default v):\n    Note/v\nP top /1\n",
        "pattern P(default v):\n    Note as n /v\nClip c:\n    P/1\n",
        "state s(1)\npattern P(default v):\n    Note/v\nClip c:\n    P/s\n",
        "pattern Arp(default root):\n    Note/root\nClip verse:\n    Arp/(1.5)\n",
    ] {
        assert!(new.checked_by_new_core(&[source(text)]).is_none(), "{text}");
    }
}
