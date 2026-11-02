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
