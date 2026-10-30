mod support;

use symphonyscript::{Code, Compiler};
use symphonyscript_fixtures::{Orders, Rows, covered, vocabulary};

type InlineRow = (Vec<String>, Vec<(Code, &'static str)>);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Form {
    Literal,
    ConstantAttribute,
    DefaultAttribute,
    CellAttribute,
    Cell,
    LetTakingAValue,
    LetTakingACell,
    ChoiceTakingAValue,
}

const FORMS: [Form; 8] = [
    Form::Literal,
    Form::ConstantAttribute,
    Form::DefaultAttribute,
    Form::CellAttribute,
    Form::Cell,
    Form::LetTakingAValue,
    Form::LetTakingACell,
    Form::ChoiceTakingAValue,
];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Asker {
    RecursiveBody,
    RecursiveArgument,
    LetBound,
    DirectBound,
}

const ASKERS: [Asker; 4] = [
    Asker::RecursiveBody,
    Asker::RecursiveArgument,
    Asker::LetBound,
    Asker::DirectBound,
];

impl Form {
    fn read(self) -> &'static str {
        match self {
            Form::Literal => "3.0",
            Form::ConstantAttribute => "v.gain",
            Form::DefaultAttribute => "v.level",
            Form::CellAttribute => "w.gain",
            Form::Cell => "g",
            Form::LetTakingAValue => "x",
            Form::LetTakingACell => "y",
            Form::ChoiceTakingAValue => "(if false then g else 3.0)",
        }
    }

    fn declarations(self) -> &'static [&'static str] {
        match self {
            Form::Literal => &[],
            Form::ConstantAttribute | Form::DefaultAttribute => &["Clip v | gain=3\n"],
            Form::CellAttribute => &["input g = 2.0\n", "Clip w | gain=g\n"],
            Form::Cell | Form::ChoiceTakingAValue => &["input g = 2.0\n"],
            Form::LetTakingAValue => &["input g = 2.0\n", "let x = if false then g else 3.0\n"],
            Form::LetTakingACell => &["input g = 2.0\n", "let y = if true then g else 3.0\n"],
        }
    }

    fn reads_cell_at(self, asker: Asker) -> bool {
        match self {
            Form::Literal
            | Form::ConstantAttribute
            | Form::DefaultAttribute
            | Form::LetTakingAValue => false,
            Form::CellAttribute | Form::Cell | Form::LetTakingACell => true,
            Form::ChoiceTakingAValue => asker == Asker::RecursiveBody,
        }
    }
}

impl Asker {
    fn items(self, read: &str) -> Vec<String> {
        match self {
            Asker::RecursiveBody => vec![
                format!("fn r(n): if n < 1 then i32({read}) else r(n - 1)\n"),
                "Clip c:\n    Note/(60 + r(3))\n".to_string(),
            ],
            Asker::RecursiveArgument => vec![
                "fn r(n): if n < 1 then 0 else r(n - 1)\n".to_string(),
                format!("Clip c:\n    Note/(60 + r(i32({read})))\n"),
            ],
            Asker::LetBound => vec![
                format!("let k = i32({read})\n"),
                "Clip c:\n    for i in 0..k:\n        Note/60\n".to_string(),
            ],
            Asker::DirectBound => vec![format!(
                "Clip c:\n    for i in 0..i32({read}):\n        Note/60\n"
            )],
        }
    }

    fn fault(self, read: &str) -> (Code, String) {
        match self {
            Asker::RecursiveBody => (Code::ReactiveRecursion, "r(3)".to_string()),
            Asker::RecursiveArgument => (Code::ReactiveRecursion, format!("r(i32({read}))")),
            Asker::LetBound => (Code::ReactiveIterable, "k".to_string()),
            Asker::DirectBound => (Code::ReactiveIterable, format!("i32({read})")),
        }
    }
}

fn reads_cell_rows(rows: &Rows, orders: Orders) -> (Vec<String>, usize) {
    let compiler = Compiler::new(&vocabulary()).expect("the vocabulary compiles");
    let mut failures = Vec::new();
    let mut programs = 0;
    for (form_tag, form) in FORMS.iter().enumerate() {
        for (asker_tag, asker) in ASKERS.iter().enumerate() {
            let mut items: Vec<String> = form
                .declarations()
                .iter()
                .map(|item| (*item).to_string())
                .collect();
            items.extend(asker.items(form.read()));
            let expected: Vec<(Code, String)> = if form.reads_cell_at(*asker) {
                vec![asker.fault(form.read())]
            } else {
                Vec::new()
            };
            let item_refs: Vec<&str> = items.iter().map(String::as_str).collect();
            for (order_tag, order) in orders.of(&item_refs).into_iter().enumerate() {
                if !rows.take(&[form_tag, asker_tag, order_tag]) {
                    continue;
                }
                programs += 1;
                let text = order.concat();
                let found = support::error_texts(&compiler, &text);
                if found != expected {
                    failures.push(format!(
                        "{text}\n  found {found:?}\n  expected {expected:?}\n"
                    ));
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
fn a_value_reads_a_cell_by_the_bindings_it_reaches_at_every_position_that_asks() {
    assert_none_fail(
        covered(|rows| reads_cell_rows(rows, Orders::ForwardAndReversed)),
        32,
    );
}

#[test]
#[ignore = "exhaustive"]
fn exhaustive_a_value_reads_a_cell_by_the_bindings_it_reaches_at_every_position_that_asks() {
    assert_none_fail(reads_cell_rows(&Rows::Every, Orders::Every), 321);
}

#[test]
fn a_recursive_call_counts_toward_the_inline_limit_unless_it_reads_a_cell() {
    let four = Compiler::with_limits(
        &vocabulary(),
        symphonyscript_core::LimitOverrides {
            inlined_calls: Some(4),
            ..symphonyscript_core::LimitOverrides::default()
        },
    )
    .expect("the vocabulary compiles");
    let h = "fn h(x): x + 1\n";
    let body = |read: &str| {
        format!("fn r(n): if n < 1 then i32({read}) else r(n - 1) + h(h(h(h(h(n)))))\n")
    };
    let rows: Vec<InlineRow> = vec![
        (
            vec![
                "input g = 2.0\n".into(),
                "let x = if false then g else 3.0\n".into(),
                h.into(),
                body("x"),
                "Clip c:\n    Note/(60 + r(3))\n".into(),
            ],
            vec![(Code::InlineLimit, "r(3)")],
        ),
        (
            vec![
                "Clip v | gain=3\n".into(),
                h.into(),
                body("v.gain"),
                "Clip c:\n    Note/(60 + r(3))\n".into(),
            ],
            vec![(Code::InlineLimit, "r(3)")],
        ),
        (
            vec![
                "input g = 2.0\n".into(),
                "Clip w | gain=g\n".into(),
                h.into(),
                body("w.gain"),
                "Clip c:\n    Note/(60 + r(3))\n".into(),
            ],
            vec![(Code::ReactiveRecursion, "r(3)")],
        ),
    ];
    for (items, expected) in rows {
        let mut expected: Vec<(Code, String)> = expected
            .iter()
            .map(|(code, text)| (*code, (*text).to_string()))
            .collect();
        expected.sort();
        let items: Vec<&str> = items.iter().map(String::as_str).collect();
        for order in Orders::ForwardAndReversed.of(&items) {
            let text = order.concat();
            assert_eq!(support::error_texts(&four, &text), expected, "{text}");
        }
    }
}
