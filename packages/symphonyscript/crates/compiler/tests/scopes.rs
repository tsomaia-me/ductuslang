mod support;

use symphonyscript::{Code, Compiler};
use symphonyscript_core::Vocabulary;
use symphonyscript_fixtures::{bounded_vocabulary, vocabulary};

type Case = (&'static str, &'static [(Code, &'static str, usize)]);

type OwnedRow = (Vec<String>, Vec<(Code, &'static str)>);

type BorrowedRow<'a> = (Vec<&'a str>, &'a [(Code, &'static str)]);
#[test]
fn a_name_resolves_in_scope_or_is_an_error() {
    let cases: &[Case] = &[
        ("let a = b\n", &[(Code::UnknownName, "b", 0)]),
        ("fn f(x): y\n", &[(Code::UnknownName, "y", 0)]),
        (
            "fn f(x):\n    let y = x\n    y + z\n",
            &[(Code::UnknownName, "z", 0)],
        ),
        ("fn f(p = q, q = 1): p\n", &[(Code::UnknownName, "q", 0)]),
        ("fn f(p, q = p): q\n", &[]),
        (
            "pattern P():\n    Note/hook\nClip c:\n    Note as hook\n    P\n",
            &[(Code::UnknownName, "hook", 0)],
        ),
        ("Clip c:\n    Note/hook.pitch\n    Note as hook\n", &[]),
        ("Synth s:\n    Osc | wave=Saw\n", &[]),
        ("let w = Purple\n", &[(Code::UnknownName, "Purple", 0)]),
    ];
    for (text, expected) in cases {
        support::expect_codes(text, expected);
    }
}

#[test]
fn a_kind_names_a_node_kind_or_a_pattern() {
    support::expect_codes("Clip c:\n    Foo\n", &[(Code::UnknownKind, "Foo", 0)]);
    support::expect_codes(
        "let base = 1\nClip c:\n    base\n",
        &[(Code::UnknownKind, "base", 1)],
    );
    let text = "Clip c:\n    Note hook/60\n";
    let diagnostics = support::checked(text);
    assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
    assert_eq!(diagnostics[0].code, Code::UnknownKind);
    assert_eq!(diagnostics[0].span.range, 17..21);
    assert_eq!(
        diagnostics[0].help.as_deref(),
        Some("to name the placement, write `Note as hook`")
    );
}

#[test]
fn a_child_or_wire_name_repeated_in_its_body_or_an_enclosing_one_is_an_error() {
    let cases: &[Case] = &[
        (
            "Clip c:\n    Note as dup\n    Note as dup\n",
            &[(Code::ChildNameClash, "dup", 1)],
        ),
        (
            "input x = true\nClip c:\n    when x:\n        Note as dup\n    else:\n        Note as dup\n",
            &[(Code::ChildNameClash, "dup", 1)],
        ),
        (
            "Clip c:\n    Note as dup\n    Plays as dup: c\n",
            &[(Code::ChildNameClash, "dup", 1)],
        ),
        (
            "Track t:\n    Synth as syn:\n        Osc as syn\n",
            &[(Code::ChildNameClash, "syn", 1)],
        ),
        (
            "pattern P():\n    Note as dup\nClip c:\n    Note as dup\n    P\n",
            &[],
        ),
    ];
    for (text, expected) in cases {
        support::expect_codes(text, expected);
    }
}

#[test]
fn a_loop_variable_equal_to_a_visible_name_is_an_error() {
    let cases: &[Case] = &[
        (
            "let idx = 1\nClip c:\n    for idx in 0..2:\n        Note\n",
            &[(Code::ForVariableClash, "idx", 1)],
        ),
        (
            "Clip c:\n    for sin in 0..2:\n        Note\n",
            &[(Code::ForVariableClash, "sin", 0)],
        ),
        (
            "Clip c:\n    Note as nn\n    [for nn in 0..2: Note]\n",
            &[(Code::ForVariableClash, "nn", 1)],
        ),
        ("Clip c:\n    for i in 0..2:\n        Note/i\n", &[]),
    ];
    for (text, expected) in cases {
        support::expect_codes(text, expected);
    }
}

#[test]
fn parameters_and_locals_are_declared_once_and_never_like_a_parameter() {
    let cases: &[Case] = &[
        ("fn f(v, v): v\n", &[(Code::DuplicateLocal, "v", 1)]),
        (
            "pattern P(v, v):\n    Note/v\n",
            &[(Code::DuplicateLocal, "v", 1)],
        ),
        (
            "fn f(a, b): a + b\nClip c:\n    Note/(f(1, 2))\n    Note/(f(b=3, a=4))\n",
            &[],
        ),
        (
            "fn f(x):\n    let a = 1\n    let a = 2\n    a\n",
            &[(Code::DuplicateLocal, "a", 1)],
        ),
        (
            "fn f(x):\n    let x = 1\n    x\n",
            &[(Code::DuplicateLocal, "x", 1)],
        ),
        (
            "pattern P(root):\n    let root = 1\n    Note/root\n",
            &[(Code::DuplicateLocal, "root", 1)],
        ),
    ];
    for (text, expected) in cases {
        support::expect_codes(text, expected);
    }
}

#[test]
fn a_functions_local_lets_are_visible_throughout_its_body() {
    let later = "fn f(x):\n    let a = b + x\n    let b = 1\n    a\nClip c:\n";
    support::expect_codes(&format!("{later}    Note/(f(1))\n"), &[]);
    let text = format!("{later}    Note/(f(2147483647))\n");
    support::expect_codes(&text, &[(Code::DoesNotFit, "a", 1)]);
    expect_a_cycle_of_a_and_b(
        "fn f():\n    let a = b\n    let b = a\n    a\nClip c:\n    Note/(f())\n",
    );
}

fn expect_a_cycle_of_a_and_b(text: &str) {
    let diagnostics = support::checked(text);
    assert_eq!(diagnostics.len(), 1, "{text}: {diagnostics:?}");
    assert_eq!(diagnostics[0].code, Code::DeclarationCycle, "{text}");
    let name_in = |line| {
        let (start, _) = support::at(text, line, 0);
        start..start + 1
    };
    assert_eq!(diagnostics[0].span.range, name_in("a = b"), "{text}");
    let secondary: Vec<_> = diagnostics[0]
        .related
        .iter()
        .map(|related| &related.span)
        .map(|span| span.range.clone())
        .collect();
    assert_eq!(secondary, vec![name_in("b = a")], "{text}");
}

#[test]
fn a_cycle_among_the_lets_of_an_unused_function_or_pattern_is_reported() {
    expect_a_cycle_of_a_and_b("fn f():\n    let a = b\n    let b = a\n    a\nSession s\n");
    expect_a_cycle_of_a_and_b(
        "pattern P():\n    let a = b\n    let b = a\n    Note/a\nSession s\n",
    );
    let acyclic = "fn g():\n    let a = 1\n    let b = a\n    b\nSession s\n";
    assert_eq!(support::checked_all(acyclic), vec![]);
}

#[test]
fn a_local_cycle_is_reported_once() {
    let g = "fn g(x): x\n";
    let through_a_call = "fn f():\n    let a = g(a)\n    a\n";
    let rows = [
        format!("{g}{through_a_call}Clip c:\n    Note/(f())\nSession s\n"),
        format!("{g}{through_a_call}Session s\n"),
        "fn f():\n    let a = b\n    let b = a\n    a\nSession s\n".to_string(),
        "fn f(): a\nlet a = f()\nSession s\n".to_string(),
    ];
    for text in &rows {
        let found: Vec<Code> = support::checked(text)
            .iter()
            .map(|diagnostic| diagnostic.code)
            .collect();
        assert_eq!(found, vec![Code::DeclarationCycle], "{text}");
    }
}

#[test]
fn parameters_with_defaults_come_last_and_one_is_marked_default() {
    support::expect_codes("fn f(a = 1, b): a + b\n", &[(Code::ParameterOrder, "b", 0)]);
    support::expect_codes(
        "pattern P(default a, default b):\n    Note/a\n",
        &[(Code::DuplicateDefaultParameter, "default b", 0)],
    );
}

#[test]
fn placements_in_a_for_body_are_anonymous() {
    let cases: &[Case] = &[
        (
            "Clip c:\n    for i in 0..2:\n        Note as nn\n",
            &[(Code::NamedInFor, "nn", 0)],
        ),
        (
            "Clip c:\n    [for i in 0..2: Note as nn]\n",
            &[(Code::NamedInFor, "nn", 0)],
        ),
        (
            "input x = true\nClip c:\n    for i in 0..2:\n        when x:\n            Note as nn\n",
            &[(Code::NamedInFor, "nn", 0)],
        ),
    ];
    for (text, expected) in cases {
        support::expect_codes(text, expected);
    }
}

#[test]
fn to_stands_only_in_a_wire_clause_and_ends_at_an_attribute() {
    let cases: &[Case] = &[
        ("let a = to.gain\n", &[(Code::MisplacedTo, "to", 0)]),
        ("fn f(x): to.gain\n", &[(Code::MisplacedTo, "to", 0)]),
        (
            "Clip a:\n    Note\nClip c:\n    Plays: to.x\n",
            &[(Code::MisplacedTo, "to", 0)],
        ),
        (
            "Clip a:\n    Note\nClip c:\n    Plays | gain=to: a\n",
            &[(Code::MisplacedTo, "to", 0)],
        ),
        (
            "Clip a:\n    Note\nClip c:\n    Plays | gain=to.a.b: a\n",
            &[(Code::UnknownAttribute, "a", 3)],
        ),
        (
            "Clip a:\n    Note as n\nClip c:\n    Plays | gain=to.n: a\n",
            &[(Code::MisplacedTo, "n", 2)],
        ),
        (
            "Clip a:\n    Note as n\nClip c:\n    Plays | gain=to.n.velocity: a\n",
            &[],
        ),
        (
            "Clip a:\n    Note\nClip c:\n    Plays | gain=to.gain: a\n",
            &[],
        ),
    ];
    for (text, expected) in cases {
        support::expect_codes(text, expected);
    }
}

#[test]
fn a_type_names_a_vocabulary_enum_and_lengths_of_at_least_one() {
    support::expect_codes("input a: Shape = 1\n", &[(Code::UnknownType, "Shape", 0)]);
    support::expect_codes("input a: i32[0] = [1]\n", &[(Code::ZeroLength, "0", 0)]);
}

#[test]
fn a_local_name_equal_to_a_variant_is_an_error() {
    let cases: &[Case] = &[
        ("fn f(Saw): 1\n", &[(Code::VariantName, "Saw", 0)]),
        (
            "Clip c:\n    for Sine in 0..2:\n        Note\n",
            &[(Code::VariantName, "Sine", 0)],
        ),
        (
            "Clip c:\n    Note as Square\n",
            &[(Code::VariantName, "Square", 0)],
        ),
    ];
    for (text, expected) in cases {
        support::expect_codes(text, expected);
    }
}

#[test]
fn a_child_wire_or_loop_name_equal_to_a_visible_local_is_refused_naming_both() {
    let tone_note = "pattern Mk(default root):\n    let tone = root + 2\n    Note as tone /tone\n    Note/(tone + 7)\n";
    let stand_in = "pattern Mk(default root):\n    let tone = root + 2\n    Clip as tone:\n        Note/tone\n";
    let named_loop = "Clip c:\n    Note as i /60\n    for i in 0..2:\n        Note/(62 + i)\n";
    let loop_named = "Clip c:\n    for i in 0..2:\n        Note/(62 + i)\n    Note as i /60\n";
    let rows: &[support::Row] = &[
        (
            &[tone_note, "Clip c:\n    Mk/60\n"],
            &[(Code::ChildNameClash, "tone")],
        ),
        (
            &[tone_note, "Clip c:\n    Mk/60\n    Mk/62\n"],
            &[(Code::ChildNameClash, "tone")],
        ),
        (&[tone_note], &[(Code::ChildNameClash, "tone")]),
        (&[stand_in, "Mk m /60\n"], &[(Code::ChildNameClash, "tone")]),
        (&[stand_in], &[(Code::ChildNameClash, "tone")]),
        (
            &[
                "pattern P(default root, extra):\n    Note as extra /root\n    Note/(root + extra)\n",
                "Clip c:\n    P/60 | extra=7\n",
            ],
            &[(Code::ChildNameClash, "extra")],
        ),
        (
            &[
                "pattern P(default root):\n    let tone = root\n    Clip:\n        Note as tone /60\n",
                "P p /60\n",
            ],
            &[(Code::ChildNameClash, "tone")],
        ),
        (
            &[
                "pattern L(default root, again):\n    Clip:\n        Note/root\n        Plays as again: again\n",
                "Clip x:\n    Note\n",
                "L l /36 | again=x\n",
            ],
            &[(Code::ChildNameClash, "again")],
        ),
        (&[named_loop], &[(Code::ForVariableClash, "i")]),
        (&[loop_named], &[(Code::ForVariableClash, "i")]),
        (
            &[
                "let i = 1\n",
                "Clip c:\n    for i in 0..2:\n        Note/(60 + i)\n",
            ],
            &[(Code::ForVariableClash, "i")],
        ),
        (
            &[
                "pattern P(default root):\n    for root in 0..2:\n        Note/root\n",
                "Clip c:\n    P/60\n",
            ],
            &[(Code::ForVariableClash, "root")],
        ),
        (
            &["Clip c:\n    for i in 0..2:\n        for i in 0..2:\n            Note/(60 + i)\n"],
            &[(Code::ForVariableClash, "i")],
        ),
        (
            &["Clip c:\n    Note as i /60\n    [for i in 0..3: Note/(62 + i * 4)]\n"],
            &[(Code::ForVariableClash, "i")],
        ),
        (&["let hook = 1\n", "Clip c:\n    Note as hook /60\n"], &[]),
        (
            &[
                "Clip a:\n    for i in 0..2:\n        Note/(60 + i)\n",
                "Clip b:\n    for i in 0..2:\n        Note/(60 + i)\n",
            ],
            &[],
        ),
        (
            &[
                "pattern A():\n    for i in 0..2:\n        Note/(60 + i)\n",
                "pattern B():\n    Note as i /60\n",
                "Clip c:\n    A\n    B\n",
            ],
            &[],
        ),
        (
            &[
                "let root = 1\n",
                "pattern P(default root):\n    Note/root\n",
                "Clip c:\n    P/60\n",
            ],
            &[],
        ),
        (
            &[
                "pattern Arp(default root, steps = 4, gap = 4):\n    for i in 0..steps:\n        Note/(root + i * gap)\n",
                "Clip verse:\n    Arp/48\n    Arp/60 | steps=8 gap=3\n",
            ],
            &[],
        ),
        (
            &[
                "pattern Riff(default root):\n    Clip:\n        Note/root Note/(root + 7) Note/(root + 12)\n",
                "Riff low /36\n",
                "Riff high /48\n",
            ],
            &[],
        ),
        (
            &[
                "pattern Loop(default root, again):\n    Clip:\n        Note/root Note/(root + 7)\n        Plays: again\n",
                "Loop low /36 | again=low\n",
            ],
            &[],
        ),
        (
            &[
                "fn f(x):\n    let i = x\n    i + 1\n",
                "let i = 1\n",
                "Clip c:\n    Note/(f(i))\n",
            ],
            &[],
        ),
    ];
    let vocabulary = symphonyscript_fixtures::vocabulary();
    support::assert_rows_in_every_order(&vocabulary, rows);
    let compiler = symphonyscript::Compiler::new(&vocabulary).expect("the vocabulary compiles");
    for (items, _) in rows {
        let text = items.concat();
        for diagnostic in support::errors(compiler.check(&[support::source(&text)])) {
            let named = |range: &std::ops::Range<u32>| {
                text.get(range.start as usize..range.end as usize)
                    .unwrap_or_default()
            };
            let name = named(&diagnostic.span.range);
            assert!(
                diagnostic
                    .related
                    .iter()
                    .any(|related| related.span.range != diagnostic.span.range
                        && named(&related.span.range) == name),
                "{text}{diagnostic:?}"
            );
        }
    }
}

type Summary = (
    fn() -> Vocabulary,
    &'static [&'static str],
    Code,
    &'static str,
    &'static str,
);

#[test]
fn a_summary_reads_as_a_sentence_in_the_programs_names() {
    let rows: &[Summary] = &[
        (
            vocabulary,
            &[
                "Clip verse:\n    Plays | gain=to.hook.nosuch: chorus\n",
                "Clip chorus:\n    Note as hook /72\n",
            ],
            Code::UnknownAttribute,
            "nosuch",
            "the destination, the `Note` named `hook`, has no attribute `nosuch`",
        ),
        (
            vocabulary,
            &[
                "input x = 1\n",
                "state s = x{1, 0, 2}\n",
                "Clip c:\n    Note when s > 0\n",
            ],
            Code::ExpressionSyntax,
            ",",
            "expected `}`, found `,`",
        ),
        (
            bounded_vocabulary,
            &[
                "input i = 0\n",
                "A one\n",
                "B two\n",
                "Root r:\n    Item\n    Link: r\n    Pick | level=to.x: [one, two][i]\n",
            ],
            Code::ToCandidates,
            "to.x",
            "the destination candidates hold `x` with different types or unit families, the `A` named `one` and the `B` named `two` among them, and a `to` read takes one",
        ),
        (
            support::to_array_vocabulary,
            &[
                "input sec = 0\n",
                "Clip b:\n    Note\n",
                "Lane l\n",
                "Clip a:\n    Cues | gain=to.seq[0]: [b, l][sec]\n",
            ],
            Code::ToCandidates,
            "to.seq",
            "the destination candidates hold `seq` with different types or unit families, the `Clip` named `b` and the `Lane` named `l` among them, and a `to` read takes one",
        ),
    ];
    for (vocabulary, items, code, at, summary) in rows {
        let compiler = Compiler::new(&vocabulary()).expect("the vocabulary compiles");
        for order in support::permutations(items) {
            let text = order.concat();
            let found = support::errors(compiler.check(&[support::source(&text)]));
            let [diagnostic] = found.as_slice() else {
                panic!("{text}{found:?}");
            };
            let range = &diagnostic.span.range;
            assert_eq!(
                (
                    diagnostic.code,
                    &text[range.start as usize..range.end as usize],
                    diagnostic.summary.as_str()
                ),
                (*code, *at, *summary),
                "{text}"
            );
        }
    }
}

#[test]
fn in_code_that_nothing_expands_a_flag_naming_no_attribute_is_a_name_error_and_a_repeated_one_is_not()
 {
    let clip = "Clip c:\n    Note\n";
    let flag = (Code::UnknownFlag, "'");
    let rows: &[support::Row] = &[
        (&["Clip c:\n    for i in 0..0:\n        Note''\n"], &[]),
        (&["pattern P():\n    Note'''\n", clip], &[]),
        (&["pattern P():\n    Synth:\n        Osc'\n", clip], &[flag]),
        (&["Synth s:\n    for i in 0..0:\n        Osc'\n"], &[flag]),
        (
            &["Clip c:\n    for i in 0..1:\n        Note''\n"],
            &[(Code::DuplicateAttribute, "'")],
        ),
        (
            &["pattern P():\n    Synth:\n        Osc'\n", "P p\n"],
            &[flag],
        ),
    ];
    support::assert_rows_in_every_order(&symphonyscript_fixtures::vocabulary(), rows);
}

#[test]
fn in_code_that_nothing_expands_a_path_from_an_as_name_resolves_through_its_written_kind() {
    let clip = "Clip c:\n    Note\n";
    let used = "Clip c:\n    P\n";
    let q = "pattern Q():\n    Note\n";
    let r = "pattern R():\n    Q\n";
    let v = "pattern V():\n    Note as a\n    Note as b\n";
    let note = "pattern P():\n    Note as n\n    Note | velocity=n.nothing\n";
    let named_use = "pattern P():\n    Q as q\n    Note | velocity=q.nothing\n";
    let wire = "pattern P():\n    Plays as w /0.5: c\n    Note | velocity=w.nothing\n";
    let child =
        "pattern P():\n    Clip as k:\n        Note as m\n    Note | velocity=k.m.nothing\n";
    let group = "pattern P():\n    V as v\n    Note | velocity=v.nothing\n";
    let member = "pattern P():\n    V as v\n    Note | velocity=v.a.nothing\n";
    let chained = "pattern P():\n    R as r\n    Note | velocity=r.nothing\n";
    let arm = "pattern P():\n    when true:\n        Note as m\n    Note | velocity=m.nothing\n";
    let deeper = "pattern P():\n    Note as n\n    Note | velocity=n.nothing.velocity\n";
    let looped =
        "pattern P():\n    Note as n\n    for i in 0..0:\n        Note | velocity=n.nothing\n";
    let voice = "pattern Voice():\n    Note\n    Note as m\n";
    let verse = "Clip verse:\n    Note\n";
    let past_note = "pattern P():\n    Note as n\n    Note | velocity=n.velocity.x\n";
    let past_member = "pattern P():\n    Voice as k\n    Note | velocity=k.m.velocity.x\n";
    let past_wire = "pattern P():\n    Plays as w: verse\n    Note | velocity=w.gain.x\n";
    let attribute = (Code::UnknownAttribute, "nothing");
    let anonymous = [(Code::NamedInFor, "n"), (Code::UnknownName, "n")];
    let rows: &[support::Row] = &[
        (&[note, clip], &[attribute]),
        (&[q, named_use, clip], &[attribute]),
        (&[wire, clip], &[attribute]),
        (&[child, clip], &[attribute]),
        (&[v, group, clip], &[attribute]),
        (&[v, member, clip], &[attribute]),
        (&[q, r, chained, clip], &[attribute]),
        (&[arm, clip], &[attribute]),
        (&[deeper, clip], &[attribute]),
        (&[looped, clip], &[attribute]),
        (
            &[
                "pattern P():\n    Nope as n\n    Note | velocity=n.nothing\n",
                clip,
            ],
            &[(Code::UnknownKind, "Nope")],
        ),
        (
            &[
                "pattern P():\n    Note as n\n    Note | velocity=n.velocity\n",
                clip,
            ],
            &[],
        ),
        (
            &[
                "pattern P():\n    Plays as w /0.5: c\n    Note | velocity=w.gain\n",
                clip,
            ],
            &[],
        ),
        (
            &[
                "pattern P():\n    V as v\n    Note | velocity=v.a.velocity\n",
                v,
                clip,
            ],
            &[],
        ),
        (
            &[
                "pattern P():\n    Clip as k:\n        Note as m\n    Note | velocity=k.m.velocity\n",
                clip,
            ],
            &[],
        ),
        (
            &[past_note, clip],
            &[(Code::UnknownAttribute, "n.velocity.x")],
        ),
        (
            &[voice, past_member, clip],
            &[(Code::UnknownAttribute, "k.m.velocity.x")],
        ),
        (
            &[verse, past_wire, clip],
            &[(Code::UnknownAttribute, "w.gain.x")],
        ),
        (
            &[
                voice,
                "pattern P():\n    Voice as k\n    Note | velocity=k.m.velocity\n",
                clip,
            ],
            &[],
        ),
        (
            &[
                verse,
                "pattern P():\n    Plays as w: verse\n    Note | velocity=w.gain\n",
                clip,
            ],
            &[],
        ),
        (
            &[past_note, used],
            &[(Code::UnknownAttribute, "n.velocity.x")],
        ),
        (
            &[voice, past_member, used],
            &[(Code::UnknownAttribute, "k.m.velocity.x")],
        ),
        (
            &[verse, past_wire, used],
            &[(Code::UnknownAttribute, "w.gain.x")],
        ),
        (&[note, used], &[attribute]),
        (&[q, named_use, used], &[attribute]),
        (&[wire, used], &[attribute]),
        (&[v, group, used], &[attribute]),
        (&[v, member, used], &[attribute]),
        (&[q, r, chained, used], &[attribute]),
        (
            &["Clip c:\n    Note as n\n    for i in 0..0:\n        Note | velocity=n.nothing\n"],
            &[attribute],
        ),
        (
            &[
                q,
                "Clip c:\n    Q as q\n    for i in 0..0:\n        Note | velocity=q.nothing\n",
            ],
            &[attribute],
        ),
        (
            &[
                "Clip c:\n    Plays as w /0.5: c\n    for i in 0..0:\n        Note | velocity=w.nothing\n",
            ],
            &[attribute],
        ),
        (
            &["Clip c:\n    Note as n\n    for i in 0..0:\n        Note | velocity=n.velocity\n"],
            &[],
        ),
        (
            &[
                "Clip c:\n    for i in 0..0:\n        Note as n\n        Note | velocity=n.nothing\n",
            ],
            &anonymous,
        ),
        (
            &[
                "Clip c:\n    for i in 0..0:\n        when true:\n            Note as n\n            Note | velocity=n.nothing\n",
            ],
            &anonymous,
        ),
        (
            &[
                "pattern P():\n    for i in 0..0:\n        Note as n\n        Note | velocity=n.nothing\n",
                clip,
            ],
            &anonymous,
        ),
    ];
    support::assert_rows_in_every_order(&vocabulary(), rows);
    let summaries = |text: String| -> Vec<String> {
        support::errors(support::checked_with(&vocabulary(), &text))
            .into_iter()
            .filter(|diagnostic| diagnostic.code == Code::UnknownAttribute)
            .map(|diagnostic| diagnostic.summary)
            .collect()
    };
    for (declared, body) in [
        ("", note),
        (q, named_use),
        ("", wire),
        (v, group),
        (v, member),
        (&*format!("{q}{r}"), chained),
        ("", arm),
        ("", past_note),
        (voice, past_member),
        (verse, past_wire),
    ] {
        let unused = summaries(format!("{declared}{body}{clip}"));
        assert_eq!(unused.len(), 1, "{body}");
        assert_eq!(
            unused,
            summaries(format!("{declared}{body}{used}")),
            "{body}"
        );
    }
}

#[test]
fn in_code_that_nothing_expands_a_to_path_resolves_against_a_destination_that_needs_no_expansion() {
    let verse = "Clip verse:\n    Note as hook\n";
    let (flag, track, chorus) = (
        "let x = true\n",
        "Track verse\n",
        "Clip chorus:\n    Note as hook\n",
    );
    let plain = "Clip verse:\n    Note\n";
    let unknown = |text| (Code::UnknownAttribute, text);
    let rows: &[support::Row] = &[
        (
            &[
                flag,
                track,
                chorus,
                "pattern Unused():\n    Plays | gain=to.label: (if x then verse else chorus)\n",
            ],
            &[],
        ),
        (
            &[
                flag,
                track,
                chorus,
                "pattern Unused():\n    Plays | gain=to.zzz: (if x then verse else chorus)\n",
            ],
            &[(Code::UnknownAttribute, "to.zzz")],
        ),
        (
            &[
                flag,
                track,
                chorus,
                "pattern Unused():\n    Plays | gain=to.label: [verse, chorus][0]\n",
            ],
            &[],
        ),
        (
            &[
                flag,
                track,
                chorus,
                "pattern Unused():\n    Plays | gain=to.zzz: [verse, chorus][0]\n",
            ],
            &[(Code::UnknownAttribute, "to.zzz")],
        ),
        (
            &[
                flag,
                track,
                chorus,
                "pattern Unused():\n    Plays | gain=to.label: (if x then chorus else [chorus, verse][0])\n",
            ],
            &[],
        ),
        (
            &[
                flag,
                track,
                chorus,
                "pattern Unused():\n    Plays | gain=to.hook.zzz: (if x then chorus else [chorus, verse][0])\n",
            ],
            &[(Code::UnknownAttribute, "to.hook.zzz")],
        ),
        (
            &[
                flag,
                track,
                chorus,
                "pattern Unused():\n    Plays | gain=to.hook.velocity: (if x then verse else chorus)\n",
            ],
            &[],
        ),
        (
            &[
                flag,
                track,
                chorus,
                "pattern Unused():\n    Plays | gain=to.hook: (if x then verse else chorus)\n",
            ],
            &[(Code::UnknownAttribute, "to.hook")],
        ),
        (
            &[
                flag,
                plain,
                chorus,
                "pattern Unused():\n    Plays | gain=to.hook: (if x then chorus else verse)\n",
            ],
            &[(Code::UnknownAttribute, "to.hook")],
        ),
        (
            &[
                flag,
                plain,
                chorus,
                "pattern Unused():\n    Plays | gain=to.hook: [chorus, verse][0]\n",
            ],
            &[(Code::UnknownAttribute, "to.hook")],
        ),
        (
            &[
                flag,
                plain,
                chorus,
                "pattern Unused():\n    Plays | gain=to.hook: (if x then chorus else chorus)\n",
            ],
            &[(Code::MisplacedTo, "hook")],
        ),
        (
            &[
                flag,
                track,
                chorus,
                "Clip bridge:\n    Note as hook\n",
                "pattern Unused():\n    Plays | gain=to.hook: (if x then bridge else chorus)\n",
            ],
            &[(Code::MisplacedTo, "hook")],
        ),
        (
            &[
                "let ds = [verse, chorus]\n",
                "Track verse\n",
                "Clip chorus\n",
                "pattern Unused():\n    Plays | gain=to.zzz: ds[0]\n",
            ],
            &[(Code::UnknownAttribute, "to.zzz")],
        ),
        (
            &[
                "Clip verse:\n    Note as hook\n    for i in 0..0:\n        Plays | gain=to.yyy: verse\n        Plays | gain=to.hook.www: verse\n",
                "pattern Unused():\n    Plays | gain=to.zzz: verse\n    Plays | gain=to.nochild.velocity: verse\n",
            ],
            &[
                unknown("yyy"),
                unknown("www"),
                unknown("zzz"),
                unknown("nochild"),
            ],
        ),
        (
            &[verse, "pattern Unused():\n    Plays | gain=to.zzz: verse\n"],
            &[unknown("zzz")],
        ),
        (
            &[
                verse,
                "pattern Unused():\n    Plays | gain=to.nochild.velocity: verse\n",
            ],
            &[unknown("nochild")],
        ),
        (
            &[
                "Clip verse:\n    Note as hook\n    for i in 0..0:\n        Plays | gain=to.yyy: verse\n",
            ],
            &[unknown("yyy")],
        ),
        (
            &[
                "Clip verse:\n    Note as hook\n    for i in 0..0:\n        Plays | gain=to.hook.www: verse\n",
            ],
            &[unknown("www")],
        ),
        (
            &[
                verse,
                "pattern Unused():\n    Plays when to.www > 0 | gain=0.5: verse\n",
            ],
            &[unknown("www")],
        ),
        (
            &[verse, "pattern Unused():\n    Plays /(to.www): verse\n"],
            &[unknown("www")],
        ),
        (
            &[
                verse,
                "pattern Unused():\n    Plays | gain=to.zzz: verse.hook\n",
            ],
            &[unknown("zzz")],
        ),
        (
            &[
                verse,
                "pattern Unused():\n    Plays | gain=to.hook: verse\n",
            ],
            &[(Code::MisplacedTo, "hook")],
        ),
        (
            &[
                verse,
                "pattern Unused():\n    Plays | gain=to.hook.velocity: verse\n",
            ],
            &[],
        ),
        (
            &[
                verse,
                "pattern P(target):\n    Plays | gain=to.gain: target\n",
            ],
            &[],
        ),
        (
            &[
                verse,
                "Clip c:\n    Plays | gain=to.zzz: verse\n    Plays | gain=to.nochild.velocity: verse\n    Plays | gain=to.hook.www: verse\n    Plays | gain=to.hook: verse\n    Plays | gain=to.hook.velocity: verse\n",
            ],
            &[
                unknown("zzz"),
                unknown("nochild"),
                unknown("www"),
                (Code::MisplacedTo, "hook"),
            ],
        ),
    ];
    support::assert_rows_in_every_order(&vocabulary(), rows);
    let compiler = Compiler::new(&vocabulary()).expect("the vocabulary compiles");
    for choice in ["(if x then chorus else verse)", "[chorus, verse][0]"] {
        let text =
            format!("{flag}{plain}{chorus}pattern Unused():\n    Plays | gain=to.hook: {choice}\n");
        let found = support::errors(compiler.check(&[support::source(&text)]));
        let labels: Vec<(&str, &str)> = found
            .iter()
            .flat_map(|diagnostic| &diagnostic.related)
            .map(|related| {
                let range = &related.span.range;
                (
                    &text[range.start as usize..range.end as usize],
                    related.label.as_str(),
                )
            })
            .collect();
        assert_eq!(
            labels,
            [("Clip verse", "the `Clip` named `verse` has no `to.hook`")],
            "{text}"
        );
    }
}

#[test]
fn code_that_nothing_expands_or_computes_draws_only_its_name_resolution_errors() {
    let clip = "Clip c:\n    Note\n";
    let value = (Code::NotAValue, "sin");
    let attribute = (Code::UnknownAttribute, "nothing");
    let name = (Code::UnknownName, "nothing");
    let rows: &[support::Row] = &[
        (&["let u = sin\n"], &[value]),
        (&["let u = c.nothing\n", clip], &[attribute]),
        (&["let u = nothing\n"], &[name]),
        (&["let u = 1 div 0.5\n"], &[]),
        (&["let u = [1, 2][5]\n"], &[]),
        (&["fn f(x): sin + x\n", "let u = f(1)\n"], &[value]),
        (
            &["fn f(x): c.nothing\n", "let u = f(1)\n", clip],
            &[attribute],
        ),
        (&["fn f(x): nothing + x\n", "let u = f(1)\n"], &[name]),
        (&["fn f(x): x div 0.5\n", "let u = f(1)\n"], &[]),
        (
            &[
                "fn f(n): if n < 1 then sin else f(n - 1)\n",
                "let u = f(1)\n",
            ],
            &[value],
        ),
        (
            &[
                "fn g(n): if n < 1 then c.nothing else g(n - 1)\n",
                "let u = g(1)\n",
                clip,
            ],
            &[attribute],
        ),
        (
            &[
                "fn g(n): if n < 1 then nothing else g(n - 1)\n",
                "let u = g(1)\n",
            ],
            &[name],
        ),
        (
            &[
                "fn g(n): if n < 1 then 1 div 0.5 else g(n - 1)\n",
                "let u = g(1)\n",
            ],
            &[],
        ),
        (
            &[
                "fn g(n): if n < 1 then [1, 2][5] else g(n - 1)\n",
                "let u = g(1)\n",
            ],
            &[],
        ),
        (&["fn f(x): sin + x\n"], &[value]),
        (&["fn f(x): c.nothing\n", clip], &[attribute]),
        (&["fn f(x): nothing\n"], &[name]),
        (&["fn f(x): x div 0.5\n"], &[]),
        (&["pattern P():\n    Note/(sin)\n"], &[value]),
        (
            &["pattern P():\n    Note | velocity=c.nothing\n", clip],
            &[attribute],
        ),
        (&["pattern P():\n    Note/(nothing)\n"], &[name]),
        (&["pattern P():\n    Note/(1 div 0.5)\n"], &[]),
        (
            &["let u = 1 div 0.5\n", "Clip c:\n    Note/u\n"],
            &[(Code::OperandType, "1 div 0.5")],
        ),
        (
            &["let u = [1, 2][5]\n", "Clip c:\n    Note/u\n"],
            &[(Code::IndexOutOfRange, "5")],
        ),
        (
            &[
                "fn f(x): x div 0.5\n",
                "let u = f(1)\n",
                "Clip c:\n    Note/u\n",
            ],
            &[(Code::OperandType, "x div 0.5")],
        ),
        (
            &[
                "fn g(n): if n < 1 then 1 div 0.5 else g(n - 1)\n",
                "let u = g(1)\n",
                "Clip c:\n    Note/u\n",
            ],
            &[(Code::OperandType, "1 div 0.5")],
        ),
        (
            &[
                "fn g(n): if n < 1 then [1, 2][5] else g(n - 1)\n",
                "let u = g(1)\n",
                "Clip c:\n    Note/u\n",
            ],
            &[(Code::IndexOutOfRange, "5")],
        ),
    ];
    support::assert_rows_in_every_order(&vocabulary(), rows);
    let input = "input k = 1\n";
    let loud = (Code::UnknownAttribute, "loud");
    let variant = (Code::UnknownVariant, "Nope");
    let destination = (Code::DestinationType, "k");
    let parameter = (Code::UnknownParameter, "nope");
    let calling = |call: &str| format!("Clip c:\n    Note/({call})\n");
    let (untaken, taken) = (calling("f(1)"), calling("f()"));
    let looped =
        |range: &str, line: &str| format!("Clip c:\n    for i in {range}:\n        {line}\n");
    let synth =
        |range: &str| format!("Synth s:\n    for i in {range}:\n        Osc | wave=Wave::Nope\n");
    let wired = |range: &str| format!("Clip c:\n    for i in {range}:\n        Plays: k\n");
    let pattern_loop = |range: &str| {
        format!("pattern Q():\n    for i in {range}:\n        Note | velocity=c.nothing\n")
    };
    let defaulted = "pattern P(default r = c.nothing):\n    Note/r\n";
    let loud_pattern = "pattern Q():\n    Note | loud=1\n";
    let use_pattern = "pattern Q():\n    R/60 | nope=1\n";
    let root = "pattern R(default root):\n    Note/root\n";
    let wire_pattern = "pattern Q():\n    Clip:\n        Plays: k\n";
    let mut paths: Vec<OwnedRow> = Vec::new();
    let operand = (Code::OperandType, "1 div 0.5");
    let range_index = (Code::IndexOutOfRange, "5");
    let chosen = |range: &str| {
        format!(
            "input x = true\nClip c:\n    for i in {range}:\n        when x:\n            Note | loud=1\n        else:\n            Note/(sin)\n"
        )
    };
    let chord = |range: &str| format!("Clip c:\n    [Note for i in {range}: (Note | loud=1)]\n");
    let joined = (Code::ArrayOperand, "1 + [1, 2]");
    for (range, reached, unused, sum) in [
        ("0..0", vec![], vec![], vec![]),
        ("0..1", vec![operand], vec![range_index], vec![joined]),
    ] {
        paths.extend([
            (vec![looped(range, "Note/(sin)")], vec![value]),
            (
                vec![looped(range, "Note | velocity=c.nothing")],
                vec![attribute],
            ),
            (vec![synth(range)], vec![variant]),
            (vec![looped(range, "Note | loud=1")], vec![loud]),
            (vec![looped(range, "Note/(i + nothing)")], vec![name]),
            (vec![input.to_string(), wired(range)], vec![destination]),
            (
                vec![pattern_loop(range), "Clip c:\n    Q\n".to_string()],
                vec![attribute],
            ),
            (vec![looped(range, "Note/(1 div 0.5)")], reached),
            (vec![looped(range, "Note/(i)")], vec![]),
            (vec![chosen(range)], vec![loud, value]),
            (vec![chord(range)], vec![loud]),
            (vec![looped(range, "Note when c.nothing")], vec![attribute]),
            (
                vec![root.to_string(), looped(range, "R/(sin)")],
                vec![value],
            ),
            (
                vec![looped(range, "for j in 0..2:\n            Note | loud=1")],
                vec![loud],
            ),
            (
                vec![looped(range, "Plays: 3")],
                vec![(Code::DestinationType, "3")],
            ),
            (
                vec!["let u = [1, 2][5]\n".to_string(), looped(range, "Note/(u)")],
                unused,
            ),
            (
                vec![
                    "let u = 1 + [1, 2]\n".to_string(),
                    looped(range, "Note/(u)"),
                ],
                sum,
            ),
        ]);
    }
    for (call, reached) in [(&untaken, vec![]), (&taken, vec![operand])] {
        paths.extend([
            (
                vec!["fn f(x = sin): x\n".to_string(), call.clone()],
                vec![value],
            ),
            (
                vec!["fn f(x = c.nothing): x\n".to_string(), call.clone()],
                vec![attribute],
            ),
            (
                vec!["fn f(x = 1 div 0.5): x\n".to_string(), call.clone()],
                reached,
            ),
        ]);
    }
    for placed in ["P/60", "P"] {
        paths.push((
            vec![defaulted.to_string(), format!("Clip c:\n    {placed}\n")],
            vec![attribute],
        ));
    }
    let fit = (Code::DoesNotFit, "2147483647 + 1");
    for (used, reached) in [(None, vec![]), (Some("Clip c:\n    Q\n"), vec![fit])] {
        let mut with = |items: &[&str], expected: Vec<(Code, &'static str)>| {
            let mut items: Vec<String> = items.iter().map(|item| item.to_string()).collect();
            items.extend(used.map(str::to_string));
            paths.push((items, expected));
        };
        with(&[loud_pattern], vec![loud]);
        with(&[use_pattern, root], vec![parameter]);
        with(&["pattern Q():\n    Note/(2147483647 + 1)\n"], reached);
    }
    paths.extend([
        (
            vec![
                "Clip n:\n    Note\n".to_string(),
                "pattern Q():\n    Note as n | velocity=1\n    Note/(n.velocity)\n".to_string(),
            ],
            vec![],
        ),
        (
            vec![input.to_string(), wire_pattern.to_string()],
            vec![destination],
        ),
        (
            vec![
                input.to_string(),
                wire_pattern.to_string(),
                "Q q\n".to_string(),
            ],
            vec![destination],
        ),
    ]);
    let items: Vec<BorrowedRow> = paths
        .iter()
        .map(|(items, expected)| {
            (
                items.iter().map(String::as_str).collect(),
                expected.as_slice(),
            )
        })
        .collect();
    let rows: Vec<support::Row> = items
        .iter()
        .map(|(items, expected)| (items.as_slice(), *expected))
        .collect();
    support::assert_rows_in_every_order(&vocabulary(), &rows);
}
