mod support;

use std::collections::BTreeMap;

use symphonyscript_fixtures::{Rows, covered};

use symphonyscript::{Code, Compiler, Source};
use symphonyscript_fixtures::native_vocabulary;

struct NativeResult {
    native: &'static str,
    ty: &'static str,
    zero: &'static str,
    dimensions: usize,
}

const RESULTS: [NativeResult; 5] = [
    NativeResult {
        native: "three",
        ty: "i32[3]",
        zero: "[0, 0, 0]",
        dimensions: 1,
    },
    NativeResult {
        native: "halves",
        ty: "f32[2]",
        zero: "[0.0, 0.0]",
        dimensions: 1,
    },
    NativeResult {
        native: "upto",
        ty: "i32[..5]",
        zero: "[]",
        dimensions: 1,
    },
    NativeResult {
        native: "grid",
        ty: "i32[2][3]",
        zero: "[[0, 0], [0, 0], [0, 0]]",
        dimensions: 2,
    },
    NativeResult {
        native: "octets",
        ty: "u8[..4]",
        zero: "[]",
        dimensions: 1,
    },
];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Position {
    Index,
    IndexFromEnd,
    SliceThenIndex,
    JoinLeft,
    JoinRight,
    Len,
    TopLet,
    State,
    FixedAttribute,
    VariableAttribute,
    InlinedArgument,
    NativeArgument,
    RowsAttribute,
    OpenArgument,
    FixedArgument,
    DeepIndex,
}

const POSITIONS: [Position; 12] = [
    Position::Index,
    Position::IndexFromEnd,
    Position::SliceThenIndex,
    Position::JoinLeft,
    Position::JoinRight,
    Position::Len,
    Position::TopLet,
    Position::State,
    Position::FixedAttribute,
    Position::VariableAttribute,
    Position::InlinedArgument,
    Position::NativeArgument,
];

const ARGUMENTS: [(&str, &str); 3] = [
    ("state n(1)\n", "n"),
    ("state n(1)\nlet r = n + 1\n", "r"),
    ("state n(1)\nstate q = n\n", "q"),
];

impl Position {
    fn reads_through_a_cell(self) -> bool {
        match self {
            Position::TopLet | Position::State | Position::InlinedArgument => true,
            Position::Index
            | Position::IndexFromEnd
            | Position::SliceThenIndex
            | Position::JoinLeft
            | Position::JoinRight
            | Position::Len
            | Position::FixedAttribute
            | Position::VariableAttribute
            | Position::NativeArgument
            | Position::RowsAttribute
            | Position::OpenArgument
            | Position::FixedArgument
            | Position::DeepIndex => false,
        }
    }

    fn operation_read(self, dimensions: usize) -> String {
        match self {
            Position::Index => format!("t[0]{}", "[0]".repeat(dimensions - 1)),
            Position::IndexFromEnd
            | Position::SliceThenIndex
            | Position::JoinLeft
            | Position::JoinRight
            | Position::Len
            | Position::TopLet
            | Position::State
            | Position::FixedAttribute
            | Position::VariableAttribute
            | Position::InlinedArgument
            | Position::NativeArgument
            | Position::RowsAttribute
            | Position::OpenArgument
            | Position::FixedArgument => "t".into(),
            Position::DeepIndex => format!("t[^1][^1]{}", "[0]".repeat(dimensions - 1)),
        }
    }

    fn items(self, value: &str, dimensions: usize) -> (Vec<String>, String, String) {
        let rest = "[0]".repeat(dimensions - 1);
        let literal = if dimensions == 1 { "[1]" } else { "[[1, 1]]" };
        let pitch = |read: String| (Vec::new(), String::new(), format!("Note/({read})"));
        match self {
            Position::Index => pitch(format!("{value}[0]{rest}")),
            Position::IndexFromEnd => pitch(format!("{value}[^1]{rest}")),
            Position::SliceThenIndex => pitch(format!("{value}[1..][0]{rest}")),
            Position::JoinLeft => pitch(format!("({value} + {literal})[0]{rest}")),
            Position::JoinRight => pitch(format!("({literal} + {value})[0]{rest}")),
            Position::Len => pitch(format!("({value}).size")),
            Position::TopLet => (
                vec![format!("let held = {value}\n")],
                String::new(),
                format!("Note/(held[0]{rest})"),
            ),
            Position::State => (
                vec![format!("state held = {value}\n")],
                String::new(),
                format!("Note/(held[0]{rest})"),
            ),
            Position::FixedAttribute => (Vec::new(), format!(" | trio={value}"), "Note".into()),
            Position::VariableAttribute => {
                (Vec::new(), format!(" | sequence={value}"), "Note".into())
            }
            Position::InlinedArgument => (
                vec![format!("fn pick(v): v[0]{rest}\n")],
                String::new(),
                format!("Note/(pick({value}))"),
            ),
            Position::NativeArgument => pitch(format!("first_of({value})")),
            Position::RowsAttribute => (Vec::new(), format!(" | rows={value}"), "Note".into()),
            Position::OpenArgument => pitch(format!("checksum({value})")),
            Position::FixedArgument => pitch(format!("pair_sum({value})")),
            Position::DeepIndex => pitch(format!("{value}[^1][^1]{rest}")),
        }
    }
}

fn program(
    position: Position,
    declarations: &[String],
    value: &str,
    dimensions: usize,
    reversed: bool,
) -> String {
    let (mut items, clause, read) = position.items(value, dimensions);
    items.splice(0..0, declarations.iter().cloned());
    items.push(format!("Song g:\n    Clip{clause}:\n        {read}\n"));
    if reversed {
        items.reverse();
    }
    items.concat()
}

type Found = BTreeMap<(Code, String), usize>;

fn found(compiler: &Compiler, text: &str, call: &str) -> Found {
    let diagnostics = support::errors(compiler.check(&[Source {
        path: "main.sym".into(),
        text: text.to_string(),
    }]));
    let mut found = Found::new();
    for diagnostic in diagnostics {
        let range = diagnostic.span.range;
        let at = text
            .get(range.start as usize..range.end as usize)
            .unwrap_or("?")
            .replace(call, "t");
        *found.entry((diagnostic.code, at)).or_default() += 1;
    }
    found
}

#[test]
fn a_native_call_that_runs_at_runtime_has_its_signatures_result_layout_wherever_it_stands() {
    let compiler = Compiler::new(&native_vocabulary()).expect("the native vocabulary compiles");
    let mut failures = Vec::new();
    let mut programs = 0;
    for result in &RESULTS {
        for position in POSITIONS {
            for (declared, argument) in ARGUMENTS {
                for reversed in [false, true] {
                    let call = format!("{}({argument})", result.native);
                    let declarations = vec![declared.to_string()];
                    let text = program(position, &declarations, &call, result.dimensions, reversed);
                    let mut twin_declarations = declarations.clone();
                    twin_declarations.push(format!("state t({}): {}\n", result.zero, result.ty));
                    let twin = program(
                        position,
                        &twin_declarations,
                        "t",
                        result.dimensions,
                        reversed,
                    );
                    let mut expected = Found::new();
                    let read = position.operation_read(result.dimensions);
                    for ((code, at), count) in found(&compiler, &twin, "t") {
                        let (code, at) =
                            if code == Code::FloatToInteger && !position.reads_through_a_cell() {
                                let at = if at == "t" { read.clone() } else { at };
                                (Code::TypeMismatch, at)
                            } else {
                                (code, at)
                            };
                        *expected.entry((code, at)).or_default() += count;
                    }
                    let found = found(&compiler, &text, &call);
                    programs += 1;
                    if found != expected {
                        failures.push(format!("{text}\n  found {found:?}\n  twin {expected:?}\n"));
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
    assert_eq!(programs, 360);
}

const SHAPE_POSITIONS: [Position; 4] = [
    Position::RowsAttribute,
    Position::OpenArgument,
    Position::FixedArgument,
    Position::DeepIndex,
];

fn shape_position_rows(rows: &Rows) -> Vec<String> {
    let compiler = Compiler::new(&native_vocabulary()).expect("the native vocabulary compiles");
    let mut failures = Vec::new();
    for (index, result) in RESULTS.iter().enumerate() {
        for (place, position) in SHAPE_POSITIONS.into_iter().enumerate() {
            for (form, (declared, argument)) in ARGUMENTS.into_iter().enumerate() {
                for reversed in [false, true] {
                    let open_integers = position == Position::OpenArgument
                        && result.dimensions == 1
                        && result.ty.starts_with("i32");
                    if open_integers || !rows.take(&[index, place, form, usize::from(reversed)]) {
                        continue;
                    }
                    let call = format!("{}({argument})", result.native);
                    let declarations = vec![declared.to_string()];
                    let text = program(position, &declarations, &call, result.dimensions, reversed);
                    let mut twin_declarations = declarations.clone();
                    twin_declarations.push(format!("state t({}): {}\n", result.zero, result.ty));
                    let twin = program(
                        position,
                        &twin_declarations,
                        "t",
                        result.dimensions,
                        reversed,
                    );
                    let mut expected = Found::new();
                    let read = position.operation_read(result.dimensions);
                    for ((code, at), count) in found(&compiler, &twin, "t") {
                        let (code, at) =
                            if code == Code::FloatToInteger && !position.reads_through_a_cell() {
                                let at = if at == "t" { read.clone() } else { at };
                                (Code::TypeMismatch, at)
                            } else {
                                (code, at)
                            };
                        *expected.entry((code, at)).or_default() += count;
                    }
                    let found = found(&compiler, &text, &call);
                    if found != expected {
                        failures.push(format!(
                            "310: {text}\n  found {found:?}\n  twin {expected:?}\n"
                        ));
                    }
                }
            }
        }
    }
    failures
}

#[test]
fn a_runtime_native_value_draws_at_an_array_position_the_one_diagnostic_its_input_twin_draws() {
    let failures = covered(shape_position_rows);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
#[ignore = "exhaustive"]
fn exhaustive_a_runtime_native_value_draws_at_an_array_position_the_one_diagnostic_its_input_twin_draws()
 {
    let failures = shape_position_rows(&Rows::Every);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
