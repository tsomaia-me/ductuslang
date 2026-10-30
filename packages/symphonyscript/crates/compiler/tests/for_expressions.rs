mod support;

use symphonyscript::{Code, Compiler, Diagnostic};
use symphonyscript_fixtures::{Rows, covered, spec_examples, vocabulary};

const HELD: &str = "input held: i32[..10] = []\n";

const HEADS: [(&str, &str); 6] = [
    ("x", "s + x"),
    ("x: i32", "s + x"),
    ("i, x", "s + i * x"),
    ("i: i32, x", "s + i * x"),
    ("i, x: i32", "s + i * x"),
    ("i: i32, x: i32", "s + i * x"),
];

const ITERABLES: [&str; 2] = ["0..4", "held"];

const WHOLE: [(&str, &str, &str); 4] = [
    ("let v = ", "", ""),
    ("fn f(p):\n    let v = ", "    ", "    v\n"),
    ("state v = ", "", ""),
    ("fn f(p):\n    ", "    ", ""),
];

const WHOLE_LETS: [Option<usize>; 4] = [None, Some(0), Some(1), Some(3)];

const ELSEWHERE: [(&str, &str); 9] = [
    ("let v = 1 + F\n", "0"),
    ("fn g(a): a\nlet v = g(F)\n", "0"),
    ("Clip c:\n    Note | velocity=F\n", "0.5"),
    ("Clip c:\n    Note when F\n", "true"),
    ("let v = match F:\n    0: 1\n    else: 2\n", "0"),
    ("let v = for y in [1] into t from F: t + y\n", "0"),
    ("let v = for y in F into t from 0: t + y\n", "[1]"),
    ("Clip c:\n    for y in F:\n        Note\n", "[1]"),
    ("let v = for y in [1] into t from 0: F\n", "0"),
];

const INSIDE_A_FOR_EXPRESSION: [usize; 3] = [5, 6, 8];

fn for_text(head: usize, iterable: usize, lets: Option<usize>, indent: &str) -> String {
    let (names, body) = HEADS[head];
    let start = format!("for {names} in {} into s from 0:", ITERABLES[iterable]);
    let Some(lets) = lets else {
        return format!("{start} {body}");
    };
    let mut text = format!("{start}\n");
    let mut value = body.to_string();
    for at in 0..lets {
        text.push_str(&format!("{indent}    let a{at} = x + {at}\n"));
        value.push_str(&format!(" + a{at}"));
    }
    text.push_str(&format!("{indent}    {value}\n"));
    text
}

fn spanned<'t>(text: &'t str, diagnostic: &Diagnostic) -> &'t str {
    let range = &diagnostic.span.range;
    text.get(range.start as usize..range.end as usize)
        .unwrap_or_default()
        .trim_end()
}

fn unbuilt_at(compiler: &Compiler, text: &str, construct: &str) -> Result<(), String> {
    let errors = support::errors(compiler.check(&[support::source(text)]));
    match errors.as_slice() {
        [diagnostic]
            if diagnostic.code == Code::UnbuiltConstruct
                && spanned(text, diagnostic) == construct.trim_end()
                && diagnostic.summary.contains("`for` expression (SPEC §5.10)") =>
        {
            Ok(())
        }
        _ => Err(format!(
            "{text}\n  draws {errors:?}\n  expected one unbuilt-construct at {construct:?}\n"
        )),
    }
}

fn unparenthesized_at(compiler: &Compiler, text: &str, at: usize) -> Result<(), String> {
    let parsed = support::parsed(text).diagnostics.clone();
    let errors = support::errors(compiler.check(&[support::source(text)]));
    let expected = |found: &[Diagnostic]| match found {
        [diagnostic] => {
            diagnostic.code == Code::UnparenthesizedFor
                && diagnostic.span.range == (at as u32..at as u32 + 3)
                && diagnostic
                    .help
                    .as_deref()
                    .is_some_and(|help| help.starts_with("parenthesize the `for` expression"))
        }
        _ => false,
    };
    if expected(&parsed) && expected(&errors) {
        return Ok(());
    }
    Err(format!(
        "{text}\n  parses to {parsed:?}\n  checks to {errors:?}\n  expected one unparenthesized-for at {at}\n"
    ))
}

fn twin_holds(compiler: &Compiler, twin: &str) -> Result<(), String> {
    let errors = support::errors(compiler.check(&[support::source(twin)]));
    if errors.is_empty() {
        return Ok(());
    }
    Err(format!("the twin {twin}\n  draws {errors:?}\n"))
}

fn position_rows(rows: &Rows) -> (Vec<String>, usize) {
    let compiler = Compiler::new(&vocabulary()).expect("the vocabulary compiles");
    let mut failures = Vec::new();
    let mut programs = 0;
    for head in 0..HEADS.len() {
        for iterable in 0..ITERABLES.len() {
            for (position, (prefix, indent, rest)) in WHOLE.iter().enumerate() {
                for (form, lets) in WHOLE_LETS.iter().enumerate() {
                    if !rows.take(&[position, form, head, iterable]) {
                        continue;
                    }
                    programs += 1;
                    let construct = for_text(head, iterable, *lets, indent);
                    let line_end = if lets.is_some() { "" } else { "\n" };
                    let text = format!("{HELD}{prefix}{construct}{line_end}{rest}");
                    let twin = format!("{HELD}{prefix}0\n{rest}");
                    let outcome = twin_holds(&compiler, &twin)
                        .and_then(|()| unbuilt_at(&compiler, &text, &construct));
                    failures.extend(outcome.err());
                }
            }
            for (at, (template, value)) in ELSEWHERE.iter().enumerate() {
                for (form, parenthesized) in [true, false].into_iter().enumerate() {
                    let position = WHOLE.len() + at;
                    if !rows.take(&[position, WHOLE_LETS.len() + form, head, iterable]) {
                        continue;
                    }
                    programs += 1;
                    let inner = for_text(head, iterable, None, "");
                    let written = if parenthesized {
                        format!("({inner})")
                    } else {
                        inner.clone()
                    };
                    let text = format!("{HELD}{}", template.replace('F', &written));
                    let twin = format!("{HELD}{}", template.replace('F', value));
                    let start = HELD.len() + template.find('F').unwrap_or(0);
                    let outer = |text: &str| {
                        text[HELD.len()..]
                            .trim_start_matches("let v = ")
                            .to_string()
                    };
                    let nested = INSIDE_A_FOR_EXPRESSION.contains(&at);
                    let twin_outcome = if nested {
                        unbuilt_at(&compiler, &twin, &outer(&twin))
                    } else {
                        twin_holds(&compiler, &twin)
                    };
                    let outcome = twin_outcome.and_then(|()| match (parenthesized, nested) {
                        (false, _) => unparenthesized_at(&compiler, &text, start),
                        (true, true) => unbuilt_at(&compiler, &text, &outer(&text)),
                        (true, false) => unbuilt_at(&compiler, &text, &inner),
                    });
                    failures.extend(outcome.err());
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
        failures.iter().take(6).cloned().collect::<String>()
    );
    assert_eq!(programs, expected);
}

#[test]
fn a_for_expression_stands_bare_only_as_a_whole_value_and_draws_unbuilt_construct_where_it_is_built()
 {
    assert_none_fail(covered(position_rows), 79);
}

#[test]
#[ignore = "exhaustive"]
fn exhaustive_a_for_expression_stands_bare_only_as_a_whole_value_and_draws_unbuilt_construct_where_it_is_built()
 {
    assert_none_fail(position_rows(&Rows::Every), 408);
}

const SPEC_FOR_EXPRESSIONS: [&str; 5] = [
    "state high = for x in held into kept from []:\n    let keep = x > 60\n    if keep then kept + [x] else kept\n",
    "state total = 1 + (for i, x in held into sum from 0: sum + i * x)\n",
    "state lowest = for x in held into low from 127: min(low, x)\n",
    "fn count(xs, keep):\n    for x in xs into n from 0: if keep(x) then n + 1 else n\n",
    "fn mean(xs): (for x in xs into total from 0.0: total + x) / 3\n",
];

#[test]
fn the_for_expressions_of_the_spec_parse_and_draw_unbuilt_construct_and_its_rejected_ones_draw_unparenthesized_for()
 {
    let compiler = Compiler::new(&vocabulary()).expect("the vocabulary compiles");
    for declaration in SPEC_FOR_EXPRESSIONS {
        let text = format!("{HELD}{declaration}");
        assert_eq!(support::parsed(&text).diagnostics, vec![], "{text}");
        let start = text
            .find("for x")
            .or_else(|| text.find("for i"))
            .unwrap_or(0);
        let tail = &text[start..];
        let construct = match tail.find(')') {
            Some(close) if declaration.contains("(for") => &tail[..close],
            Some(_) | None => tail,
        };
        let construct = construct.trim_end_matches(" / 3\n");
        assert_eq!(unbuilt_at(&compiler, &text, construct), Ok(()));
    }
    for ordinal in [1, 2] {
        let example = spec_examples()
            .into_iter()
            .find(|example| {
                example.section == "5.10"
                    && example.tag == "sym-error"
                    && example.ordinal == ordinal
            })
            .expect("the SPEC holds the example");
        let at = example
            .text
            .find("for x in held")
            .expect("the example holds a `for`") as u32;
        let found: Vec<(Code, u32, u32)> = support::codes(&example.text)
            .into_iter()
            .filter(|(code, _, _)| *code == Code::UnparenthesizedFor)
            .collect();
        assert_eq!(
            found,
            [(Code::UnparenthesizedFor, at, at + 3)],
            "{}",
            example.text
        );
    }
}

#[test]
fn a_block_form_after_a_one_line_function_colon_draws_one_fn_syntax_and_builds_nothing() {
    let text = format!("{HELD}fn f(p): for x in held into s from 0:\n    s + x\n");
    let codes: Vec<Code> = support::checked(&text)
        .iter()
        .map(|diagnostic| diagnostic.code)
        .collect();
    assert_eq!(codes, [Code::FnSyntax], "{text}");
    assert_eq!(support::codes(&text).len(), 1, "{text}");
}
