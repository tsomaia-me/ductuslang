use symphonyscript_fixtures::vocabulary;

use crate::compiler::Compiler;
use crate::source::Source;

#[test]
fn a_failing_row_prints_as_one_line_holding_its_program_and_its_whole_diagnostics() {
    let program = "fn f(x): x + true\nSession s | tempo=f(1)\n";
    let compiler = Compiler::new(&vocabulary()).expect("the vocabulary compiles");
    let diagnostics = compiler.check(&[Source {
        path: "test.sym".into(),
        text: program.into(),
    }]);
    assert_eq!(
        crate::pinned_row::pinned_row(program, &diagnostics),
        r#"pinned-row {"program": "fn f(x): x + true\nSession s | tempo=f(1)\n", "diagnostics": [{"code": "operand-type", "file": "test.sym", "span": [9, 17], "summary": "`+` takes two numbers or two strings", "help": "give the operator operands of the types it takes, or convert them, as in `i32(x)`", "related": [{"file": "test.sym", "span": [36, 40], "label": "in this call"}]}]}"#
    );
    assert_eq!(
        crate::pinned_row::pinned_counts(
            "let a = \"\t\"\n",
            &[("HashProbe".to_string(), vec![1, 2, 4])]
        ),
        r#"pinned-row {"program": "let a = \"\t\"\n", "diagnostics": [{"counter": "HashProbe", "counts": [1, 2, 4]}]}"#
    );
}

fn pinned_failures() -> Vec<String> {
    vec![
        "pinned-row {\"program\": \"a\"}\nfirst".to_string(),
        "pinned-row {\"program\": \"b\"}\nsecond".to_string(),
    ]
}

#[test]
fn a_row_failing_exactly_its_pinned_set_passes_in_any_order() {
    crate::pinned_row::assert_pinned_failures(
        &pinned_failures(),
        "pinned-row {\"program\": \"b\"}\npinned-row {\"program\": \"a\"}\n",
        488,
    );
}

#[test]
#[should_panic(expected = "the failing set differs from the 1 rows ledger 488 pins: 2 fail")]
fn a_row_failing_one_program_beyond_its_pinned_set_fails() {
    crate::pinned_row::assert_pinned_failures(
        &pinned_failures(),
        "pinned-row {\"program\": \"a\"}\n",
        488,
    );
}

#[test]
#[should_panic(expected = "the failing set differs from the 3 rows ledger 488 pins: 2 fail")]
fn a_row_failing_one_program_fewer_than_its_pinned_set_fails() {
    crate::pinned_row::assert_pinned_failures(
        &pinned_failures(),
        "pinned-row {\"program\": \"a\"}\npinned-row {\"program\": \"b\"}\npinned-row {\"program\": \"c\"}\n",
        488,
    );
}
