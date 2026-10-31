mod support;

use symphonyscript::Code;
use symphonyscript::syntax::SyntaxKind;
use symphonyscript::syntax::ast::{AstNode, CellDecl, Item};
use symphonyscript_fixtures::{Rows, covered};

const WORDS: [(&str, SyntaxKind); 3] = [
    ("state", SyntaxKind::StateKw),
    ("stream", SyntaxKind::StreamKw),
    ("channel", SyntaxKind::ChannelKw),
];

const CLOCKS: [&str; 3] = ["", " on gate", " on (gate, bar)"];

const RULES: [(&str, Option<SyntaxKind>); 4] = [
    ("\n", None),
    (" = level + 1\n", Some(SyntaxKind::BinaryExpr)),
    (
        " = match level:\n    0: 1\n    else: 2\n",
        Some(SyntaxKind::MatchBlock),
    ),
    (
        " = for x in [1, 2] into s from 0:\n    s + x\n",
        Some(SyntaxKind::ForExprBlock),
    ),
];

fn cell_text(
    word: &str,
    (capacity, init, typed): (bool, bool, bool),
    clock: &str,
    rule: &str,
) -> String {
    format!(
        "{word}{} x{}{}{clock}{rule}",
        if capacity { "[16]" } else { "" },
        if init { "(0)" } else { "" },
        if typed { ": i32" } else { "" },
    )
}

fn parts_problem(
    cell: &CellDecl<'_>,
    (word, capacity, init, typed): (SyntaxKind, bool, bool, bool),
    clock: usize,
    rule: Option<SyntaxKind>,
) -> Option<String> {
    let text = |node: Option<String>| node.unwrap_or_default();
    let found_rule = cell
        .value()
        .map(|value| value.syntax().kind())
        .or_else(|| cell.match_block().map(|block| block.syntax().kind()));
    let found = (
        cell.word().map(|token| token.kind()),
        cell.has_capacity(),
        text(cell.capacity().map(|token| token.text().to_string())),
        text(cell.name().map(|name| name.text())),
        text(
            cell.init()
                .and_then(|init| init.value())
                .map(|value| value.syntax().to_string()),
        ),
        cell.ty().is_some(),
        cell.clock()
            .map(|clock| (clock.is_list(), clock.entries().count())),
        (cell.has_rule(), found_rule),
    );
    let expected = (
        Some(word),
        capacity,
        if capacity { "16" } else { "" }.to_string(),
        "x".to_string(),
        if init { "0" } else { "" }.to_string(),
        typed,
        match clock {
            0 => None,
            1 => Some((false, 1)),
            _ => Some((true, 2)),
        },
        (rule.is_some(), rule),
    );
    (found != expected).then(|| format!("found {found:?}, expected {expected:?}"))
}

fn cell_rows(rows: &Rows) -> (Vec<String>, usize) {
    let mut failures = Vec::new();
    let mut programs = 0;
    for (w, (word, kind)) in WORDS.iter().enumerate() {
        for capacity in [false, true] {
            for init in [false, true] {
                for typed in [false, true] {
                    for (c, clock) in CLOCKS.iter().enumerate() {
                        for (r, (rule, rule_kind)) in RULES.iter().enumerate() {
                            let tag = [
                                w,
                                usize::from(capacity),
                                usize::from(init),
                                usize::from(typed),
                                c,
                                r,
                            ];
                            if !rows.take(&tag) {
                                continue;
                            }
                            programs += 1;
                            let text = cell_text(word, (capacity, init, typed), clock, rule);
                            let parsed = support::parsed(&text);
                            if parsed.root().to_string() != text {
                                failures.push(format!("{text:?} does not round-trip"));
                                continue;
                            }
                            let end = text.find('\n').unwrap_or(text.len()) as u32;
                            let fault = match (rule_kind, c, init) {
                                (None, 1.., _) => Some("a clock stands on a derived cell"),
                                (None, 0, false) => {
                                    Some("an init in parentheses, a rule after `=`")
                                }
                                _ => None,
                            };
                            let drawn: Vec<(Code, u32, u32)> = parsed
                                .diagnostics
                                .iter()
                                .map(|diagnostic| {
                                    (
                                        diagnostic.code,
                                        diagnostic.span.range.start,
                                        diagnostic.span.range.end,
                                    )
                                })
                                .collect();
                            if let Some(summary) = fault {
                                let named = parsed.diagnostics.first().is_some_and(|diagnostic| {
                                    diagnostic.summary.contains(summary)
                                        && diagnostic
                                            .help
                                            .as_ref()
                                            .is_some_and(|help| help.contains(&format!("`{word} ")))
                                });
                                if drawn != [(Code::CellSyntax, end, end + 1)] || !named {
                                    failures
                                        .push(format!("{text:?} draws {:?}", parsed.diagnostics));
                                }
                                continue;
                            }
                            if !drawn.is_empty() {
                                failures.push(format!("{text:?} draws {drawn:?}"));
                                continue;
                            }
                            let file = parsed.file().expect("the root is a file");
                            let items: Vec<Item<'_>> = file.items().collect();
                            let [Item::Cell(cell)] = items.as_slice() else {
                                failures.push(format!("{text:?} is not one cell declaration"));
                                continue;
                            };
                            failures.extend(
                                parts_problem(cell, (*kind, capacity, init, typed), c, *rule_kind)
                                    .map(|problem| format!("{text:?}: {problem}")),
                            );
                        }
                    }
                }
            }
        }
    }
    (failures, programs)
}

fn assert_none_fail((failures, programs): (Vec<String>, usize), expected: usize) {
    assert!(
        failures.is_empty(),
        "{} of {programs} programs fail:\n{}",
        failures.len(),
        failures.join("\n")
    );
    assert_eq!(programs, expected);
}

#[test]
fn a_cell_declaration_parses_every_combination_of_its_parts_and_one_with_neither_init_nor_rule_names_both()
 {
    assert_none_fail(covered(cell_rows), 14);
}

#[test]
#[ignore = "exhaustive"]
fn exhaustive_a_cell_declaration_parses_every_combination_of_its_parts_and_one_with_neither_init_nor_rule_names_both()
 {
    assert_none_fail(cell_rows(&Rows::Every), 288);
}

#[test]
fn a_line_of_any_cell_word_ends_an_unclosed_bracket_and_starts_an_item() {
    for word in ["state", "stream", "channel"] {
        for (text, at) in [
            (format!("let a = (1 +\n{word} s(1)\nlet b = 2\n"), word),
            (format!("let a = [1,\n2\n{word} s = a\n"), word),
            (format!("{word} s(f(1,\nlet b = 2\n"), "let"),
        ] {
            let (start, end) = support::at(&text, at, 0);
            assert_eq!(
                support::codes(&text),
                vec![(Code::ExpressionSyntax, start, end)],
                "{text}"
            );
            let tree = support::tree(&text);
            let item = if at == "let" {
                "LetDecl[let b".to_string()
            } else {
                format!("CellDecl[{word} s")
            };
            assert!(tree.contains(&item), "{text}\n{tree}");
        }
    }
}

const READS: [(&str, &str); 3] = [
    (
        "x.past(1)",
        "CallExpr[FieldExpr[NameRef[x] . past] ArgList[( Literal[1] )]]",
    ),
    (
        "x.past(2, 0)",
        "CallExpr[FieldExpr[NameRef[x] . past] ArgList[( Literal[2] , Literal[0] )]]",
    ),
    ("x.stepped", "FieldExpr[NameRef[x] . stepped]"),
];

const POSITIONS: [&str; 7] = [
    "state s = {}\n",
    "Clip c:\n    Note | velocity={}\n",
    "Clip c:\n    Note when {}\n",
    "let a = f({}, 1)\n",
    "let a = 1 + {}\n",
    "let a = {}\n",
    "fn f(y): {}\n",
];

#[test]
fn a_history_read_and_stepped_parse_wherever_a_read_of_the_cell_parses() {
    for (read, tree) in READS {
        for position in POSITIONS {
            let text = position.replacen("{}", read, 1);
            let parsed = support::tree(&text);
            assert_eq!(support::codes(&text), vec![], "{text}");
            assert!(parsed.contains(tree), "{text}\n{parsed}");
        }
    }
}

#[test]
fn a_keyword_stands_as_a_member_name_after_a_dot_and_after_the_bar_of_an_attribute_clause() {
    for (text, tree) in [
        (
            "let a = x.state + x.channel.out\n",
            "BinaryExpr[FieldExpr[NameRef[x] . state] + FieldExpr[FieldExpr[NameRef[x] . channel] . out]]",
        ),
        (
            "Clip c:\n    Note | channel=1 out=x.on !cell\n",
            "Attrs[| AttrEntry[channel = Literal[1]] AttrEntry[out = FieldExpr[NameRef[x] . on]] AttrEntry[! cell]]",
        ),
        ("Clip c:\n    Note | stream\n", "Attrs[| AttrEntry[stream]]"),
        (
            "Clip c:\n    Note/x.on\n",
            "DefaultArg[/ FieldExpr[NameRef[x] . on]]",
        ),
    ] {
        assert_eq!(support::codes(text), vec![], "{text}");
        let parsed = support::tree(text);
        assert!(parsed.contains(tree), "{text}\n{parsed}");
    }
    let text = "Clip c:\n    Note | velocity=1 when x\n";
    let (start, end) = support::at(text, "when", 0);
    assert_eq!(
        support::codes(text),
        vec![(Code::PlacementSyntax, start, end)],
        "{text}"
    );
    for word in ["stream", "channel", "cell", "out", "on"] {
        let text = format!("let {word} = 1\n");
        let (start, end) = support::at(&text, word, 0);
        assert_eq!(
            support::codes(&text),
            vec![(Code::LetSyntax, start, end)],
            "{text}"
        );
    }
}

const HELPERS: &str = "state n(2)\nstate h([]): i32[..4]\n";

fn cell_line(word: &str, derived: bool, shape: usize) -> String {
    let (init, rule) = match shape {
        0 => ("(3)", "n + 1"),
        1 => ("([1, 2])", "(if n > 1 then [1, 2] else [3, 4])"),
        _ => ("([]): i32[..4]", "h[^2..]"),
    };
    if derived {
        format!("{word} x = {rule}\n")
    } else {
        format!("{word} x{init}\n")
    }
}

fn read_text(read: usize, shape: usize) -> String {
    let fallback = ["0", "[0, 0]", "[]"][shape];
    match read {
        0 => "x".to_string(),
        1 => "x.past(1)".to_string(),
        2 => format!("x.past(2, {fallback})"),
        _ => "x.stepped".to_string(),
    }
}

fn use_site(position: usize, value: &str) -> String {
    match position {
        0 => format!("state r = {value}\nClip c:\n    Note/(r)\n"),
        1 => format!("Clip c:\n    Note/({value})\n"),
        _ => format!("Clip c:\n    Note when {value}\n"),
    }
}

fn read_rows(rows: &Rows) -> (Vec<String>, usize) {
    let compiler = symphonyscript::Compiler::new(&symphonyscript_fixtures::vocabulary())
        .expect("the vocabulary compiles");
    let mut failures = Vec::new();
    let mut programs = 0;
    for (w, (word, _)) in WORDS.iter().enumerate() {
        for derived in [false, true] {
            for shape in 0..3 {
                for read in 0..4 {
                    for position in 0..3 {
                        if !rows.take(&[w, usize::from(derived), shape, read, position]) {
                            continue;
                        }
                        programs += 1;
                        let read_text = read_text(read, shape);
                        let stepped = read == 3;
                        let value = if stepped || shape == 0 {
                            read_text.clone()
                        } else {
                            format!("(for e in {read_text} into s from 0: s + e)")
                        };
                        let site = use_site(position, &value);
                        let text = format!("{HELPERS}{}{site}", cell_line(word, derived, shape));
                        let inner = value
                            .strip_prefix('(')
                            .and_then(|value| value.strip_suffix(')'))
                            .unwrap_or(&value)
                            .to_string();
                        let expected: Vec<(Code, String)> = match (position, stepped) {
                            (0, true) => vec![(Code::TypeMismatch, "r".to_string())],
                            (1, true) => vec![(Code::TypeMismatch, inner)],
                            (2, false) => vec![(Code::ConditionType, inner)],
                            _ => Vec::new(),
                        };
                        let found = support::error_texts(&compiler, &text);
                        let found: Vec<(Code, String)> = found
                            .into_iter()
                            .filter(|(code, _)| *code != Code::NoEntryInstance)
                            .collect();
                        if found != expected {
                            failures
                                .push(format!("{text}\n  draws {found:?}, expected {expected:?}"));
                        }
                    }
                }
            }
        }
    }
    (failures, programs)
}

#[test]
fn a_cell_of_every_word_and_form_is_read_in_the_present_in_its_past_and_by_stepped_wherever_a_cell_is_read()
 {
    assert_none_fail(covered(read_rows), 15);
}

#[test]
#[ignore = "exhaustive"]
fn exhaustive_a_cell_of_every_word_and_form_is_read_in_the_present_in_its_past_and_by_stepped_wherever_a_cell_is_read()
 {
    assert_none_fail(read_rows(&Rows::Every), 216);
}

#[test]
fn a_derived_cells_init_counts_for_its_own_past_and_fits_the_type_its_rule_gives() {
    let reader = "Clip c:\n    Note/(s)\n";
    let rows: &[support::Row] = &[
        (&["state s(0) = s.past(1) + 1\n", reader], &[]),
        (&["state s(0) = s.past(1, 5) + 1\n", reader], &[]),
        (&["state s = s.past(1, 0) + 1\n", reader], &[]),
        (
            &["state s(0.5) = s.past(1) + 1\n", reader],
            &[(Code::FloatToInteger, "s")],
        ),
        (
            &["state s = s.past(1) + 1\n", reader],
            &[(Code::HistoryFallback, "s.past(1)")],
        ),
        (
            &["state n(2)\n", "state s(true) = n + 1\n", reader],
            &[(Code::TypeMismatch, "true")],
        ),
        (
            &["state n(2)\n", "state s(300): u8 = n\n", reader],
            &[(Code::DoesNotFit, "300")],
        ),
        (
            &[
                "state n(2)\n",
                "state s([1, 2]) = (if n > 1 then [1, 2, 3] else [4, 5, 6])\n",
                "Clip c:\n    Note/(s[0])\n",
            ],
            &[(Code::TypeMismatch, "[1, 2]")],
        ),
        (
            &["state n(2)\n", "state s(n) = n + 1\n", reader],
            &[(Code::ReactiveInitializer, "n")],
        ),
        (
            &["Clip c:\n    Note\n", "state s(c) = 1\n"],
            &[(Code::StateValue, "c")],
        ),
        (
            &[
                "state a(0) = b.past(1) + 1\n",
                "state b = a.past(1) + 1\n",
                "Clip c:\n    Note/(a + b)\n",
            ],
            &[],
        ),
        (
            &[
                "state a = b.past(1) + 1\n",
                "state b(0) = a.past(1) + 1\n",
                "Clip c:\n    Note/(a + b)\n",
            ],
            &[],
        ),
        (
            &[
                "state t(0) = r.past(1)\n",
                "state r = t + 1\n",
                "Clip c:\n    Note/(t)\n",
            ],
            &[],
        ),
        (
            &[
                "state t(0.5) = r.past(1)\n",
                "state r = t + 1\n",
                "Clip c:\n    Note/(r)\n",
            ],
            &[(Code::TypeMismatch, "0.5")],
        ),
        (
            &[
                "state t(0) = r.past(1)\n",
                "state r(0.5) = t + 1\n",
                "Clip c:\n    Note | brightness=r\n",
            ],
            &[],
        ),
        (
            &[
                "state n(1)\n",
                "state t = r.past(1)\n",
                "state r([1, 2]) = [t[1] + n, t[0]]\n",
                "Clip c:\n    Note/(t[0] + r[1])\n",
            ],
            &[],
        ),
        (
            &[
                "state note(60)\n",
                "state t = r.past(1)\n",
                "state r([]) = (t + [note])[^4..]\n",
                "Clip c:\n    Note/(note)\n",
            ],
            &[],
        ),
        (
            &[
                "state n(1)\n",
                "state t = r.past(1)\n",
                "state r = [n, n]\n",
                "Clip c:\n    Note/(t[0])\n",
            ],
            &[],
        ),
    ];
    support::assert_rows_in_every_order(&symphonyscript_fixtures::vocabulary(), rows);
    let compiler = symphonyscript::Compiler::new(&symphonyscript_fixtures::vocabulary())
        .expect("the vocabulary compiles");
    let cycles: &[&[(&str, &str)]] = &[
        &[
            ("state a = b.past(1) + 1\n", "b.past(1)"),
            ("state b = a.past(1) + 1\n", "a.past(1)"),
        ],
        &[
            ("state a = b + 1\n", ""),
            ("state b = a.past(1)\n", "a.past(1)"),
        ],
        &[
            ("state d = a.past(1) + 1\n", "a.past(1)"),
            ("state a = b + 1\n", ""),
            ("state b = d.past(1)\n", "d.past(1)"),
        ],
    ];
    for cycle in cycles {
        let mut items: Vec<(&str, &str)> = cycle.to_vec();
        items.push(("Clip c:\n    Note/(a + b)\n", ""));
        for order in support::permutations(&items) {
            let text: String = order.iter().map(|(item, _)| *item).collect();
            let earliest = order
                .iter()
                .map(|(_, read)| *read)
                .find(|read| !read.is_empty())
                .unwrap_or_default();
            assert_eq!(
                support::error_texts(&compiler, &text),
                vec![(Code::HistoryFallback, earliest.to_string())],
                "{text}"
            );
        }
    }
}

#[test]
fn a_cells_capacity_clock_string_and_history_target_draw_the_codes_their_rules_name() {
    let rows: &[support::Row] = &[
        (&["state[4] x(0)\n"], &[(Code::CellCapacity, "[4]")]),
        (&["stream[4] x(0)\n"], &[]),
        (&["channel[16] x(0)\n"], &[]),
        (
            &["state n(1)\n", "stream[4] x = n\n"],
            &[(Code::CellCapacity, "[4]")],
        ),
        (
            &["state n(1)\n", "channel[4] x = n\n"],
            &[(Code::CellCapacity, "[4]")],
        ),
        (
            &[
                "state a(1)\n",
                "stream b(2)\n",
                "state c2 on b = a + b\n",
                "Clip c:\n    Note/(c2)\n",
            ],
            &[(Code::UnbuiltConstruct, "on b")],
        ),
        (
            &[
                "state a(1)\n",
                "stream b(2)\n",
                "state c4 on (a, b) = a + b\n",
                "Clip c:\n    Note/(c4)\n",
            ],
            &[(Code::UnbuiltConstruct, "on (a, b)")],
        ),
        (&["state s(\"a\")\n"], &[(Code::UnbuiltConstruct, "\"a\"")]),
        (
            &["state n(2)\n", "state s(\"a\") = n\n"],
            &[(Code::UnbuiltConstruct, "\"a\"")],
        ),
        (&["state s = \"a\"\n"], &[(Code::UnbuiltConstruct, "\"a\"")]),
        (
            &["fn f(p): p.past(1)\n", "Clip c:\n    Note/(f(1))\n"],
            &[(Code::HistoryTarget, "p")],
        ),
        (
            &["fn f(p): p.stepped\n", "Clip c:\n    Note when f(1)\n"],
            &[(Code::HistoryTarget, "p")],
        ),
        (
            &["let a = 1\n", "Clip c:\n    Note/(a.past(1))\n"],
            &[(Code::HistoryTarget, "a.past(1)")],
        ),
        (
            &["state n(1)\n", "Clip c:\n    Note/((n + 1).past(1))\n"],
            &[(Code::HistoryTarget, "(n + 1)")],
        ),
        (
            &["state n(1)\n", "Clip c:\n    Note/(n.past(1, 2, 3))\n"],
            &[(Code::ArgumentCount, "n.past(1, 2, 3)")],
        ),
        (
            &[
                "state n(1)\n",
                "fn f(): n.past(2, 0)\n",
                "let a = n.past(1)\n",
                "Clip c:\n    Note/(f() + a)\n",
            ],
            &[],
        ),
    ];
    support::assert_rows_in_every_order(&symphonyscript_fixtures::vocabulary(), rows);
}

#[test]
fn a_line_written_as_an_input_draws_the_top_level_diagnostic_whose_help_names_the_cell_form() {
    for text in [
        "input level = 1\nClip c:\n    Note\n",
        "input fine: f32 = 0\nClip c:\n    Note\n",
    ] {
        let parsed = support::parsed(text);
        let (start, end) = support::at(text, "input", 0);
        let found: Vec<(Code, u32, u32, Option<&str>)> = parsed
            .diagnostics
            .iter()
            .map(|diagnostic| {
                (
                    diagnostic.code,
                    diagnostic.span.range.start,
                    diagnostic.span.range.end,
                    diagnostic.help.as_deref(),
                )
            })
            .collect();
        assert_eq!(
            found,
            vec![(
                Code::TopLevelSyntax,
                start,
                end,
                Some("write `state <name>(<init>)`")
            )],
            "{text}"
        );
    }
    let text = "state input(1)\nClip c:\n    Note/(input + input.past(1))\n";
    assert_eq!(support::check_codes(text), vec![], "{text}");
}
