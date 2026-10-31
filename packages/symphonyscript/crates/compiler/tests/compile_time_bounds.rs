mod support;

use std::collections::BTreeSet;

use symphonyscript::{Code, Compiler, Source};
use symphonyscript_fixtures::native_vocabulary;

#[derive(Clone)]
struct Base {
    declarations: &'static [&'static str],
    value: &'static str,
    history: bool,
    length_known: bool,
}

fn bases() -> Vec<(Base, String)> {
    let mut all: Vec<(Base, String)> = BASES
        .iter()
        .map(|base| (base.clone(), base.value.to_string()))
        .collect();
    for base in &BASES {
        all.push((base.clone(), format!("({})[0..]", base.value)));
    }
    all
}

const LITERAL: Base = Base {
    declarations: &[],
    value: "[1, 2, 3]",
    history: false,
    length_known: true,
};

const BASES: [Base; 13] = [
    LITERAL,
    Base {
        declarations: &["let a = [1, 2, 3]\n"],
        value: "a",
        history: false,
        length_known: true,
    },
    Base {
        declarations: &["state xf([1, 2, 3])\n"],
        value: "xf",
        history: false,
        length_known: true,
    },
    Base {
        declarations: &["state xs([1, 2, 3]): i32[..5]\n"],
        value: "xs",
        history: false,
        length_known: false,
    },
    Base {
        declarations: &["state xf([1, 2, 3])\n"],
        value: "xf.past(1)",
        history: true,
        length_known: true,
    },
    Base {
        declarations: &["state xf([1, 2, 3])\n"],
        value: "xf.past(1, [0, 0, 0])",
        history: true,
        length_known: true,
    },
    Base {
        declarations: &["state xs([1, 2, 3]): i32[..5]\n"],
        value: "xs.past(1)",
        history: true,
        length_known: false,
    },
    Base {
        declarations: &["state xs([1, 2, 3]): i32[..5]\n"],
        value: "xs.past(1, [0])",
        history: true,
        length_known: false,
    },
    Base {
        declarations: &["state xs([1, 2, 3]): i32[..5]\n", "state s = xs\n"],
        value: "s",
        history: false,
        length_known: false,
    },
    Base {
        declarations: &[],
        value: "bytes(\"abc\")",
        history: false,
        length_known: true,
    },
    Base {
        declarations: &["state xf([1, 2, 3])\n"],
        value: "bytes(xf)",
        history: false,
        length_known: true,
    },
    Base {
        declarations: &["state n(1)\n"],
        value: "three(n)",
        history: false,
        length_known: true,
    },
    Base {
        declarations: &["state xs([1, 2, 3]): i32[..5]\n"],
        value: "mirror(xs)",
        history: false,
        length_known: false,
    },
];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Position {
    Index,
    Lower,
    Upper,
    IndexFromEnd,
    LowerFromEnd,
    UpperFromEnd,
}

const POSITIONS: [Position; 6] = [
    Position::Index,
    Position::Lower,
    Position::Upper,
    Position::IndexFromEnd,
    Position::LowerFromEnd,
    Position::UpperFromEnd,
];

const OUTSIDE: [&str; 4] = [
    "2 ** 63",
    "-(2 ** 63) - 1",
    "2 ** 70",
    "-170141183460469231731687303715884105727",
];

const INSIDE: [&str; 4] = ["2 ** 63 - 1", "-(2 ** 63)", "0", "-1"];

impl Position {
    fn bracket(self, value: &str) -> (String, usize) {
        let (before, after) = match self {
            Position::Index => ("[", "]"),
            Position::Lower => ("[", "..]"),
            Position::Upper => ("[..", "]"),
            Position::IndexFromEnd => ("[^", "]"),
            Position::LowerFromEnd => ("[^", "..]"),
            Position::UpperFromEnd => ("[..^", "]"),
        };
        (format!("{before}{value}{after}"), before.len())
    }

    fn reads_one_element(self) -> bool {
        matches!(self, Position::Index | Position::IndexFromEnd)
    }
}

struct Built {
    text: String,
    bound: usize,
}

fn build(
    base: &str,
    declarations: &[&str],
    history: bool,
    position: Position,
    value: &str,
    reversed: bool,
) -> Built {
    let (bracket, offset) = position.bracket(value);
    let indexed = format!("{base}{bracket}");
    let read = if position.reads_one_element() {
        indexed
    } else {
        format!("len({indexed})")
    };
    let mut items: Vec<String> = declarations.iter().map(|item| item.to_string()).collect();
    let (state, pitch) = if history {
        (Some(format!("state h = {read}\n")), "h".to_string())
    } else {
        (None, read)
    };
    items.extend(state);
    items.push(format!("Song g:\n    Clip:\n        Note/({pitch})\n"));
    if reversed {
        items.reverse();
    }
    let text = items.concat();
    let bracketed = format!("{base}{bracket}");
    let start = text.find(&bracketed).expect("the program holds its bound");
    Built {
        bound: start + base.len() + offset,
        text,
    }
}

type Found = BTreeSet<(Code, isize, isize, bool)>;

fn found(compiler: &Compiler, built: &Built) -> Found {
    let bound = isize::try_from(built.bound).unwrap_or(isize::MAX);
    support::errors(compiler.check(&[Source {
        path: "main.sym".into(),
        text: built.text.clone(),
    }]))
    .into_iter()
    .map(|diagnostic| {
        let at = |offset: u32| isize::try_from(offset).unwrap_or(isize::MAX) - bound;
        (
            diagnostic.code,
            at(diagnostic.span.range.start).max(-1),
            at(diagnostic.span.range.end),
            diagnostic.summary.contains("outside `i64`"),
        )
    })
    .collect()
}

#[test]
fn a_compile_time_index_bound_or_offset_outside_i64_draws_at_the_bound_whatever_the_base() {
    let compiler = Compiler::new(&native_vocabulary()).expect("the native vocabulary compiles");
    let bases = bases();
    let mut failures = Vec::new();
    let mut programs = 0;
    for position in POSITIONS {
        for (outside, value) in OUTSIDE
            .iter()
            .map(|value| (true, *value))
            .chain(INSIDE.iter().map(|value| (false, *value)))
        {
            for reversed in [false, true] {
                let twin = build(
                    LITERAL.value,
                    LITERAL.declarations,
                    false,
                    position,
                    value,
                    reversed,
                );
                let twin_found = found(&compiler, &twin);
                let length = isize::try_from(value.len()).unwrap_or(isize::MAX);
                if outside {
                    let at_bound = Found::from([(Code::IndexOutOfRange, 0, length, true)]);
                    let plain_index = Found::from([(Code::IndexOutOfRange, 0, length, false)]);
                    let expected = if position == Position::Index {
                        &plain_index
                    } else {
                        &at_bound
                    };
                    if twin_found != *expected {
                        failures.push(format!("{}\n  twin draws {twin_found:?}\n", twin.text));
                    }
                }
                for (base, written) in &bases {
                    let built = build(
                        written,
                        base.declarations,
                        base.history,
                        position,
                        value,
                        reversed,
                    );
                    let found = found(&compiler, &built);
                    programs += 1;
                    let expected: Found = if outside && base.length_known {
                        twin_found.clone()
                    } else if outside {
                        twin_found
                            .iter()
                            .map(|(code, start, end, _)| (*code, *start, *end, true))
                            .collect()
                    } else if base.length_known {
                        twin_found.clone()
                    } else {
                        found
                            .iter()
                            .filter(|(code, ..)| *code != Code::IndexOutOfRange)
                            .copied()
                            .collect()
                    };
                    if found != expected {
                        failures.push(format!(
                            "{}\n  found {found:?}\n  expected {expected:?}\n",
                            built.text
                        ));
                    }
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {programs} programs fail:\n{}",
        failures.len(),
        failures.iter().take(12).cloned().collect::<String>()
    );
    assert_eq!(programs, 2_496);
}
