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
fn the_declined_pattern_forms_stay_with_the_old_engine() {
    let vocabulary = symphonyscript_fixtures::vocabulary();
    let new = Compiler::new(&vocabulary).expect("the vocabulary compiles");
    for text in [
        "pattern P(default v):\n    Note as n /v\n    Note/(n.pitch)\n",
        "state s(0)\npattern P(default v, w = v + 1):\n    Note/w\nClip c:\n    P/s\n",
        "pattern P(default v):\n    for i in 0..v:\n        Note/i\nClip c:\n    Note as n /2\n    P/(n.pitch)\n",
        "pattern Voice(default f):\n    Osc as osc /f\n    Filter\nSynth s:\n    Voice as v1 /440\n",
        "pattern P(default v):\n    Plays: v\nClip c:\n    P/d\nClip d\n",
    ] {
        assert!(new.checked_by_new_core(&[source(text)]).is_none(), "{text}");
    }
}

fn assert_files_agree(vocabulary: &Vocabulary, programs: &[&[&str]]) {
    let old = Compiler::new(vocabulary)
        .expect("the vocabulary compiles")
        .on_the_old_engine();
    let new = Compiler::new(vocabulary).expect("the vocabulary compiles");
    for files in programs {
        let sources: Vec<crate::Source> = files
            .iter()
            .enumerate()
            .map(|(at, text)| crate::Source {
                path: format!("f{at}.sym"),
                text: (*text).into(),
            })
            .collect();
        let found = new
            .checked_by_new_core(&sources)
            .unwrap_or_else(|| panic!("the new core takes {files:?}"));
        assert_eq!(found, old.check(&sources), "{files:?}");
    }
}

const TOP_LEVEL_USES: [&str; 12] = [
    "pattern P(default v):\n    Clip:\n        Note/v\nP top /1\n",
    "pattern P(default v):\n    Clip:\n        Note/v\nSession s\nP top /1\n",
    "pattern P(default v):\n    Session:\n        Track\nP top /1\n",
    "pattern P(default v):\n    Session\nP top /1\nP two /2\n",
    "pattern Q(default v):\n    Clip:\n        Note/v\npattern P(default v):\n    Q/v\nP top /1\n",
    "pattern P(default v):\n    Clip:\n        Note/(v + true)\nP top /1\nP other /2\n",
    "pattern P(default v):\n    Clip:\n        Note/v\nP top /1\nP top /2\n",
    "pattern P(default v):\n    Clip:\n        Note/v\nP top /(1 + true)\nP t2 | bogus=1\n",
    "pattern P(default v):\n    Clip:\n        Note/v\npattern Q(default v):\n    P/v\nQ top /300000000000\n",
    "pattern P(default v):\n    Clip\nP top /1\nClip c:\n    Plays: top\n",
    "pattern P():\n    Note\nP top\n",
    "pattern P():\n    Clip\nP top\nP top\n",
];

const TOP_LEVEL_GROUPS: [&str; 4] = [
    "pattern P(default v):\n    Note/v\n    Note/v\nP top /1\n",
    "pattern P(default v):\n    Nope/v\nP top /1\n",
    "pattern P(default v):\n    for i in 0..v:\n        Clip\nP top /1\n",
    "pattern P(default v):\n    Clip\n    Clip\nP top /(1 + true)\n",
];

#[test]
fn a_top_level_use_of_a_stand_in_places_its_node_under_the_uses_name_and_a_group_draws_top_level_pattern()
 {
    let vocabulary = symphonyscript_fixtures::vocabulary();
    assert_engines_agree(&vocabulary, &TOP_LEVEL_USES);
    assert_engines_agree(&vocabulary, &TOP_LEVEL_GROUPS);
    let new = Compiler::new(&vocabulary).expect("the vocabulary compiles");
    let found = new
        .checked_by_new_core(&[source(TOP_LEVEL_USES[3])])
        .expect("the new core takes two top-level Session stand-ins");
    let exceeded: Vec<_> = found
        .iter()
        .filter(|diagnostic| diagnostic.code == Code::CountExceeded)
        .map(|diagnostic| (diagnostic.span.range.clone(), diagnostic.related.len()))
        .collect();
    assert_eq!(exceeded, vec![(26..33, 1)]);
    let groups = new
        .checked_by_new_core(&[source(TOP_LEVEL_GROUPS[0])])
        .expect("the new core takes a top-level group");
    assert!(
        groups
            .iter()
            .any(|diagnostic| diagnostic.code == Code::TopLevelPattern)
    );
}

const NESTED_USES: [&str; 8] = [
    "pattern Q(default v):\n    Note/v\n    Note/(v + 1)\npattern P(default v):\n    Q/v\n    Q/(v * 2)\nClip c:\n    P/1\n    P/(1 + true)\n",
    "pattern Q(default v):\n    Note/(v + true)\npattern P(default v):\n    Q/v\n    Q/(v * 2)\nClip c:\n    P/1\n    P/2\n",
    "pattern Q():\n    Clip\npattern P():\n    Q\n    Q\nClip c:\n    P\n",
    "pattern Q():\n    Clip\npattern P():\n    Q\n    Q\nClip c:\n    P\n    P\n",
    "pattern Q(default v):\n    Note/([1, 2][v])\npattern P(default v):\n    Q/v\n    Q/(v - 2)\nClip c:\n    P/7\n",
    "pattern P0():\n    Note/300000000000\npattern P1():\n    P0\n    P0\npattern P2():\n    P1\n    P1\nClip c:\n    P2\n    P2\n",
    "pattern Q(default v, n = 2):\n    for i in 0..n:\n        Note/(v + i)\npattern P(default v):\n    for k in 0..2:\n        Q/(v + k) | n=k\nClip c:\n    P/40\n",
    "pattern Q(default v):\n    Note/v\npattern P(default v):\n    Q/v\nClip c:\n    P/1\n    for i in 0..0:\n        P/2\n",
];

const RECURSIVE_PATTERNS: [&str; 5] = [
    "pattern Q():\n    P\npattern P():\n    Q\nClip c:\n    P\n",
    "pattern P():\n    P\nClip c:\n    P\n",
    "pattern P():\n    Note/(bogus)\n    Note/(1 + true)\n    P\nClip c:\n    P\n",
    "pattern P():\n    Note\n    P\nClip c:\n    P\n",
    "pattern Q():\n    P\npattern P():\n    Q\nP top\n",
];

const UNRUN_USES: [&str; 4] = [
    "pattern P(default v):\n    Note/(v + true)\nClip c:\n    for i in 0..0:\n        P/i\n",
    "pattern P(default v):\n    Note/v\nClip c:\n    for i in 0..0:\n        P/(i + bogus)\n",
    "pattern P(default v):\n    Note/v\nClip c:\n    for i in 0..0:\n        P/i | bogus=1 bogus=2\n        P | v=1\n",
    "pattern P(default v):\n    Note/v\nClip c:\n    Nope:\n        P/2 | x=1\n",
];

#[test]
fn nested_uses_draw_one_diagnostic_per_outermost_use_and_a_pattern_that_reaches_itself_draws_recursive_pattern()
 {
    let vocabulary = symphonyscript_fixtures::vocabulary();
    assert_engines_agree(&vocabulary, &NESTED_USES);
    assert_engines_agree(&vocabulary, &RECURSIVE_PATTERNS);
    assert_engines_agree(&vocabulary, &UNRUN_USES);
    let new = Compiler::new(&vocabulary).expect("the vocabulary compiles");
    let found = new
        .checked_by_new_core(&[source(NESTED_USES[1])])
        .expect("the new core takes nested uses");
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
    assert_eq!(chains, vec![vec![68..71, 98..101], vec![68..71, 106..109]]);
    let cyclic = new
        .checked_by_new_core(&[source(RECURSIVE_PATTERNS[0])])
        .expect("the new core takes two patterns that use each other");
    assert_eq!(
        cyclic
            .iter()
            .map(|diagnostic| diagnostic.code)
            .collect::<Vec<_>>(),
        vec![Code::RecursivePattern]
    );
}

#[test]
fn name_entries_supply_a_bool_parameter_and_a_pattern_is_used_from_another_file() {
    let vocabulary = symphonyscript_fixtures::vocabulary();
    assert_engines_agree(
        &vocabulary,
        &[
            "pattern P(default v, a = false):\n    Note/v | accent=a\nClip c:\n    P/1 | a\n    P/2 | !a\n",
            "pattern P(default v, a = false):\n    Note/(v + a)\nClip c:\n    P/1 | a\n",
            "pattern P(default v, a = 1):\n    Note/(v + a)\nClip c:\n    P/1 | a\n    P/2 | !b\n",
            "pattern P(default v, a = false):\n    Note/v | accent=a\nClip c:\n    P/1 | a !a\n",
        ],
    );
    assert_files_agree(
        &vocabulary,
        &[
            &[
                "pattern P(default v):\n    Note/(v + true)\n    Nope/1\n",
                "Clip c:\n    P/1\n    P/2\n",
            ],
            &[
                "Clip c:\n    P/1\n",
                "pattern P(default v):\n    Note/([1][v])\n",
            ],
            &[
                "pattern P(default v):\n    Clip:\n        Note/(v + true)\n",
                "P top /1\nSession s\n",
            ],
            &[
                "pattern Q(default v):\n    Note/(v + true)\n",
                "pattern P(default v):\n    Q/v\n",
                "Clip c:\n    P/1\n    for i in 0..0:\n        P/2 | bogus=2\n",
            ],
        ],
    );
}

const ARGUMENTS: [&str; 12] = [
    "pattern P(default v):\n    Note/v\nClip c:\n    P/(1.5)\n",
    "pattern P(default v):\n    Note/v | velocity=v\nClip c:\n    P/(1.5)\n",
    "pattern P(default v):\n    Note/1 | velocity=v\nClip c:\n    P/(1.5)\n    P/2\n",
    "pattern P(default v):\n    Note/(v + 1) | velocity=(v * 0.5)\nClip c:\n    P/(2.5)\n",
    "pattern P(default v):\n    Osc/v | rate=v\nSynth s:\n    P/(0.1)\n",
    "pattern P(default v):\n    Osc/v | rate=v\nSynth s:\n    P/16777217\n",
    "pattern Q(default v):\n    Osc/v | rate=v\npattern P(default v):\n    Q/v\nSynth s:\n    P/(0.1)\n    P/16777217\n",
    "pattern P(default v):\n    Note/([1, 2, 3][v])\nClip c:\n    P/(1.5)\n",
    "pattern P(default v):\n    Note/v\nClip c:\n    P/(1 + true)\n    P/([1][3])\n",
    "pattern P(default v, w = 1 + true):\n    Note/v | velocity=w\nClip c:\n    P/1\n    P/2\n",
    "pattern P(default v, w = [1][0] + true):\n    Note/v | velocity=w\nClip c:\n    P/1\n    P/2 | w=0.5\n",
    "pattern P(default v):\n    Note/v\nClip c:\n    P/bogus\n",
];

#[test]
fn a_flexible_float_argument_a_faulty_argument_and_a_faulty_default_draw_as_the_old_engine_draws_them()
 {
    let vocabulary = symphonyscript_fixtures::vocabulary();
    assert_engines_agree(&vocabulary, &ARGUMENTS);
    let new = Compiler::new(&vocabulary).expect("the vocabulary compiles");
    let found = new
        .checked_by_new_core(&[source(ARGUMENTS[9])])
        .expect("the new core takes a faulty default");
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
    assert_eq!(chains, vec![vec![72..75], vec![80..83]]);
}

const NAMED: [&str; 12] = [
    "pattern P(default v):\n    Note as v /1\n",
    "pattern P(default v):\n    Note as v /1\nClip c:\n    P/1\n    P/2\n",
    "pattern P(default v):\n    Note as gain /v\nClip c:\n    P/1\n    P/2\n",
    "pattern P(default v):\n    Note as n /v\n    Note as n /2\nClip c:\n    P/1\n    P/2\n",
    "pattern P(default v):\n    Note as n /v\n    for i in 0..2:\n        Note as m /i\nClip c:\n    P/1\n",
    "pattern P(default v):\n    Note as n /v\nClip c:\n    Note as n /3\n    P/1\n    P/2\n",
    "pattern P(default v):\n    Clip as x:\n        Note as v /v\nSession s:\n    P/1\n",
    "pattern P(default v):\n    Clip as x:\n        Note as n /v\n        Note as n /v\nSession s:\n    P/1\n",
    "pattern P(default v):\n    Note/v\nClip c:\n    P as p /1\n    P as p /2\n",
    "pattern P(default v):\n    Note/v\nClip c:\n    P as level /1\n",
    "pattern P():\n    Note\nClip c:\n    P as gain\n",
    "pattern Voice(default f):\n    Osc as osc /f\nSynth s:\n    Voice as v1 /440\nClip c:\n    Note/(s.v1.osc.freq)\n",
];

#[test]
fn a_named_placement_in_a_pattern_body_and_a_named_stand_in_use_draw_their_name_clashes() {
    let vocabulary = symphonyscript_fixtures::vocabulary();
    assert_engines_agree(&vocabulary, &NAMED);
    let new = Compiler::new(&vocabulary).expect("the vocabulary compiles");
    let found = new
        .checked_by_new_core(&[source(NAMED[8])])
        .expect("the new core takes two named stand-in uses");
    assert!(
        found
            .iter()
            .any(|diagnostic| diagnostic.code == Code::ChildNameClash)
    );
}

const PLACEHOLDER_STAND_INS: [&str; 10] = [
    "pattern P(default v):\n    Q/v\nlet Q = 1\nlet Q = 2\nClip c:\n    P as p /1\n    Note/(p.pitch)\n",
    "pattern P(default v):\n    Q/v\nlet Q = 1\nlet Q = 2\nP as p /1\nClip c:\n    Note/(p.pitch)\n",
    "pattern P(default v):\n    Q/v\nlet Q = 1\nlet Q = 2\nClip c:\n    P as p /1\n    P as p /2\n",
    "pattern P(default v):\n    Q/(v + true)\nlet Q = 1\nlet Q = 2\nClip c:\n    P as p /1\n",
    "pattern P(default v):\n    Q/v | gain=(1 + true)\nlet Q = 1\nlet Q = 2\nP as p /1\n",
    "pattern P(default v):\n    Q/v\npattern Q(default v):\n    Note/v\npattern Q(default v):\n    Note/v\nClip c:\n    P as p /1\n",
    "pattern P(default v):\n    Q/v\nlet Q = 1\nlet Q = 2\nClip c:\n    P as gain /1\n",
    "pattern P(default v):\n    Q/v\nlet Q = 1\nlet Q = 2\nClip c:\n    P as p /1\n    Note as p /2\n",
    "pattern P(default v):\n    Q/v\nlet Q = 1\nlet Q = 2\nClip c:\n    Note as p /2\n    P as p /1\n",
    "pattern P(default v):\n    Q/v\nlet Q = 1\nlet Q = 2\nSession s:\n    Clip as p\n    Clip as d:\n        P as p /1\n",
];

const PLACEHOLDER_TWINS: [&str; 3] = [
    "pattern P(default v):\n    Q/v\npattern Q(default v):\n    Note/v\nClip c:\n    P as p /1\n    Note/(p.pitch)\n",
    "pattern P(default v):\n    Note/v\nClip c:\n    P as p /1\n    Note as p /2\n",
    "pattern P(default v):\n    Note/v\nClip c:\n    P as gain /1\n",
];

#[test]
fn a_named_stand_in_whose_kind_is_a_duplicated_name_is_a_placeholder_whose_name_still_clashes() {
    let vocabulary = symphonyscript_fixtures::vocabulary();
    assert_engines_agree(&vocabulary, &PLACEHOLDER_STAND_INS);
    assert_engines_agree(&vocabulary, &PLACEHOLDER_TWINS);
}

const DEFAULTS: [&str; 12] = [
    "pattern P(default v, w = [1, 2][v]):\n    Note/v | velocity=w\nClip c:\n    P/1\n    P/5\n",
    "pattern P(default v, w = v + 1):\n    Note/w\nClip c:\n    P/1\n    P/2\n",
    "pattern P(default v, w = v + true):\n    Note/w\nClip c:\n    P/1\n    P/2\n",
    "pattern P(default v, w = v * 0.5):\n    Note/v | velocity=w\nClip c:\n    P/1\n",
    "pattern P(default v, w = x, x = 1):\n    Note/v | velocity=w\nClip c:\n    P/1\n",
    "pattern P(default v, w = v + 300000000000):\n    Note/w\nClip c:\n    P/1\n",
    "pattern P(default v, w = v):\n    Note/w | velocity=w\nClip c:\n    P/1\n    P/2 | w=3\n",
    "pattern P(default v, n = v):\n    for i in 0..n:\n        Note/i\nClip c:\n    P/2\n    P/3\n",
    "pattern P(default v, w = [1][2]):\n    Note/v | velocity=w\nClip c:\n    P/1\n    P/2\n",
    "pattern Q(default v, w = [1][2]):\n    Note/v | velocity=w\npattern P(default v):\n    Q/v\nClip c:\n    P/1\n    P/2\n",
    "pattern P(default v, w = 300000000000):\n    Note/v | velocity=w\nClip c:\n    P/1\n",
    "pattern P(default v, w = 1 / 0):\n    Note/v | velocity=w\nClip c:\n    P/1\n    P/2 | w=3\n",
];

#[test]
fn a_default_reads_the_earlier_parameters_and_its_value_faults_draw_at_each_outermost_use() {
    let vocabulary = symphonyscript_fixtures::vocabulary();
    assert_engines_agree(&vocabulary, &DEFAULTS);
    let new = Compiler::new(&vocabulary).expect("the vocabulary compiles");
    let found = new
        .checked_by_new_core(&[source(DEFAULTS[9])])
        .expect("the new core takes a default whose value faults under a nested use");
    let chains: Vec<Vec<std::ops::Range<u32>>> = found
        .iter()
        .filter(|diagnostic| diagnostic.code == Code::IndexOutOfRange)
        .map(|diagnostic| {
            diagnostic
                .related
                .iter()
                .map(|related| related.span.range.clone())
                .collect()
        })
        .collect();
    assert_eq!(chains, vec![vec![84..87, 100..103], vec![84..87, 108..111]]);
}

const READS_IN_USES: [&str; 15] = [
    "pattern P(default v):\n    Note/v | velocity=0.5\nClip c:\n    P as p /1\n    Note/(p.velocity)\n",
    "pattern P(default v):\n    Note/v | velocity=0.5\nClip c:\n    P as p /1\n    Note/(p.velocity)\n    Note/(p.pitch)\n",
    "pattern P(default v):\n    Note/v | velocity=(v * 0.25)\nClip c:\n    P as p /1\n    P as q /2\n    Note/(p.velocity + q.velocity)\n",
    "pattern P(default v):\n    Note/v | velocity=true\nClip c:\n    P as p /1\n    Note/(p.velocity)\n",
    "pattern P(default v):\n    Note/v\nClip c:\n    P as p /1\n    Note/(p.pitch + 0.5)\n    Note/(p.pitch)\n",
    "pattern P(default v):\n    Q/v\npattern Q(default v):\n    Note/v\nClip c:\n    P as p /1\n    Note/(p.pitch)\n",
    "pattern P(default v):\n    Note/v | velocity=([0.5, 1.0][v])\nClip c:\n    P as p /1\n    P as q /5\n    Note/(q.velocity)\n",
    "state s(1)\npattern P(default v):\n    Note/v | velocity=0.5\nClip c:\n    P as p /1\n    Note/(p.velocity) | velocity=(p.velocity)\n",
    "pattern P(default v):\n    Note/v\nlet k = c.p.pitch\nClip c:\n    P as p /1\n    Note/k\n",
    "pattern P(default v, w = v * 2):\n    Note/w\nClip c:\n    P as p /3\n    Note/(p.pitch)\n",
    "pattern P(default v):\n    Note/v\nClip c:\n    P as p /1\n    P as q /5\n    Note/([1, 2][p.pitch])\n    Note/([1, 2][q.pitch])\n",
    "pattern P(default v):\n    Note/(v + 1)\nlet k = [1, 2, 3][c.p.pitch]\nClip c:\n    P as p /1\n    Note/k\n",
    "pattern P(default v):\n    Note/(v * 1000000)\nClip c:\n    P as p /3000\n    Note/(p.pitch)\n",
    "state s(1)\npattern P(default v):\n    Note/v | velocity=(0.5 + s)\nClip c:\n    P as p /1\n    Note/1 | velocity=(p.velocity)\n",
    "pattern P(default v, w = [1, 2][v]):\n    Note/w\nClip c:\n    P as p /1\n    P as q /7\n    Note/(p.pitch + q.pitch)\n",
];

#[test]
fn a_path_into_a_stand_in_rows_attribute_written_in_the_pattern_body_reads_it_in_that_rows_env() {
    let vocabulary = symphonyscript_fixtures::vocabulary();
    assert_engines_agree(&vocabulary, &READS_IN_USES);
    let new = Compiler::new(&vocabulary).expect("the vocabulary compiles");
    let found = new
        .checked_by_new_core(&[source(READS_IN_USES[10])])
        .expect("the new core takes reads of two uses' rows");
    let ranges: Vec<std::ops::Range<u32>> = found
        .iter()
        .filter(|diagnostic| diagnostic.code == Code::IndexOutOfRange)
        .map(|diagnostic| diagnostic.span.range.clone())
        .collect();
    assert_eq!(ranges, vec![113..120]);
}

const CELL_ARGUMENTS: [&str; 13] = [
    "state s(1)\npattern P(default v):\n    Note/v\nClip c:\n    P/s\n",
    "state s(1.5)\npattern P(default v):\n    Note/v\nClip c:\n    P/s\n",
    "state s(1)\npattern P(default v):\n    Note/(v + true)\nClip c:\n    P/s\n    P/2\n",
    "state s(1)\npattern Q(default v):\n    Note/v\npattern P(default v):\n    Q/v\nClip c:\n    P/s\n",
    "state s(1)\npattern P(default v):\n    Note/v | velocity=v\nClip c:\n    P/s\n",
    "state s(1)\nlet k = s + 1\npattern P(default v):\n    Note/v\nClip c:\n    P/k\n",
    "state s(1)\npattern P(default v):\n    Note/v\nClip c:\n    P/(s + 1)\n    P/(s + 300000000000)\n",
    "state s(1)\npattern P(default v):\n    Note/([1, 2][v])\nClip c:\n    P/s\n",
    "state s(1)\nstate t = s * 2\npattern P(default v):\n    Note/v\n    Note/(v * 2)\nClip c:\n    P/t\n    P/s\n    P/3\n",
    "state s(1)\npattern P(default v):\n    Note/v\nClip c:\n    P/(s + 300000000000)\n",
    "state s(1)\npattern P(default v):\n    Note/v\n    Note/1 | velocity=v\nClip c:\n    P/(s + 3000000000)\n",
    "state s(1)\nClip c:\n    Note/1 | velocity=(s + 300000000000)\n",
    "state s(1)\nClip c:\n    Note/1 | velocity=(s + 1)\n",
];

const PATH_ARGUMENTS: [&str; 11] = [
    "pattern P(default v):\n    Note/v\nClip c:\n    P/(d.gain)\nClip d | gain=0.5\n",
    "pattern P(default v):\n    Note/1 | velocity=v\nClip c:\n    P/(d.gain)\nClip d | gain=0.5\n",
    "pattern P(default v):\n    Note/v\nClip c:\n    P/(d.bogus)\nClip d\n",
    "pattern P(default v):\n    Q/v\npattern Q(default v):\n    Note/1 | velocity=v\nClip c:\n    P/(d.gain)\n    P/(e.gain)\nClip d | gain=0.5\nClip e | gain=2\n",
    "pattern P(default v):\n    Plays | gain=v: e\nClip c:\n    Plays | gain=to.gain: d\n    P/(c.gain)\nClip d\nClip e\n",
    "pattern P(default v):\n    Note/([1, 2, 3][v])\nClip c:\n    Note as n /1\n    Note as m /5\n    P/(n.pitch)\n    P/(m.pitch)\n",
    "pattern P(default v):\n    Note/(v * 1000000)\nClip c:\n    Note as n /3000\n    P/(n.pitch)\n",
    "pattern P(default v):\n    Q/(v + 1)\npattern Q(default v):\n    Note/([1, 2][v])\nClip c:\n    Note as n /0\n    Note as m /3\n    P/(n.pitch)\n    P/(m.pitch)\n",
    "state s(1)\npattern P(default v):\n    Note/(v + 1)\nClip c:\n    Note as n /s\n    P/(n.pitch)\n",
    "pattern P(default v):\n    Note/v\nClip c:\n    P/1\n    P/(d.gain)\n    P/(e.gain)\nClip d | gain=0.5\nClip e | gain=0.25\n",
    "pattern P(default v):\n    Note/1 | velocity=v\nClip c:\n    P/(d.gain)\n    P/(e.gain)\nClip d | gain=0.5\nClip e | gain=0.25\n",
];

fn codes_of(new: &Compiler, text: &str) -> Vec<(Code, std::ops::Range<u32>, usize)> {
    new.checked_by_new_core(&[source(text)])
        .unwrap_or_else(|| panic!("the new core takes {text}"))
        .iter()
        .map(|diagnostic| {
            (
                diagnostic.code,
                diagnostic.span.range.clone(),
                diagnostic.related.len(),
            )
        })
        .collect()
}

#[test]
fn an_argument_that_reads_a_cell_or_a_path_keys_its_use_once_the_cells_and_paths_are_known() {
    let vocabulary = symphonyscript_fixtures::vocabulary();
    assert_engines_agree(&vocabulary, &CELL_ARGUMENTS);
    assert_engines_agree(&vocabulary, &PATH_ARGUMENTS);
    let new = Compiler::new(&vocabulary).expect("the vocabulary compiles");
    assert_eq!(
        codes_of(
            &new,
            "state s(1)\npattern P(default v):\n    Note/1 | velocity=v\nClip c:\n    P/(s + 300000000000)\n"
        ),
        vec![
            (Code::NoEntryInstance, 0..0, 0),
            (Code::IntegerCell, 6..7, 0),
            (Code::DoesNotFit, 76..88, 0),
        ]
    );
    assert_eq!(
        codes_of(
            &new,
            "state s(1)\npattern P(default v):\n    Note/1 | velocity=v\nClip c:\n    P/(s + 1)\n"
        ),
        vec![
            (Code::NoEntryInstance, 0..0, 0),
            (Code::IntegerCell, 6..7, 0)
        ]
    );
    assert_eq!(
        codes_of(
            &new,
            "state s(1)\npattern P(default v):\n    for i in 0..v:\n        Note/i\nClip c:\n    P/s\n"
        ),
        vec![
            (Code::NoEntryInstance, 0..0, 0),
            (Code::ReactiveIterable, 49..50, 1),
        ]
    );
}

const OWN_NAMES: [&str; 9] = [
    "pattern P(default v):\n    Note as n /v\n    Note/(n.pitch)\nClip c:\n    P/1\n",
    "pattern P(default v):\n    Note as n /v\n    Note/(n.pitch)\nClip c:\n    P/1\n    P/2\n",
    "pattern P(default v):\n    Note/(n.pitch)\n    Note as n /v\nClip c:\n    P/1\n",
    "pattern P(default v):\n    Clip as x:\n        Note as n /v\n    Note/(x.n.pitch + 0.5)\nSession s:\n    P/1\n",
    "pattern P(default v):\n    Note as n /v | velocity=0.5\n    Note/([1, 2][n.pitch]) | velocity=(n.velocity * 2)\nClip c:\n    P/1\n    P/5\n",
    "pattern P(default v):\n    Note as n /v\n    Note/(n.bogus)\nClip c:\n    P/1\n    P/2\n",
    "pattern P(default v):\n    Note as n /(v * 1000000)\n    Note/(n.pitch)\nClip c:\n    P/3000\n",
    "pattern Q(default v):\n    Note as n /v\n    Note/([1, 2][n.pitch])\npattern P(default v):\n    Q/(v + 1)\nClip c:\n    P/0\n    P/4\n",
    "pattern P(default v):\n    Note as n /v\n    for i in 0..2:\n        Note/(n.pitch + i)\nClip c:\n    P/1\n",
];

#[test]
fn a_pattern_body_reads_the_as_names_of_its_own_children_in_each_uses_env() {
    let vocabulary = symphonyscript_fixtures::vocabulary();
    assert_engines_agree(&vocabulary, &OWN_NAMES);
    let new = Compiler::new(&vocabulary).expect("the vocabulary compiles");
    let found = new
        .checked_by_new_core(&[source(OWN_NAMES[4])])
        .expect("the new core takes a body reading its own child");
    let chains: Vec<Vec<std::ops::Range<u32>>> = found
        .iter()
        .filter(|diagnostic| diagnostic.code == Code::IndexOutOfRange)
        .map(|diagnostic| {
            diagnostic
                .related
                .iter()
                .map(|related| related.span.range.clone())
                .collect()
        })
        .collect();
    assert_eq!(chains, vec![vec![129..132]]);
}
