mod support;

use symphonyscript::Code;

type Case = (&'static str, &'static [(Code, &'static str, usize)]);

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
