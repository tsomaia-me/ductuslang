use symphonyscript_core::Vocabulary;
use symphonyscript_fixtures::{Rows, covered};
use symphonyscript_fixtures::{
    envelope_vocabulary, native_vocabulary, signature_vocabulary, vocabulary,
};

use crate::compiler::Compiler;
use crate::diagnostics::{Code, Diagnostic};
use crate::source::Source;

const LEAVES: [&str; 10] = [
    "q",
    "1",
    "1.5",
    "true",
    "Wave::Saw",
    "\"a\"",
    "[1, 2]",
    "[1, 2][5]",
    "300000000000",
    "i32(true)",
];
const UNARY: [&str; 6] = ["-", "not ", "i32", "u8", "len", "bytes"];
const BINARY: [&str; 10] = ["+", "-", "*", "/", "div", "%", "and", "or", "<", "is"];

struct Position {
    vocabulary: usize,
    items: &'static [&'static str],
}

const POSITIONS: [Position; 9] = [
    Position {
        vocabulary: 0,
        items: &[
            "fn r(n): if n > 0 then r(n - 1) else 1\n",
            "Clip c:\n    Note/(r(3))\n    Note/(E)\n",
        ],
    },
    Position {
        vocabulary: 0,
        items: &["state note(60)\n", "Clip c:\n    Note when (E)\n"],
    },
    Position {
        vocabulary: 0,
        items: &["state note(60)\n", "Clip c:\n    Note | velocity=(E)\n"],
    },
    Position {
        vocabulary: 1,
        items: &["state note(60)\n", "Clip c | data=(E)\n"],
    },
    Position {
        vocabulary: 0,
        items: &["let a: i32 = E\n", "Clip c:\n    Note/(a)\n"],
    },
    Position {
        vocabulary: 0,
        items: &["let a = E\n", "Clip c:\n    Note/(a)\n    Note/(a + 1)\n"],
    },
    Position {
        vocabulary: 0,
        items: &["fn g(v): v + (E)\n", "Clip c:\n    Note/(g(1))\n"],
    },
    Position {
        vocabulary: 0,
        items: &["pattern P():\n    Note/(E)\n", "Clip c:\n    P\n"],
    },
    Position {
        vocabulary: 0,
        items: &["state s(0) = E\n", "Clip c:\n    Note/(s)\n"],
    },
];

const PROGRAMS: [(usize, &[&str]); 16] = [
    (
        0,
        &[
            "let base_key = Key\n",
            "let lift = base_key.transpose(semitones = 2)\n",
            "Clip bridge:\n    lift\n",
        ],
    ),
    (
        0,
        &[
            "state x(1)\n",
            "let a = nothing + 1.5\n",
            "let k = nothing + (1 + true) + [1, 2][5]\n",
            "Clip c:\n    Note/a\n    Note/(a + 1)\n    Note/k\n    Note/(k * 2)\n",
        ],
    ),
    (
        0,
        &[
            "let b = nothing + [1, 2][5]\n",
            "fn g(x): nothing + (1 + true) + x\n",
            "pattern P():\n    Note/(nothing + (3 + true))\n",
            "Clip k:\n    Note/b\n    Note/(b * 2)\n    Note/(g(1))\n    P\n    P\n",
        ],
    ),
    (
        0,
        &[
            "fn f(a, b, c): if a < 1 then 0 else f(a - 1, b + 1, c) + f(a - 1, b, c + 1) + f(a - 1, b, c)\n",
            "Clip k:\n    Note/(f(6, 0, 0) % 100)\n    Note/(zz)\n",
        ],
    ),
    (
        0,
        &[
            "let v0 = [q]\n",
            "let v1 = v0 + [1]\n",
            "let v2 = v1 + [1]\n",
            "Clip c:\n    Note/(v2[0])\n",
        ],
    ),
    (
        0,
        &[
            "let v0 = q + 1\n",
            "let v1 = q0 + 1\n",
            "Clip c:\n    Note/(v0)\n    Note/(v1)\n    Note when q2 or 1 or 2\n",
        ],
    ),
    (
        0,
        &[
            "fn g(v): v + 1\n",
            "state s0 = s0.past(1, c).g()\n",
            "Clip c:\n    Note/(s0)\n",
        ],
    ),
    (
        0,
        &[
            "fn g(v): v + 1\n",
            "state s0 = s0.past(1, q).g()\n",
            "Clip c:\n    Note/(s0)\n",
        ],
    ),
    (
        0,
        &[
            "fn twice(v): v * 2\n",
            "Clip c:\n    Note/(q.twice())\n    Note/(q.xpast(1))\n    Note/(q.size())\n    Note/(q.i64(3))\n",
        ],
    ),
    (
        0,
        &[
            "fn twice(v): v * 2\n",
            "Clip c:\n    Note/((1 +).twice())\n    Note/(5[0].size())\n    Note/(true[0].i64(3))\n    Note/(to.twice())\n",
        ],
    ),
    (
        2,
        &["Env e | attack=to.i64(3)\n", "Env f | attack=q.i64(3)\n"],
    ),
    (
        3,
        &[
            "let a =to. checksum(\"GET\")\n",
            "let b = q.checksum(\"GET\")\n",
            "Clip c:\n    Note/(len(b))\n",
        ],
    ),
    (
        0,
        &[
            "state gain(0.5)\n",
            "Clip k | gain=gain:\n    Note/(q.gain)\n    Note/(k.gain + q)\n",
            "Clip m:\n    Note/(1)\n",
        ],
    ),
    (
        0,
        &[
            "state xs([1, 2]): i32[..4]\n",
            "Clip c:\n    for x in q:\n        Note/(x)\n    when q:\n        Note/(1)\n    Note/(2)\n",
        ],
    ),
    (
        0,
        &[
            "fn h(n): n.velocity + q\n",
            "Clip c:\n    Note/(1) as one\n    Note/(h(one))\n    Note/(one.velocity)\n",
        ],
    ),
    (
        0,
        &[
            "let a = q\n",
            "let b = a\n",
            "fn g(v): b + v\n",
            "Clip c:\n    Note/(g(1))\n    Note when b or 1 or 2\n    Note/(b + true)\n",
        ],
    ),
];

type Drawn = Vec<(Code, String, u32, u32, String)>;

fn drawn(diagnostics: &[Diagnostic]) -> Drawn {
    diagnostics
        .iter()
        .map(|diagnostic| {
            (
                diagnostic.code,
                diagnostic.span.file.clone(),
                diagnostic.span.range.start,
                diagnostic.span.range.end,
                diagnostic.summary.clone(),
            )
        })
        .collect()
}

fn compilers() -> Vec<Compiler> {
    let vocabularies: [Vocabulary; 4] = [
        vocabulary(),
        signature_vocabulary(),
        envelope_vocabulary(),
        native_vocabulary(),
    ];
    vocabularies
        .iter()
        .map(|vocabulary| Compiler::new(vocabulary).expect("the vocabulary compiles"))
        .collect()
}

fn differs(compiler: &Compiler, items: &[String], orders: &[bool]) -> Option<String> {
    let mut failures = Vec::new();
    for order in orders {
        let mut ordered = items.to_vec();
        if *order {
            ordered.reverse();
        }
        let sources = [Source {
            path: "main.sym".into(),
            text: ordered.concat(),
        }];
        let twin_program = drawn(&compiler.check(&sources));
        let whole_program = drawn(&compiler.checked_by_whole_program(&sources));
        let without_476_moves: Drawn = twin_program
            .iter()
            .filter(|entry| {
                let sliced_base =
                    entry.0 == Code::NotAnArray && entry.4 == "only an array is sliced";
                let condition =
                    entry.0 == Code::ConditionType && entry.4 == "the condition is a `bool`";
                !(sliced_base || condition) || whole_program.contains(entry)
            })
            .cloned()
            .collect();
        if twin_program != whole_program && without_476_moves != whole_program {
            failures.push(crate::pinned_row::failed(
                &ordered.concat(),
                &compiler.check(&sources),
                format!(
                    "{}\n  twin program  {twin_program:?}\n  whole program {whole_program:?}",
                    ordered.concat()
                ),
            ));
        }
    }
    (!failures.is_empty()).then(|| failures.join("\n"))
}

fn expressions() -> Vec<(Vec<usize>, String)> {
    let mut found = vec![(vec![0, 0, 0, 0], LEAVES[0].to_string())];
    for (op, unary) in UNARY.iter().enumerate() {
        let text = match *unary {
            "len" => "(q).size".to_string(),
            "bytes" => "(q).bytes".to_string(),
            "-" | "not " => format!("{unary}q"),
            name => format!("{name}(q)"),
        };
        found.push((vec![1, op, 0, 0], text));
    }
    for (first, lhs) in LEAVES.iter().enumerate() {
        for (second, rhs) in LEAVES.iter().enumerate() {
            if first != 0 && second != 0 {
                continue;
            }
            let (other, side) = if first == 0 { (second, 0) } else { (first, 1) };
            for (op, binary) in BINARY.iter().enumerate() {
                found.push((vec![2, op, other, side], format!("{lhs} {binary} {rhs}")));
            }
            found.push((vec![3, 0, other, side], format!("{lhs}[{rhs}]")));
            found.push((vec![4, 0, other, side], format!("{lhs}[{rhs}..]")));
            for (third, otherwise) in LEAVES.iter().enumerate() {
                found.push((
                    vec![5, third, other, side],
                    format!("if {lhs} then {rhs} else {otherwise}"),
                ));
            }
        }
    }
    found
}

fn class_failures(rows: &Rows) -> (Vec<String>, usize) {
    let compilers = compilers();
    let mut failures = Vec::new();
    let mut programs = 0;
    for (tag, expression) in expressions() {
        for (position, place) in POSITIONS.iter().enumerate() {
            for (direction, reversed) in [false, true].into_iter().enumerate() {
                let mut row = tag.clone();
                row.extend([position, direction]);
                if !rows.take(&row) {
                    continue;
                }
                programs += 1;
                let items: Vec<String> = place
                    .items
                    .iter()
                    .map(|item| item.replace('E', &expression))
                    .collect();
                failures.extend(differs(&compilers[place.vocabulary], &items, &[reversed]));
            }
        }
    }
    (failures, programs)
}
#[test]
fn the_twin_program_draws_what_the_whole_program_twins_draw_for_the_readers_callers_and_receivers_of_a_placeholder_in_both_orders_299()
 {
    let compilers = compilers();
    let failures: Vec<String> = PROGRAMS
        .iter()
        .filter_map(|(vocabulary, items)| {
            let items: Vec<String> = items.iter().map(|item| item.to_string()).collect();
            differs(&compilers[*vocabulary], &items, &[false, true])
        })
        .collect();
    crate::pinned_row::assert_no_failure(&failures);
}

#[test]
fn the_twin_program_draws_what_the_whole_program_twins_draw_for_every_expression_holding_a_placeholder_at_every_position_299()
 {
    let (failures, programs) = covered(class_failures);
    assert!(programs > 0, "the covering design took no row");
    crate::pinned_row::assert_no_failure(&failures);
}

#[test]
#[ignore = "exhaustive"]
fn exhaustive_the_twin_program_draws_what_the_whole_program_twins_draw_for_every_expression_holding_a_placeholder_at_every_position_299()
 {
    let (failures, programs) = class_failures(&Rows::Every);
    assert_eq!(programs, 2 * POSITIONS.len() * expressions().len());
    crate::pinned_row::assert_no_failure(&failures);
}
