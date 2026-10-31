mod support;

use symphonyscript::{Code, Compiler, Diagnostic};
use symphonyscript_fixtures::{Rows, covered, spec_examples, vocabulary};

const HELD: &str = "state held([]): i32[..10]\n";

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

const SAME_VALUES: [&str; 2] = ["0", "held[0]"];

const FAULT: &str = "1 + true";

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

fn codes_of(compiler: &Compiler, text: &str) -> Vec<Code> {
    let mut codes: Vec<Code> = support::errors(compiler.check(&[support::source(text)]))
        .iter()
        .map(|diagnostic| diagnostic.code)
        .collect();
    codes.sort();
    codes
}

fn as_its_twin(compiler: &Compiler, text: &str, twin: &str, faulty: bool) -> Result<(), String> {
    let found = codes_of(compiler, text);
    let mut expected = codes_of(compiler, twin);
    if faulty {
        for code in &mut expected {
            if *code == Code::OperandType {
                *code = Code::IterableType;
            }
        }
    }
    if found == expected {
        return Ok(());
    }
    Err(format!(
        "{text}\n  draws {found:?}\n  its twin {twin}\n  draws {expected:?}\n"
    ))
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

fn position_rows(rows: &Rows) -> (Vec<String>, usize) {
    let compiler = Compiler::new(&vocabulary()).expect("the vocabulary compiles");
    let mut failures = Vec::new();
    let mut programs = 0;
    for (head, (names, _)) in HEADS.iter().enumerate() {
        for (iterable, (iterated, same)) in ITERABLES.iter().zip(SAME_VALUES).enumerate() {
            let value = if names.contains(',') && *iterated == "0..4" {
                FAULT
            } else {
                same
            };
            let faulty = value == FAULT;
            for (position, (prefix, indent, rest)) in WHOLE.iter().enumerate() {
                for (form, lets) in WHOLE_LETS.iter().enumerate() {
                    if !rows.take(&[position, form, head, iterable]) {
                        continue;
                    }
                    programs += 1;
                    let construct = for_text(head, iterable, *lets, indent);
                    let line_end = if lets.is_some() { "" } else { "\n" };
                    let text = format!("{HELD}{prefix}{construct}{line_end}{rest}");
                    let twin = format!("{HELD}{prefix}{value}\n{rest}");
                    failures.extend(as_its_twin(&compiler, &text, &twin, faulty).err());
                }
            }
            for (at, (template, _)) in ELSEWHERE.iter().enumerate() {
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
                    let twin = format!("{HELD}{}", template.replace('F', &format!("({value})")));
                    let start = HELD.len() + template.find('F').unwrap_or(0);
                    let outcome = if parenthesized {
                        as_its_twin(&compiler, &text, &twin, faulty)
                    } else {
                        unparenthesized_at(&compiler, &text, start)
                    };
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
fn a_for_expression_stands_bare_only_as_a_whole_value_and_draws_what_a_value_of_its_type_draws() {
    assert_none_fail(covered(position_rows), 79);
}

#[test]
#[ignore = "exhaustive"]
fn exhaustive_a_for_expression_stands_bare_only_as_a_whole_value_and_draws_what_a_value_of_its_type_draws()
 {
    assert_none_fail(position_rows(&Rows::Every), 408);
}

const SPEC_FOR_EXPRESSIONS: [&str; 4] = [
    "state high = for x in held into kept from []:\n    let keep = x > 60\n    if keep then kept + [x] else kept\n",
    "state total = 1 + (for i, x in held into sum from 0: sum + i * x)\n",
    "state lowest = for x in held into low from 127: min(low, x)\n",
    "fn mean(xs): (for x in xs into total from 0.0: total + x) / 3\n",
];

#[test]
fn the_for_expressions_of_the_spec_pass_check_and_its_rejected_ones_draw_unparenthesized_for() {
    let compiler = Compiler::new(&vocabulary()).expect("the vocabulary compiles");
    for declaration in SPEC_FOR_EXPRESSIONS {
        let text = format!("{HELD}{declaration}");
        assert_eq!(support::parsed(&text).diagnostics, vec![], "{text}");
        assert_eq!(codes_of(&compiler, &text), vec![], "{text}");
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

#[test]
fn a_block_form_that_stands_as_an_operand_or_an_argument_draws_unparenthesized_for() {
    let compiler = Compiler::new(&vocabulary()).expect("the vocabulary compiles");
    let block = "for x in held into s from 0:\n    s + x";
    let mut failures = Vec::new();
    for template in ["state v = 1 + F\n", "fn g(a): a\nstate v = g(F)\n"] {
        let text = format!("{HELD}{}", template.replace('F', block));
        let at = HELD.len() + template.find('F').unwrap_or(0);
        failures.extend(unparenthesized_at(&compiler, &text, at).err());
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

const EXPRESSION_HEAD: &str = "write `for <name> in <iterable> into <name> from <start>:`";

const PLACEMENT_HEAD: &str = "write `for <name> in <iterable>:` with one name or two";

const FAULTED_HEADS: [(&str, &str, &str); 8] = [
    ("for ", "in", " held into s from 0: s"),
    ("for x ", "held", " into s from 0: s + x"),
    ("for x in ", "into", " s from 0: s"),
    ("for x in held ", "s", " from 0: s"),
    ("for x in held into ", "from", " 0: x"),
    ("for x in held into s ", "0", ": s"),
    ("for x in held into s from ", ":", " s"),
    ("for x in held into s from 0 ", "s", ""),
];

const HEAD_POSITIONS: [(&str, &str); 3] = [
    ("let v = ", ""),
    ("state v = ", ""),
    ("state v = 1 + (", ")"),
];

const FAULTED_PLACEMENTS: [(&str, &str, &str); 4] = [
    ("Clip c:\n    for ", "in", " 0..2:\n        Note\n"),
    ("Clip c:\n    for i ", "0", "..2:\n        Note\n"),
    ("Clip c:\n    for i in ", ":", "\n        Note\n"),
    ("Clip c:\n    for i in 0..2 ", "x", ":\n        Note\n"),
];

fn one_for_syntax(
    compiler: &Compiler,
    text: &str,
    at: usize,
    token: &str,
    help: &str,
) -> Result<(), String> {
    let expected = |found: &[Diagnostic]| match found {
        [diagnostic] => {
            diagnostic.code == Code::ForSyntax
                && diagnostic.span.range == (at as u32..(at + token.len()) as u32)
                && diagnostic.help.as_deref() == Some(help)
        }
        _ => false,
    };
    let parsed = support::parsed(text).diagnostics.clone();
    let errors = support::errors(compiler.check(&[support::source(text)]));
    if expected(&parsed) && expected(&errors) {
        return Ok(());
    }
    Err(format!(
        "{text}\n  parses to {parsed:?}\n  checks to {errors:?}\n  expected one for-syntax at {token:?}\n"
    ))
}

#[test]
fn a_fault_in_either_loop_head_draws_for_syntax_with_the_help_naming_the_form_of_its_head() {
    let compiler = Compiler::new(&vocabulary()).expect("the vocabulary compiles");
    let mut failures = Vec::new();
    for (before, token, after) in FAULTED_HEADS {
        for (opening, closing) in HEAD_POSITIONS {
            let prefix = format!("{HELD}{opening}{before}");
            let text = format!("{prefix}{token}{after}{closing}\n");
            failures.extend(
                one_for_syntax(&compiler, &text, prefix.len(), token, EXPRESSION_HEAD).err(),
            );
        }
    }
    for (before, token, after) in FAULTED_PLACEMENTS {
        let text = format!("{before}{token}{after}");
        failures
            .extend(one_for_syntax(&compiler, &text, before.len(), token, PLACEMENT_HEAD).err());
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
