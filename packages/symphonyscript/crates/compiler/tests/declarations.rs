mod support;

use symphonyscript::Code;
use symphonyscript_fixtures::{Rows, covered};

const BASES: [(&str, bool); 13] = [
    ("i32", true),
    ("bool", true),
    ("Wave", true),
    ("str", false),
    ("str[..8]", false),
    ("json", false),
    ("json[..64]", false),
    ("{i32: i32}[..4]", false),
    ("none", false),
    ("(i32)", false),
    ("Stack[i32, 2]", false),
    ("lib::T", false),
    ("(i32, x: f64) -> bool", false),
];

const DIMS: [(&str, bool); 6] = [
    ("", true),
    ("[3]", true),
    ("[..4]", true),
    ("[3][..4]", true),
    ("[..4][3]", false),
    ("[N + 1]", false),
];

const OPTIONAL: [(&str, bool); 2] = [("", true), ("?", false)];

const SUM: [(&str, bool); 2] = [("", true), (" | i64", false)];

const POSITIONS: [(&str, bool); 6] = [
    ("let v: {} = 1\n", false),
    ("state v(1): {}\n", false),
    ("let s = for x: {} in [1] into a from 0: a\n", false),
    ("let s = for x in [1] into a: {} from 0: a\n", false),
    ("type R:\n    f: {}\n", true),
    ("fn f(x: {}): x\n", true),
];

fn type_rows(rows: &Rows) -> Vec<String> {
    let mut failures = Vec::new();
    for (base_at, (base, base_built)) in BASES.iter().enumerate() {
        for (dims_at, (dims, dims_built)) in DIMS.iter().enumerate() {
            for (optional_at, (optional, optional_built)) in OPTIONAL.iter().enumerate() {
                for (sum_at, (sum, sum_built)) in SUM.iter().enumerate() {
                    for (position_at, (position, unbuilt_anyway)) in POSITIONS.iter().enumerate() {
                        if !rows.take(&[base_at, dims_at, optional_at, sum_at, position_at]) {
                            continue;
                        }
                        let function = base.contains("->");
                        if function && (!dims.is_empty() || !optional.is_empty()) {
                            continue;
                        }
                        let ty = format!("{base}{dims}{optional}{sum}");
                        let text = position.replace("{}", &ty);
                        let syntax = support::codes(&text);
                        if !syntax.is_empty() {
                            failures.push(format!("{text:?} parses with {syntax:?}"));
                            continue;
                        }
                        let built = *base_built && *dims_built && *optional_built && *sum_built;
                        let found = support::check_codes(&text);
                        let unbuilt: Vec<_> = found
                            .iter()
                            .filter(|(code, ..)| *code == Code::UnbuiltConstruct)
                            .collect();
                        let (start, end) = support::at(&text, &ty, 0);
                        let holds = if *unbuilt_anyway {
                            unbuilt.len() == 1 && found.len() == 1
                        } else if built {
                            unbuilt.is_empty()
                        } else {
                            unbuilt.len() == 1
                                && found.len() == 1
                                && unbuilt
                                    .iter()
                                    .all(|(_, from, to)| *from >= start && *to <= end)
                        };
                        if !holds {
                            failures.push(format!("{text:?} draws {found:?}"));
                        }
                    }
                }
            }
        }
    }
    failures
}

#[test]
fn every_type_form_parses_in_every_annotation_and_draws_one_unbuilt_construct_where_it_is_not_built()
 {
    let failures = covered(type_rows);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
#[ignore = "exhaustive"]
fn exhaustive_every_type_form_parses_in_every_annotation_and_draws_one_unbuilt_construct_where_it_is_not_built()
 {
    let failures = type_rows(&Rows::Every);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn a_fault_inside_a_new_bracket_or_item_draws_its_sections_code_once_and_nothing_unbuilt() {
    for (text, code, needle) in [
        ("let a: (i32, = 1\n", Code::TypeSyntax, ","),
        ("let a: (i32 i32) -> i32 = 1\n", Code::TypeSyntax, "i32)"),
        ("let a: (i32) -> = 1\n", Code::TypeSyntax, "="),
        ("let a: {i32 i32}[..4] = 1\n", Code::TypeSyntax, "i32}"),
        ("let a: {i32: i32}[..] = 1\n", Code::TypeSyntax, "]"),
        ("let a: str[..] = 1\n", Code::TypeSyntax, "]"),
        ("let a: json[..,] = 1\n", Code::TypeSyntax, ","),
        ("let a: Stack[i32,,] = 1\n", Code::GenericSyntax, ",]"),
        ("let a: i32[N +] = 1\n", Code::ExpressionSyntax, "]"),
        ("let a: i32 | = 1\n", Code::TypeSyntax, "="),
        ("fn f[,](x): x\n", Code::GenericSyntax, ","),
        ("fn f[T: ](x): x\n", Code::TypeSyntax, "]"),
        ("fn f(x) -> : x\n", Code::TypeSyntax, ":"),
        ("fn f(cell): 1\n", Code::FnSyntax, "cell"),
        (
            "pattern P(out const x):\n    Note\n",
            Code::PatternSyntax,
            "const",
        ),
        (
            "pattern P(children n: Note{x}):\n    Note\n",
            Code::PatternSyntax,
            "x",
        ),
        (
            "pattern P() -> (Note,, Plays):\n    Note\n",
            Code::PatternSyntax,
            ", Plays",
        ),
        (
            "pattern P() -> [Note:\n    Note\n",
            Code::PatternSyntax,
            ":",
        ),
        ("type R:\n    f i32\n", Code::RecordSyntax, "i32"),
        ("type R = \n", Code::TypeSyntax, "\n"),
        ("type R[A,] = A\n", Code::GenericSyntax, "]"),
        (
            "Package {name: \"a\" version: \"1\"}\n",
            Code::RecordSyntax,
            "version",
        ),
        ("Package {name: }\n", Code::ExpressionSyntax, "}"),
        ("require \"a\" b\n", Code::PackageSyntax, "b"),
        ("use a: b c\n", Code::PackageSyntax, "c"),
        ("publish x y\n", Code::PackageSyntax, "y"),
    ] {
        let (start, end) = support::at(text, needle, 0);
        let end = if needle == "\n" { start } else { end };
        let found = support::check_codes(text);
        let at_fault: Vec<_> = found
            .iter()
            .filter(|(found_code, from, _)| *found_code == code && *from == start)
            .collect();
        assert!(
            found.len() == 1 && at_fault.len() == 1,
            "{text:?}: {found:?}, expected {code:?} at {start}..{end}"
        );
    }
}

#[test]
fn a_type_declaration_is_a_placeholder_name_whose_duplicate_is_refused_and_whose_uses_draw_nothing()
{
    let text = "type T = i32\nlet x: T = 1\nlet y = T\nClip c:\n    Note/(x + y)\n";
    let (start, end) = support::at(text, "type T = i32", 0);
    assert_eq!(
        support::check_codes(text),
        vec![(Code::UnbuiltConstruct, start, end)]
    );
    for text in [
        "type T = i32\nlet T = 1\nClip c:\n    Note/T\n",
        "let T = 1\ntype T = i32\nClip c:\n    Note/T\n",
        "type T = i32\ntype T:\n    a: i32\nClip c:\n    Note/T\n",
    ] {
        let found = support::check_codes(text);
        let duplicates = found
            .iter()
            .filter(|(code, ..)| *code == Code::DuplicateName)
            .count();
        let others = found
            .iter()
            .filter(|(code, ..)| !matches!(code, Code::DuplicateName | Code::UnbuiltConstruct))
            .count();
        assert_eq!((duplicates, others), (1, 0), "{text:?}: {found:?}");
    }
}

#[test]
fn a_declaration_with_an_unbuilt_head_is_a_placeholder_its_uses_draw_nothing_of() {
    for (text, unbuilt) in [
        ("fn f(x: i32): x\nClip c:\n    Note/(f(1) + f(2, 3))\n", 1),
        ("fn f(cell x): x.past(1)\nClip c:\n    Note/(f(1))\n", 1),
        ("fn f[T](x): x\nClip c:\n    Note/(f(1))\n", 1),
        ("fn f(x) -> i32: x\nClip c:\n    Note/(f(true))\n", 1),
        (
            "fn f(x):\n    let y: i32 = x\n    y\nClip c:\n    Note/(f(1))\n",
            1,
        ),
        (
            "pattern P(cell x):\n    Note\nClip c:\n    P | x=1\n    P/3\n",
            1,
        ),
        (
            "pattern P(children n: Note*):\n    Note\nClip c:\n    P:\n        Note\n",
            1,
        ),
        ("pattern P(x) -> Clip:\n    Note\nClip c:\n    P | y=1\n", 1),
        (
            "pattern P(x):\n    state s(0)\n    Note\nClip c:\n    P\n",
            1,
        ),
        (
            "pattern P(default out channel hit: i32):\n    Note\nClip c:\n    P/1\n",
            2,
        ),
    ] {
        let found = support::check_codes(text);
        let drawn = found
            .iter()
            .filter(|(code, ..)| *code == Code::UnbuiltConstruct)
            .count();
        assert!(
            drawn == unbuilt && found.len() == unbuilt,
            "{text:?}: {found:?}"
        );
    }
}

#[test]
fn the_examples_of_the_brief_parse_to_trees_that_carry_each_part() {
    for (text, tree) in [
        (
            "let a: (i32?)[..4] = []\n",
            "TypeRef[ParenType[( TypeRef[i32 ?] )] [ .. 4 ]]",
        ),
        ("let a: i32[..4]? = []\n", "TypeRef[i32 [ .. 4 ] ?]"),
        (
            "let a: str[..32][4] = []\n",
            "TypeRef[StrType[str [ .. 32 ]] [ 4 ]]",
        ),
        (
            "let a: T[..N + 1] = []\n",
            "TypeRef[T [ .. Size[BinaryExpr[NameRef[N] + Literal[1]]] ]]",
        ),
        (
            "fn name[T, const N](xs: T[N]): xs\n",
            "GenericParams[[ GenericParam[T] , GenericParam[const N] ]]",
        ),
        (
            "type Shape = Circle | Square\n",
            "TypeDecl[type Shape = SumType[TypeRef[Circle] | TypeRef[Square]] NL]",
        ),
    ] {
        let parsed = support::clean_tree(text);
        assert!(parsed.contains(tree), "{text:?}\n{parsed}");
    }
}
