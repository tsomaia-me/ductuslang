use symphonyscript::syntax::{SyntaxKind, lex};
use symphonyscript::{Code, Diagnostic, Source};
use symphonyscript_fixtures::{spec_examples, vocabulary};

fn source(text: &str) -> Source {
    Source {
        path: "test.sym".into(),
        text: text.into(),
    }
}

fn tokens(text: &str) -> String {
    let lexed = lex(&source(text), &vocabulary());
    render(text, &lexed.tokens)
}

fn render(text: &str, tokens: &[symphonyscript::syntax::Token]) -> String {
    tokens
        .iter()
        .map(|token| {
            let piece = &text[token.range.start as usize..token.range.end as usize];
            match token.kind {
                SyntaxKind::Whitespace => "_".to_string(),
                SyntaxKind::Newline => "NL".to_string(),
                SyntaxKind::Indent => ">".to_string(),
                SyntaxKind::Dedent => "<".to_string(),
                SyntaxKind::Ident
                | SyntaxKind::Number
                | SyntaxKind::StringText
                | SyntaxKind::Comment
                | SyntaxKind::Error => format!("{:?}({piece})", token.kind),
                kind => format!("{kind:?}"),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn diagnostics(text: &str) -> Vec<(Code, u32, u32)> {
    lex(&source(text), &vocabulary())
        .diagnostics
        .iter()
        .map(|diagnostic| {
            (
                diagnostic.code,
                diagnostic.span.range.start,
                diagnostic.span.range.end,
            )
        })
        .collect()
}

fn clean(text: &str) -> String {
    let lexed = lex(&source(text), &vocabulary());
    assert_eq!(lexed.diagnostics, Vec::<Diagnostic>::new(), "{text:?}");
    render(text, &lexed.tokens)
}

#[test]
fn every_line_holding_a_token_ends_with_newline() {
    assert_eq!(clean("Note"), "Ident(Note) NL");
    assert_eq!(clean("Note\n"), "Ident(Note) NL");
    assert_eq!(clean("Note\r\nClip\r\n"), "Ident(Note) NL Ident(Clip) NL");
    assert_eq!(clean(""), "");
}

#[test]
fn indentation_pushes_and_pops_columns() {
    assert_eq!(
        clean("Clip a:\n    Note\n        x\nb\n"),
        "Ident(Clip) _ Ident(a) Colon NL _ > Ident(Note) NL _ > Ident(x) NL < < Ident(b) NL"
    );
    assert_eq!(
        clean("a:\n  b:\n    c"),
        "Ident(a) Colon NL _ > Ident(b) Colon NL _ > Ident(c) NL < <"
    );
}

#[test]
fn blank_and_comment_lines_do_not_affect_layout() {
    assert_eq!(
        clean("a:\n\n  // note\n    b\n      \n    c // end\n"),
        "Ident(a) Colon NL _ _ Comment(// note) _ _ > Ident(b) NL _ _ _ Ident(c) _ Comment(// end) NL <"
    );
}

#[test]
fn a_dedent_must_land_on_a_column_of_the_stack() {
    let text = "a:\n    b\n  c\n";
    assert_eq!(diagnostics(text), vec![(Code::DedentMismatch, 11, 12)]);
    assert_eq!(
        tokens(text),
        "Ident(a) Colon NL _ > Ident(b) NL _ < Ident(c) NL"
    );
}

#[test]
fn a_tab_in_leading_whitespace_is_an_error() {
    assert_eq!(diagnostics("a:\n\tb\n"), vec![(Code::TabIndentation, 3, 4)]);
    assert_eq!(diagnostics("a:\n  \t\n  b\n"), vec![]);
    assert_eq!(diagnostics("a\tb\n"), vec![]);
}

#[test]
fn brackets_suspend_layout() {
    assert_eq!(
        clean("let x = [1,\n2,\n    3]\nlet y = (1 +\n  2)\n"),
        "LetKw _ Ident(x) _ Eq _ LBracket Number(1) Comma _ Number(2) Comma _ _ Number(3) RBracket NL \
         LetKw _ Ident(y) _ Eq _ LParen Number(1) _ Plus _ _ Number(2) RParen NL"
    );
}

#[test]
fn an_attribute_clause_continues_at_its_first_entry_column() {
    let pad = " ".repeat(14);
    let text =
        &format!("Clip m:\n    Note/62 | velocity=0.7\n{pad}brightness=w\n{pad}accent\n    Note\n");
    assert_eq!(
        clean(text),
        "Ident(Clip) _ Ident(m) Colon NL _ > Ident(Note) Slash Number(62) _ Pipe _ Ident(velocity) Eq Number(0.7) \
         _ _ Ident(brightness) Eq Ident(w) _ _ Ident(accent) NL _ Ident(Note) NL <"
    );
}

#[test]
fn a_colon_closes_the_clause_and_a_double_colon_does_not() {
    let text = "Osc o | wave=Wave::Saw\n        rate=1:\n    Note\n";
    assert_eq!(
        clean(text),
        "Ident(Osc) _ Ident(o) _ Pipe _ Ident(wave) Eq Ident(Wave) Colon2 Ident(Saw) _ _ Ident(rate) Eq Number(1) Colon NL _ > Ident(Note) NL <"
    );
    let closed = "Clip c | gain=1:\n        Note\n";
    assert_eq!(
        clean(closed),
        "Ident(Clip) _ Ident(c) _ Pipe _ Ident(gain) Eq Number(1) Colon NL _ > Ident(Note) NL <"
    );
}

#[test]
fn a_line_at_another_column_ends_the_clause() {
    assert_eq!(
        clean("Note | a=1\n  b=2\n"),
        "Ident(Note) _ Pipe _ Ident(a) Eq Number(1) NL _ > Ident(b) Eq Number(2) NL <"
    );
    assert_eq!(
        clean("Note | a=1\nb=2\n"),
        "Ident(Note) _ Pipe _ Ident(a) Eq Number(1) NL Ident(b) Eq Number(2) NL"
    );
}

#[test]
fn a_line_aligned_only_on_screen_is_an_error_and_continues_the_clause() {
    let text = &format!("Clip 音楽 | gain=1\n{}level=2\n", " ".repeat(12));
    let found = lex(&source(text), &vocabulary());
    let codes: Vec<(Code, u32, u32)> = found
        .diagnostics
        .iter()
        .map(|d| (d.code, d.span.range.start, d.span.range.end))
        .collect();
    let level = text.find("level").unwrap() as u32;
    assert_eq!(codes, vec![(Code::ClauseColumn, level, level + 1)]);
    assert!(found.diagnostics[0].message.contains("column 11"));
    assert_eq!(
        render(text, &found.tokens),
        "Ident(Clip) _ Ident(音楽) _ Pipe _ Ident(gain) Eq Number(1) _ _ Ident(level) Eq Number(2) NL"
    );
    assert_eq!(
        clean(&format!("Clip 音楽 | gain=1\n{}level=2\n", " ".repeat(10))),
        "Ident(Clip) _ Ident(音楽) _ Pipe _ Ident(gain) Eq Number(1) _ _ Ident(level) Eq Number(2) NL"
    );
}

#[test]
fn a_semicolon_is_an_error() {
    assert_eq!(diagnostics("let a = 1;\n"), vec![(Code::Semicolon, 9, 10)]);
    assert_eq!(diagnostics("let a = \"x;y\" // a;b\n"), vec![]);
}

#[test]
fn comments_run_to_the_end_of_the_line() {
    assert_eq!(
        clean("a // b \"c\"\n// d\n"),
        "Ident(a) _ Comment(// b \"c\") NL Comment(// d) _"
    );
    assert_eq!(clean("a // b\r\n"), "Ident(a) _ Comment(// b) NL");
}

#[test]
fn a_double_slash_is_text_in_a_string_and_a_comment_in_an_interpolation() {
    assert_eq!(
        clean(r#"let a = "x // y{b}// z""#),
        "LetKw _ Ident(a) _ Eq _ StringStart StringText(x // y) InterpStart Ident(b) InterpEnd StringText(// z) StringEnd NL"
    );
    let text = r#"let a = "{b // c}""#;
    assert_eq!(diagnostics(text), vec![(Code::UnterminatedString, 8, 18)]);
    assert_eq!(
        tokens(text),
        "LetKw _ Ident(a) _ Eq _ StringStart InterpStart Ident(b) _ Comment(// c}\") NL"
    );
}

#[test]
fn identifiers_take_xid_characters_and_hash() {
    assert_eq!(
        clean("C#4 F#3 _x #a über x1_y"),
        "Ident(C#4) _ Ident(F#3) _ Ident(_x) _ Ident(#a) _ Ident(über) _ Ident(x1_y) NL"
    );
    assert_eq!(
        diagnostics("_ # __#"),
        vec![
            (Code::InvalidIdentifier, 0, 1),
            (Code::InvalidIdentifier, 2, 3),
            (Code::InvalidIdentifier, 4, 7),
        ]
    );
}

#[test]
fn keywords_are_their_own_tokens() {
    assert_eq!(
        clean(
            "input let fn pattern default if then else match when for in as is not and or div to true false i32 i64 f32 f64 bool"
        ),
        "InputKw _ LetKw _ FnKw _ PatternKw _ DefaultKw _ IfKw _ ThenKw _ ElseKw _ MatchKw _ WhenKw _ ForKw _ InKw _ AsKw _ IsKw _ NotKw _ AndKw _ OrKw _ DivKw _ ToKw _ TrueKw _ FalseKw _ I32Kw _ I64Kw _ F32Kw _ F64Kw _ BoolKw NL"
    );
}

#[test]
fn reserved_words_are_errors() {
    assert_eq!(
        diagnostics("from recurrent string strings"),
        vec![
            (Code::ReservedWord, 0, 4),
            (Code::ReservedWord, 5, 14),
            (Code::ReservedWord, 15, 21),
        ]
    );
}

#[test]
fn reserved_tokens_are_errors_with_hints() {
    let lexed = lex(&source("a == b != c ^ d"), &vocabulary());
    let found: Vec<(Code, u32, Option<&str>)> = lexed
        .diagnostics
        .iter()
        .map(|d| (d.code, d.span.range.start, d.hint.as_deref()))
        .collect();
    assert_eq!(
        found,
        vec![
            (Code::ReservedToken, 2, Some("compare with `is`")),
            (Code::ReservedToken, 7, Some("compare with `is not`")),
        ]
    );
    assert_eq!(
        render("a == b != c ^ d", &lexed.tokens),
        "Ident(a) _ Error(==) _ Ident(b) _ Error(!=) _ Ident(c) _ Caret _ Ident(d) NL"
    );
}

#[test]
fn punctuation_lexes_greedily() {
    assert_eq!(
        clean("( ) [ ] { } , : :: = | / . .. + - * ** % < <= > >= ! ' ? ~ @ $ ^"),
        "LParen _ RParen _ LBracket _ RBracket _ LBrace _ RBrace _ Comma _ Colon _ Colon2 _ Eq _ Pipe _ Slash _ Dot _ Dot2 _ Plus _ Minus _ Star _ Star2 _ Percent _ Lt _ Le _ Gt _ Ge _ Bang _ Quote _ Question _ Tilde _ At _ Dollar _ Caret NL"
    );
    assert_eq!(
        clean("Note'/65 n-1 0..8"),
        "Ident(Note) Quote Slash Number(65) _ Ident(n) Minus Number(1) _ Number(0) Dot2 Number(8) NL"
    );
}

#[test]
fn unexpected_characters_are_one_error_per_run() {
    assert_eq!(
        diagnostics("a &&& b ` ` \\"),
        vec![
            (Code::UnexpectedCharacter, 2, 5),
            (Code::UnexpectedCharacter, 8, 9),
            (Code::UnexpectedCharacter, 10, 11),
            (Code::UnexpectedCharacter, 12, 13),
        ]
    );
    assert_eq!(diagnostics("a\rb"), vec![(Code::UnexpectedCharacter, 1, 2)]);
}

#[test]
fn decimal_literals_take_digits_fractions_and_exponents() {
    assert_eq!(
        clean("1_000_000 0.5 2e3 2E+3 1.5e-3 0 007"),
        "Number(1_000_000) _ Number(0.5) _ Number(2e3) _ Number(2E+3) _ Number(1.5e-3) _ Number(0) _ Number(007) NL"
    );
    assert_eq!(clean("5.x"), "Number(5) Dot Ident(x) NL");
}

#[test]
fn an_underscore_only_separates_two_digits() {
    assert_eq!(
        diagnostics("1__0 1_ 0x1_ 1.5_ 1e1_"),
        vec![
            (Code::MisplacedUnderscore, 1, 2),
            (Code::MisplacedUnderscore, 2, 3),
            (Code::MisplacedUnderscore, 6, 7),
            (Code::MisplacedUnderscore, 11, 12),
            (Code::MisplacedUnderscore, 16, 17),
            (Code::MisplacedUnderscore, 21, 22),
        ]
    );
}

#[test]
fn a_float_needs_digits_before_its_point() {
    assert_eq!(diagnostics(".5"), vec![(Code::FloatDigits, 0, 2)]);
    assert_eq!(tokens("x = .5\n"), "Ident(x) _ Eq _ Number(.5) NL");
}

#[test]
fn suffixes_are_type_suffixes_or_vocabulary_units() {
    assert_eq!(
        clean("5i64 0.5f64 5i32 1e3f32 200ms 2bars 1.0s 0bars"),
        "Number(5i64) _ Number(0.5f64) _ Number(5i32) _ Number(1e3f32) _ Number(200ms) _ Number(2bars) _ Number(1.0s) _ Number(0bars) NL"
    );
    assert_eq!(
        diagnostics("0.5i32 1e3i64 2eighths 5q 4i8"),
        vec![
            (Code::FloatTypeSuffix, 3, 6),
            (Code::FloatTypeSuffix, 10, 13),
            (Code::UnknownSuffix, 15, 22),
            (Code::UnknownSuffix, 24, 25),
            (Code::UnknownSuffix, 27, 29),
        ]
    );
}

#[test]
fn prefixed_literals_take_no_suffix() {
    assert_eq!(
        clean("0x1F 0b101 0o17 0xFFf32"),
        "Number(0x1F) _ Number(0b101) _ Number(0o17) _ Number(0xFFf32) NL"
    );
    assert_eq!(diagnostics("0x_1"), vec![(Code::UnknownSuffix, 1, 4)]);
}

#[test]
fn a_letter_or_digit_after_prefixed_digits_is_an_error() {
    assert_eq!(
        diagnostics("0b12 0o78 0x1g"),
        vec![
            (Code::PrefixedSuffix, 3, 4),
            (Code::PrefixedSuffix, 8, 9),
            (Code::PrefixedSuffix, 13, 14),
        ]
    );
    assert_eq!(tokens("0b12"), "Number(0b12) NL");
}

#[test]
fn integer_literals_stay_below_two_to_the_127() {
    assert_eq!(
        diagnostics("170141183460469231731687303715884105727"),
        vec![]
    );
    assert_eq!(
        diagnostics("170141183460469231731687303715884105728"),
        vec![(Code::IntegerTooLarge, 0, 39)]
    );
    assert_eq!(
        diagnostics("999999999999999999999999999999999999999999"),
        vec![(Code::IntegerTooLarge, 0, 42)]
    );
    assert_eq!(diagnostics("0x7fffffffffffffffffffffffffffffff"), vec![]);
    assert_eq!(
        diagnostics("0x80000000000000000000000000000000"),
        vec![(Code::IntegerTooLarge, 2, 34)]
    );
    assert_eq!(diagnostics("1e400 300000000000"), vec![]);
}

#[test]
fn strings_hold_text_escapes_and_interpolations() {
    assert_eq!(
        clean(r#"file="kit/{i}.wav""#),
        "Ident(file) Eq StringStart StringText(kit/) InterpStart Ident(i) InterpEnd StringText(.wav) StringEnd NL"
    );
    assert_eq!(
        clean(r#""a\"b\\c\nd\te\{f}g // h""#),
        r#"StringStart StringText(a\"b\\c\nd\te\{f}g // h) StringEnd NL"#
    );
    assert_eq!(
        clean(r#""x{f("y{z}")}w""#),
        "StringStart StringText(x) InterpStart Ident(f) LParen StringStart StringText(y) InterpStart Ident(z) InterpEnd StringEnd RParen InterpEnd StringText(w) StringEnd NL"
    );
    assert_eq!(clean(r#""""#), "StringStart StringEnd NL");
}

#[test]
fn an_invalid_escape_is_an_error() {
    assert_eq!(
        diagnostics(r#""a\qb\}" "c\"#),
        vec![
            (Code::InvalidEscape, 2, 4),
            (Code::InvalidEscape, 5, 7),
            (Code::InvalidEscape, 11, 12),
            (Code::UnterminatedString, 9, 12),
        ]
    );
}

#[test]
fn a_string_occupies_one_line() {
    let text = "let a = \"abc\nlet b = 1\n";
    assert_eq!(diagnostics(text), vec![(Code::UnterminatedString, 8, 12)]);
    assert_eq!(
        tokens(text),
        "LetKw _ Ident(a) _ Eq _ StringStart StringText(abc) NL LetKw _ Ident(b) _ Eq _ Number(1) NL"
    );
    let interp = "let a = \"x{(1\nlet b = 2\n";
    assert_eq!(diagnostics(interp), vec![(Code::UnterminatedString, 8, 13)]);
    assert_eq!(
        tokens(interp),
        "LetKw _ Ident(a) _ Eq _ StringStart StringText(x) InterpStart LParen Number(1) NL LetKw _ Ident(b) _ Eq _ Number(2) NL"
    );
}

#[test]
fn every_byte_belongs_to_a_token() {
    let mut texts: Vec<String> = spec_examples()
        .into_iter()
        .filter(|example| example.tag.starts_with("sym"))
        .map(|example| example.text)
        .collect();
    texts.extend([
        "a \t b\r\n\r\n  c \"unterminated {x\n\t;^ é\u{301} 0b12 .5 \\".to_string(),
        "x\r".to_string(),
    ]);
    for text in texts {
        let lexed = lex(&source(&text), &vocabulary());
        let mut at = 0;
        let mut rebuilt = String::new();
        for token in &lexed.tokens {
            assert_eq!(token.range.start, at, "{text:?}");
            at = token.range.end;
            rebuilt.push_str(&text[token.range.start as usize..token.range.end as usize]);
        }
        assert_eq!(rebuilt, text);
    }
}

#[test]
fn every_sym_example_lexes_without_errors() {
    for example in spec_examples() {
        if example.tag == "sym" || example.tag == "sym-error" {
            let lexed = lex(&source(&example.text), &vocabulary());
            assert_eq!(lexed.diagnostics, vec![], "{}", example.name());
        }
    }
}

#[test]
fn keyword_and_word_lists_agree_with_core() {
    for keyword in symphonyscript_core::KEYWORDS {
        assert!(SyntaxKind::keyword(keyword).is_some(), "{keyword}");
    }
    let keyword_kinds = SyntaxKind::ALL
        .iter()
        .filter(|kind| format!("{kind:?}").ends_with("Kw"))
        .count();
    assert_eq!(keyword_kinds, symphonyscript_core::KEYWORDS.len());
    for (raw, kind) in SyntaxKind::ALL.iter().enumerate() {
        assert_eq!(*kind as u16 as usize, raw);
        assert_eq!(SyntaxKind::from_raw(raw as u16), Some(*kind));
    }
}
