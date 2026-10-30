mod support;

use symphonyscript::{Code, Compiler};
use symphonyscript_fixtures::{Orders, Rows, covered, native_vocabulary, vocabulary};

struct Base {
    declaration: &'static str,
    twin: &'static str,
    value: &'static str,
}

const ROWS: [Base; 3] = [
    Base {
        declaration: "input v: i32[..5] = [1, 2, 3]\n",
        twin: "input v = [1, 2, 3]\n",
        value: "v",
    },
    Base {
        declaration: "input v: i32[..5] = [1, 2, 3]\n",
        twin: "input v = [1, 2, 3]\n",
        value: "v[0..]",
    },
    Base {
        declaration: "input v: i32[..5] = [1, 2, 3]\ninput k = false\n",
        twin: "input v = [1, 2, 3]\ninput k = false\n",
        value: "(if k then v else v)",
    },
];

const GRIDS: [Base; 3] = [
    Base {
        declaration: "input w: i32[2][..5] = [[1, 2], [3, 4], [5, 6]]\n",
        twin: "input w = [[1, 2], [3, 4], [5, 6]]\n",
        value: "w",
    },
    Base {
        declaration: "input w: i32[2][..5] = [[1, 2], [3, 4], [5, 6]]\n",
        twin: "input w = [[1, 2], [3, 4], [5, 6]]\n",
        value: "w[0..]",
    },
    Base {
        declaration: "input w: i32[2][..5] = [[1, 2], [3, 4], [5, 6]]\ninput k = false\n",
        twin: "input w = [[1, 2], [3, 4], [5, 6]]\ninput k = false\n",
        value: "(if k then w else w)",
    },
];

const ROW_READS: [&str; 4] = [
    "Note/(pair_sum(B[0]))",
    "Note/(B[0] + 1)",
    "Note/(B[0][0])",
    "Note/(len(B[0]))",
];

const GRID_READS: [&str; 6] = [
    "Note/((B[0] + B[2])[3])",
    "Note/(pair_sum(B[1]))",
    "Note/(B[1][0])",
    "Note/(len(B[0] + B[1]))",
    "Note/(B[0] + 1)",
    "Note/(B[0][0][0])",
];

fn element_rows(rows: &Rows, orders: Orders) -> (Vec<String>, usize) {
    let compiler = Compiler::new(&native_vocabulary()).expect("the vocabulary compiles");
    let mut failures = Vec::new();
    let mut programs = 0;
    for (dimension, (bases, reads)) in [(&ROWS[..], &ROW_READS[..]), (&GRIDS[..], &GRID_READS[..])]
        .into_iter()
        .enumerate()
    {
        for (base_tag, base) in bases.iter().enumerate() {
            for (read_tag, read) in reads.iter().enumerate() {
                let clip = format!("Clip c:\n    {}\n", read.replace('B', base.value));
                for (order_tag, (declaration, twin)) in orders
                    .of(&[base.declaration, clip.as_str()])
                    .into_iter()
                    .zip(orders.of(&[base.twin, clip.as_str()]))
                    .enumerate()
                {
                    if !rows.take(&[dimension * 3 + base_tag, read_tag, order_tag]) {
                        continue;
                    }
                    programs += 1;
                    let text = declaration.concat();
                    let twin = twin.concat();
                    let found = support::error_texts(&compiler, &text);
                    let expected = support::error_texts(&compiler, &twin);
                    if found != expected {
                        failures.push(format!("{text}\n  found {found:?}\n  twin {expected:?}\n"));
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
        failures.iter().take(8).cloned().collect::<String>()
    );
    assert_eq!(programs, expected);
}

#[test]
fn an_element_read_has_its_elements_shape_whatever_array_it_reads() {
    assert_none_fail(
        covered(|rows| element_rows(rows, Orders::ForwardAndReversed)),
        30,
    );
}

#[test]
#[ignore = "exhaustive"]
fn exhaustive_an_element_read_has_its_elements_shape_whatever_array_it_reads() {
    assert_none_fail(element_rows(&Rows::Every, Orders::Every), 60);
}

#[test]
fn a_choice_whose_condition_an_error_leaves_without_a_value_is_laid_out_as_a_runtime_choice() {
    let compiler = Compiler::new(&vocabulary()).expect("the vocabulary compiles");
    let declarations = ["input k: i32[..4] = [1]\n", "input note = 60\n"];
    for condition in [
        "[0] > 1",
        "[note] > 1",
        "1 and [1]",
        "[1, 2][5] > 0",
        "i32(true) > 0",
    ] {
        for (then, otherwise) in [("k", "[3]"), ("[3]", "k"), ("k", "[3, 4, 5]")] {
            for read in ["(C)[0]", "len(C)"] {
                let choice = format!("if {condition} then {then} else {otherwise}");
                let twin = format!("if {condition} then k else k");
                let clip =
                    |value: &str| format!("Clip c:\n    Note/({})\n", read.replace('C', value));
                for (items, twins) in Orders::ForwardAndReversed
                    .of(&[declarations[0], declarations[1], clip(&choice).as_str()])
                    .into_iter()
                    .zip(Orders::ForwardAndReversed.of(&[
                        declarations[0],
                        declarations[1],
                        clip(&twin).as_str(),
                    ]))
                {
                    let text = items.concat();
                    let found: Vec<Code> = support::error_texts(&compiler, &text)
                        .into_iter()
                        .map(|(code, _)| code)
                        .collect();
                    let expected: Vec<Code> = support::error_texts(&compiler, &twins.concat())
                        .into_iter()
                        .map(|(code, _)| code)
                        .collect();
                    assert_eq!(found, expected, "{text}");
                }
            }
        }
    }
}

#[test]
fn a_compile_time_choice_of_arrays_of_one_dimension_takes_the_arm_it_chooses() {
    let compiler = Compiler::new(&vocabulary()).expect("the vocabulary compiles");
    let rows: [(&str, &str, &[Code]); 6] = [
        ("true", "x[0][0]", &[]),
        ("false", "x[0][1]", &[]),
        ("1 > 2", "x[0][1]", &[]),
        ("true", "len(x[0])", &[]),
        ("true", "x[0]", &[Code::TypeMismatch]),
        ("false", "x[1][0]", &[Code::IndexOutOfRange]),
    ];
    for (chooser, read, expected) in rows {
        let items = [
            format!("let x = if {chooser} then [[1]] else [[1, 2]]\n"),
            format!("Clip c:\n    Note/({read})\n"),
        ];
        let items: Vec<&str> = items.iter().map(String::as_str).collect();
        for order in Orders::ForwardAndReversed.of(&items) {
            let text = order.concat();
            let found: Vec<Code> = support::error_texts(&compiler, &text)
                .into_iter()
                .map(|(code, _)| code)
                .collect();
            assert_eq!(found, expected, "{text}");
        }
    }
    let mixed = "Clip c:\n    Note/((if true then [1] else [[1]])[0])\n";
    assert_eq!(
        support::error_texts(&compiler, mixed)
            .into_iter()
            .map(|(code, _)| code)
            .collect::<Vec<_>>(),
        vec![Code::TypeMismatch],
        "{mixed}"
    );
}
