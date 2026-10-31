mod support;

use symphonyscript::Code;
use symphonyscript_fixtures::{Rows, covered};

const PRELUDE: &str = "state q(1)\nlet xs = [1, 2]\nfn f(a): a\n";

const POSITIONS: [&str; 9] = [
    "let v = {}\n",
    "let v = 1 + ({}) * 2\n",
    "let v = f({})\n",
    "Clip c:\n    Note | velocity={}\n",
    "Clip c:\n    Note when {}\n",
    "state v({})\n",
    "state v = {}\n",
    "let v = xs[{}]\n",
    "let v = \"a{{}}\"\n",
];

const FORMS: [(&str, Code, &str); 11] = [
    ("{\"a\": 1}", Code::MapSyntax, "{\"a\" 1}"),
    ("{}", Code::MapSyntax, "{,}"),
    ("Chord {root: 1}", Code::RecordSyntax, "Chord {root 1}"),
    ("Chord {}", Code::RecordSyntax, "Chord {,}"),
    ("json {\"a\": [1, null]}", Code::JsonSyntax, "json {a: [1]}"),
    ("json -2.5", Code::JsonSyntax, "json 0x2"),
    ("none", Code::ExpressionSyntax, "(none"),
    ("q ?? 1", Code::ExpressionSyntax, "q ?? *"),
    ("q without 1", Code::ExpressionSyntax, "q without *"),
    ("q is i32[2]", Code::TypeSyntax, "q is i32[,]"),
    ("q is not str", Code::TypeSyntax, "q is not str[..]"),
];

fn form_rows(rows: &Rows) -> Vec<String> {
    let mut failures = Vec::new();
    for (form_at, (form, code, faulty)) in FORMS.iter().enumerate() {
        for (position_at, position) in POSITIONS.iter().enumerate() {
            for fault in [false, true] {
                if !rows.take(&[form_at, position_at, usize::from(fault)]) {
                    continue;
                }
                let value = if fault { faulty } else { form };
                let text = format!("{PRELUDE}{}", position.replacen("{}", value, 1));
                let found = support::check_codes(&text);
                let codes: Vec<Code> = found.iter().map(|(code, ..)| *code).collect();
                let expected = if fault {
                    vec![*code]
                } else {
                    vec![Code::UnbuiltConstruct]
                };
                if codes != expected {
                    failures.push(format!("{text:?} draws {found:?}, expected {expected:?}"));
                    continue;
                }
                if !fault {
                    let (start, end) = support::at(&text, form, 0);
                    if found.iter().any(|(_, from, to)| *from < start || *to > end) {
                        failures.push(format!("{text:?} draws {found:?} outside `{form}`"));
                    }
                }
            }
        }
    }
    failures
}

#[test]
fn every_new_expression_form_parses_in_every_position_and_draws_one_unbuilt_construct_or_its_fault()
{
    let failures = covered(form_rows);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
#[ignore = "exhaustive"]
fn exhaustive_every_new_expression_form_parses_in_every_position_and_draws_one_unbuilt_construct_or_its_fault()
 {
    let failures = form_rows(&Rows::Every);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn the_fallback_binds_looser_than_addition_and_tighter_than_a_comparison() {
    for (text, tree) in [
        (
            "let a = x ?? base + 1\n",
            "BinaryExpr[NameRef[x] ?? BinaryExpr[NameRef[base] + Literal[1]]]",
        ),
        (
            "let a = port ?? 80 > 1024\n",
            "BinaryExpr[BinaryExpr[NameRef[port] ?? Literal[80]] > Literal[1024]]",
        ),
        (
            "let a = a + b ?? c < d\n",
            "BinaryExpr[BinaryExpr[BinaryExpr[NameRef[a] + NameRef[b]] ?? NameRef[c]] < NameRef[d]]",
        ),
        (
            "let a = x ?? -1\n",
            "BinaryExpr[NameRef[x] ?? PrefixExpr[- Literal[1]]]",
        ),
        (
            "let a = x ?? y ?? z\n",
            "BinaryExpr[NameRef[x] ?? BinaryExpr[NameRef[y] ?? NameRef[z]]]",
        ),
        (
            "let a = (x ?? 0) + 1\n",
            "BinaryExpr[ParenExpr[( BinaryExpr[NameRef[x] ?? Literal[0]] )] + Literal[1]]",
        ),
        (
            "let a = m without k + n\n",
            "BinaryExpr[BinaryExpr[NameRef[m] without NameRef[k]] + NameRef[n]]",
        ),
    ] {
        let parsed = support::clean_tree(text);
        assert!(parsed.contains(tree), "{text:?}\n{parsed}");
    }
}

#[test]
fn the_right_operand_of_is_is_a_fallback_unless_it_is_a_type_only_a_type_can_be() {
    for (text, tree) in [
        (
            "let a = x is none\n",
            "BinaryExpr[NameRef[x] is Literal[none]]",
        ),
        (
            "let a = x is Circle\n",
            "BinaryExpr[NameRef[x] is NameRef[Circle]]",
        ),
        (
            "let a = x is i32[2]\n",
            "BinaryExpr[NameRef[x] is AltType[TypeRef[i32 [ 2 ]]]]",
        ),
        (
            "let a = x is i32(2)\n",
            "BinaryExpr[NameRef[x] is Conversion[i32 ( Literal[2] )]]",
        ),
        (
            "let a = x is not lib::T\n",
            "BinaryExpr[NameRef[x] is not VariantRef[lib :: T]]",
        ),
        (
            "let a = v is json {\"a\": 1}\n",
            "BinaryExpr[NameRef[v] is JsonLit[json JsonObject[{ JsonEntry[\"a\" : JsonAtom[1]] }]]]",
        ),
        (
            "let a = v is json[..8]\n",
            "BinaryExpr[NameRef[v] is AltType[TypeRef[JsonType[json [ .. 8 ]]]]]",
        ),
        (
            "let a = x is xs[0]\n",
            "BinaryExpr[NameRef[x] is IndexExpr[NameRef[xs] [ Bound[Literal[0]] ]]]",
        ),
    ] {
        let parsed = support::clean_tree(text);
        assert!(parsed.contains(tree), "{text:?}\n{parsed}");
    }
}

#[test]
fn a_match_label_is_a_type_a_string_a_negative_number_a_literal_or_a_name() {
    let text = "state s(1)\nlet a = match s:\n    Circle: 1\n    none: 2\n    i32: 3\n    \"x\": 4\n    -1: 5\n    Box[i32]: 6\n    true: 7\n    else: 8\n";
    let parsed = support::clean_tree(text);
    for part in [
        "MatchArm[NameRef[Circle] :",
        "MatchArm[AltType[TypeRef[none]] :",
        "MatchArm[AltType[TypeRef[i32]] :",
        "MatchArm[StringLit[\" x \"] :",
        "MatchArm[PrefixExpr[- Literal[1]] :",
        "MatchArm[AltType[TypeRef[Box TypeArgs[[ TypeRef[i32] ]]]] :",
        "MatchArm[Literal[true] :",
    ] {
        assert!(parsed.contains(part), "{part}\n{parsed}");
    }
    let found = support::check_codes(text);
    assert!(
        found.len() == 4
            && found
                .iter()
                .all(|(code, ..)| *code == Code::UnbuiltConstruct),
        "{found:?}"
    );
}

#[test]
fn a_lambda_stands_as_an_argument_and_as_a_lets_value_and_draws_expression_syntax_elsewhere() {
    for text in [
        "fn count(xs, keep): 1\nlet a = count([1], p: p > 60)\n",
        "fn count(xs, keep): 1\nlet a = count([1], (p: i32) -> bool: p < 48)\n",
        "fn count(xs, keep): 1\nlet a = count(xs=[1], keep=(a, b): a + b)\n",
        "let inc = x: x + 1\n",
        "let inc = (x: f64) -> f64: x + 1\n",
        "fn f(x):\n    let keep = p: p > x\n    keep(1)\n",
    ] {
        assert_eq!(support::codes(text), vec![], "{text:?}");
        let found = support::check_codes(text);
        assert!(
            !found.is_empty()
                && found
                    .iter()
                    .all(|(code, ..)| *code == Code::UnbuiltConstruct),
            "{text:?}: {found:?}"
        );
    }
    for (text, needle) in [
        ("state g(x: x + 1)\n", "x"),
        ("let a = [x: x + 1]\n", "x"),
        ("let a = (x: x + 1)\n", "x"),
        ("let a = \"{p: p}\"\n", "p"),
    ] {
        let (start, end) = support::at(text, needle, 0);
        assert_eq!(
            support::codes(text),
            vec![(Code::ExpressionSyntax, start, end)],
            "{text:?}"
        );
    }
}

#[test]
fn an_operator_keyword_before_an_equals_sign_is_an_attribute_entry() {
    for word in ["without", "is", "and", "or", "div"] {
        let text = format!("Clip c:\n    Note | velocity=1 {word}=2\n");
        let tree = support::tree(&text);
        let entry = format!("AttrEntry[{word} = Literal[2]]");
        assert!(tree.contains(&entry), "{text:?}\n{tree}");
        assert_eq!(support::codes(&text), vec![], "{text:?}");
    }
}

#[test]
fn a_binding_in_an_attribute_clause_parses_its_path_and_draws_one_unbuilt_construct() {
    for (text, entry) in [
        (
            "channel kick(1)\nClip c:\n    Note | velocity => kick\n",
            "AttrEntry[velocity => NameRef[kick]]",
        ),
        (
            "Clip c:\n    Note | velocity => low.gain pitch=60\n",
            "AttrEntry[velocity => FieldExpr[NameRef[low] . gain]] AttrEntry[pitch = Literal[60]]",
        ),
        (
            "Clip c:\n    Note | velocity => drums::kick\n",
            "AttrEntry[velocity => VariantRef[drums :: kick]]",
        ),
    ] {
        let tree = support::clean_tree(text);
        assert!(tree.contains(entry), "{text:?}\n{tree}");
        let found = support::check_codes(text);
        assert!(
            found.len() == 1 && found[0].0 == Code::UnbuiltConstruct,
            "{text:?}: {found:?}"
        );
    }
    let text = "Clip c:\n    Note | velocity =>\n";
    assert_eq!(
        support::check_codes(text)
            .iter()
            .map(|(code, ..)| *code)
            .collect::<Vec<_>>(),
        vec![Code::AttributeSyntax]
    );
}

#[test]
fn a_new_expression_form_holding_or_held_by_a_syntax_fault_draws_the_syntax_fault_alone_389() {
    let faulty: [(&str, Code); 14] = [
        ("state v((none)\n", Code::CellSyntax),
        ("state v(none\n", Code::CellSyntax),
        ("state v({\"a\": 1}\n", Code::CellSyntax),
        ("state v(q ?? 1\n", Code::CellSyntax),
        ("state v(xq\n", Code::CellSyntax),
        ("let v = q is i32[,]\n", Code::TypeSyntax),
        ("let v = q is i32[2\n", Code::TypeSyntax),
        ("let v = q is not i32[2][\n", Code::TypeSyntax),
        (
            "Clip c:\n    Note | velocity=q is i32[,]\n",
            Code::TypeSyntax,
        ),
        ("Clip c:\n    Note when q is i32[,]\n", Code::TypeSyntax),
        ("state v(q is i32[,])\n", Code::TypeSyntax),
        ("state v = q is i32[,]\n", Code::TypeSyntax),
        ("let v = \"a{q is i32[,]}\"\n", Code::TypeSyntax),
        ("let v = f(q is i32[,])\n", Code::TypeSyntax),
    ];
    let built: [(&str, &str); 4] = [
        ("state v(none)\n", "none"),
        ("let v = q is i32[2]\n", "i32[2]"),
        ("state v(q is not i32[2][3])\n", "i32[2][3]"),
        ("Clip c:\n    Note when q is i32[2]\n", "i32[2]"),
    ];
    let mut failures = Vec::new();
    for (item, code) in faulty {
        for text in [format!("{PRELUDE}{item}"), format!("{item}{PRELUDE}")] {
            let codes: Vec<Code> = support::check_codes(&text)
                .iter()
                .map(|(code, ..)| *code)
                .collect();
            if codes != vec![code] {
                failures.push(format!("{text:?} draws {codes:?}, expected [{code:?}]"));
            }
        }
    }
    for (item, form) in built {
        for text in [format!("{PRELUDE}{item}"), format!("{item}{PRELUDE}")] {
            let (start, end) = support::at(&text, form, 0);
            let found = support::check_codes(&text);
            if found != vec![(Code::UnbuiltConstruct, start, end)] {
                failures.push(format!(
                    "{text:?} draws {found:?}, expected one unbuilt-construct at `{form}`"
                ));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
