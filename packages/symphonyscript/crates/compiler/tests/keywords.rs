mod support;

use symphonyscript::Code;

const NEW_KEYWORDS: [&str; 12] = [
    "type", "const", "children", "without", "none", "str", "json", "node", "wire", "require",
    "use", "publish",
];

#[test]
fn a_new_keyword_stands_as_a_member_name_and_draws_its_positions_syntax_code_as_a_name() {
    let mut failures = Vec::new();
    for word in NEW_KEYWORDS {
        for (text, tree) in [
            (
                format!("state x(1)\nlet a = x.{word}\n"),
                format!("FieldExpr[NameRef[x] . {word}]"),
            ),
            (
                format!("Clip c:\n    Note/x.{word}\n"),
                format!("DefaultArg[/ FieldExpr[NameRef[x] . {word}]]"),
            ),
            (
                format!("Clip c:\n    Note | {word}=1 !{word}\n"),
                format!("Attrs[| AttrEntry[{word} = Literal[1]] AttrEntry[! {word}]]"),
            ),
            (
                format!("Clip c:\n    Note | {word}\n"),
                format!("Attrs[| AttrEntry[{word}]]"),
            ),
            (
                format!("Clip c:\n    Note | velocity=1 {word}=2\n"),
                format!("AttrEntry[velocity = Literal[1]] AttrEntry[{word} = Literal[2]]"),
            ),
        ] {
            let found = support::codes(&text);
            let parsed = support::tree(&text);
            if !found.is_empty() || !parsed.contains(&tree) {
                failures.push(format!("{text:?}: {found:?}\n{parsed}"));
            }
        }
        for (text, code) in [
            (format!("let {word} = 1\n"), Code::LetSyntax),
            (format!("state {word}(1)\n"), Code::CellSyntax),
            (format!("fn f({word}): 1\n"), Code::FnSyntax),
            (format!("Clip {word}\n"), Code::PlacementSyntax),
            (
                format!("Clip c:\n    Note as {word}\n"),
                Code::PlacementSyntax,
            ),
            (
                format!("Clip c:\n    for {word} in 0..2:\n        Note\n"),
                Code::ForSyntax,
            ),
        ] {
            let (start, end) = support::at(&text, word, 0);
            let found = support::codes(&text);
            if found != [(code, start, end)] {
                failures.push(format!("{text:?}: {found:?}, expected {code:?}"));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn null_is_a_word_of_json_text_and_never_a_name() {
    let text = "let null = 1\n";
    let (start, end) = support::at(text, "null", 0);
    assert_eq!(support::codes(text), vec![(Code::ReservedWord, start, end)]);
    assert_eq!(support::codes("let d = json [null]\n"), vec![]);
}
