mod support;

use support::{Row, signature_vocabulary};
use symphonyscript::{Code, Compiler, Severity, Source};
use symphonyscript_fixtures::native_vocabulary;

fn codes(text: &str) -> Vec<Code> {
    let compiler = Compiler::new(&native_vocabulary()).expect("the native vocabulary compiles");
    compiler
        .check(&[Source {
            path: "natives.sym".into(),
            text: text.into(),
        }])
        .iter()
        .filter(|diagnostic| diagnostic.severity() == Severity::Error)
        .map(|diagnostic| diagnostic.code)
        .collect()
}

#[test]
fn a_native_call_is_typed_by_its_signature() {
    let rows: &[(&str, &[Code])] = &[
        ("Clip c:\n    Note/(checksum(\"GET\"))\n", &[]),
        ("Clip c:\n    Note/(checksum([u8(1), u8(2)]))\n", &[]),
        ("Clip c | gain=(text_length(\"ab\")):\n    Note\n", &[]),
        ("Clip c | sequence=(mirror([1, 2, 3])):\n    Note\n", &[]),
        ("Clip c:\n    Note/(checksum(1))\n", &[Code::TypeMismatch]),
        ("Clip c:\n    Note/(mirror(1))\n", &[Code::TypeMismatch]),
        ("Clip c:\n    Note/(checksum())\n", &[Code::ArgumentCount]),
        (
            "Clip c:\n    Note/(text_length(\"a\", \"b\"))\n",
            &[Code::ArgumentCount],
        ),
        (
            "Clip c:\n    Note/(checksum(text=\"x\"))\n",
            &[Code::NamedBuiltinArguments],
        ),
        (
            "Clip c:\n    Note/(mirror([1, 2]))\n",
            &[Code::TypeMismatch],
        ),
    ];
    for (text, expected) in rows {
        assert_eq!(codes(text), *expected, "{text}");
    }
}

#[test]
fn an_array_parameter_of_a_fixed_length_or_a_capacity_bounds_its_argument() {
    let rows: &[(&str, &[Code])] = &[
        ("Clip c:\n    Note/(pair_sum([1, 2]))\n", &[]),
        (
            "Clip c:\n    Note/(pair_sum([1, 2, 3]))\n",
            &[Code::TypeMismatch],
        ),
        (
            "input held: i32[..2] = []\nClip c:\n    Note/(pair_sum(held))\n",
            &[Code::VariableArray],
        ),
        (
            "Clip c:\n    Note/(pair_sum([[1], [2]]))\n",
            &[Code::TypeMismatch],
        ),
        ("Clip c:\n    Note/(first_of([1, 2, 3, 4]))\n", &[]),
        (
            "Clip c:\n    Note/(first_of([1, 2, 3, 4, 5]))\n",
            &[Code::CapacityExceeded],
        ),
        (
            "input held: i32[..4] = []\nClip c:\n    Note/(first_of(held))\n",
            &[],
        ),
        (
            "input held: i32[..5] = []\nClip c:\n    Note/(first_of(held))\n",
            &[Code::CapacityExceeded],
        ),
    ];
    for (text, expected) in rows {
        assert_eq!(codes(text), *expected, "{text}");
    }
}

#[test]
fn a_native_call_with_a_cell_runs_at_runtime_unless_it_takes_a_string() {
    let rows: &[(&str, &[Code])] = &[
        (
            "input i = 1\nClip c:\n    Note/(checksum([u8(i), u8(2)]))\n",
            &[],
        ),
        (
            "input held: i32[..4] = []\nClip c | sequence=(mirror(held)):\n    Note\n",
            &[],
        ),
        (
            "input i = 1\nClip c:\n    Note/(text_length(i))\n",
            &[Code::CompileTimeNative],
        ),
        (
            "input i = 1\nClip c:\n    Note/(text_length(\"a{i}\"))\n",
            &[Code::ReactiveString],
        ),
    ];
    for (text, expected) in rows {
        assert_eq!(codes(text), *expected, "{text}");
    }
}

#[test]
fn a_variable_array_attribute_accepts_a_value_up_to_its_capacity_and_refuses_a_larger_one() {
    let rows: &[(&str, &[Code])] = &[
        (
            "Clip c | sequence=[1, 2, 3, 4, 5, 6, 7, 8]:\n    Note\n",
            &[],
        ),
        (
            "Clip c | sequence=[1, 2, 3, 4, 5, 6, 7, 8, 9]:\n    Note\n",
            &[Code::CapacityExceeded],
        ),
        ("Clip c | sequence=[]:\n    Note\n", &[]),
        (
            "input held: i32[..8] = []\nClip c | sequence=held:\n    Note\n",
            &[],
        ),
        (
            "input held: i32[..9] = []\nClip c | sequence=held:\n    Note\n",
            &[Code::CapacityExceeded],
        ),
        (
            "input held: i32[..4] = []\nClip c | sequence=(held + held):\n    Note\n",
            &[],
        ),
        (
            "input held: i32[..4] = []\nClip c | sequence=(held + held + [1]):\n    Note\n",
            &[Code::CapacityExceeded],
        ),
        (
            "input held: i32[..20] = []\nClip c | sequence=(held[^8..]):\n    Note\n",
            &[],
        ),
        ("Clip c | sequence=1:\n    Note\n", &[Code::TypeMismatch]),
        (
            "Clip c | sequence=[[1], [2]]:\n    Note\n",
            &[Code::TypeMismatch],
        ),
    ];
    for (text, expected) in rows {
        assert_eq!(codes(text), *expected, "{text}");
    }
}

#[test]
fn a_native_calls_compile_time_arguments_are_typed_and_fit_by_its_signature() {
    let natives: &[Row] = &[
        (
            &["Clip c:\n    Note/(text_length(5))\n"],
            &[(Code::TypeMismatch, "5")],
        ),
        (
            &["Clip c:\n    Note/(pair_sum([1.5, 2]))\n"],
            &[(Code::TypeMismatch, "[1.5, 2]")],
        ),
        (
            &["Clip c:\n    Note/(pair_sum([true, false]))\n"],
            &[(Code::TypeMismatch, "[true, false]")],
        ),
        (
            &["Clip c:\n    Note/(checksum([300]))\n"],
            &[(Code::DoesNotFit, "300")],
        ),
        (
            &["Clip c:\n    Note/(text_length(true))\n"],
            &[(Code::TypeMismatch, "true")],
        ),
        (
            &["Clip c:\n    Note/(checksum([1, 2]) + text_length(1.5))\n"],
            &[(Code::TypeMismatch, "1.5")],
        ),
        (
            &[
                "Clip c:\n    Note/(checksum(\"GET\") + pair_sum([1, 2]) + first_of([1, 2, 3, 4]) + text_length(\"abc\"))\n",
            ],
            &[],
        ),
        (
            &[
                "fn f(n): if n < 1 then text_length(5) else f(n - 1)\n",
                "Clip c:\n    Note/(f(2))\n",
            ],
            &[(Code::TypeMismatch, "5")],
        ),
        (
            &["Clip c:\n    for i in 0..checksum([300]):\n        Note\n"],
            &[(Code::DoesNotFit, "300")],
        ),
        (
            &["input f = 1.5\n", "Clip c:\n    Note/(pair_sum([f, 2]))\n"],
            &[(Code::TypeMismatch, "[f, 2]")],
        ),
    ];
    support::assert_rows_in_every_order(&native_vocabulary(), natives);
    let plain: &[Row] = &[
        (
            &["Clip c:\n    Note/(plain(1.5))\n"],
            &[(Code::TypeMismatch, "1.5")],
        ),
        (
            &["Clip c:\n    Note/(plain(true))\n"],
            &[(Code::TypeMismatch, "true")],
        ),
        (
            &["Clip c:\n    Note/(plain(\"a\"))\n"],
            &[(Code::TypeMismatch, "\"a\"")],
        ),
        (&["Clip c:\n    Note/(plain(2))\n"], &[]),
    ];
    support::assert_rows_in_every_order(&signature_vocabulary(), plain);
}

#[test]
fn a_native_argument_in_a_unit_family_is_an_error_at_the_argument() {
    let rows: &[Row] = &[
        (
            &["Clip c:\n    Note/(plain(2bars))\n"],
            &[(Code::UnitOperation, "2bars")],
        ),
        (
            &["Clip c:\n    Note | length=(half(2bars))\n"],
            &[(Code::UnitOperation, "2bars")],
        ),
        (
            &["Clip c:\n    Note/(cap4([1bars, 2]))\n"],
            &[(Code::UnitOperation, "[1bars, 2]")],
        ),
        (
            &["input d = 2bars\n", "Clip c:\n    Note/(plain(d))\n"],
            &[(Code::UnitOperation, "d")],
        ),
        (
            &["Clip c:\n    Note/(plain(2))\n    Note | length=(half(1.0))\n"],
            &[],
        ),
    ];
    support::assert_rows_in_every_order(&signature_vocabulary(), rows);
}

#[test]
fn every_one_dimensional_u8_parameter_takes_a_compile_time_string_as_its_bytes() {
    let rows: &[Row] = &[
        (&["Clip c:\n    Note/(fixed3(\"GET\"))\n"], &[]),
        (
            &["Clip c:\n    Note/(fixed3(\"GETX\"))\n"],
            &[(Code::TypeMismatch, "\"GETX\"")],
        ),
        (
            &["Clip c:\n    Note/(fixed3([1, 2, 3, 4]))\n"],
            &[(Code::TypeMismatch, "[1, 2, 3, 4]")],
        ),
    ];
    support::assert_rows_in_every_order(&signature_vocabulary(), rows);
    support::assert_rows_in_every_order(
        &native_vocabulary(),
        &[(&["Clip c:\n    Note/(checksum(\"GET\"))\n"], &[])],
    );
}

#[test]
fn a_capacity_of_result_holds_at_most_its_parameters_capacity_in_rows() {
    let rows: &[Row] = &[
        (
            &["Clip c | rows=(shrink([[1, 2, 3], [4, 5, 6]])):\n    Note\n"],
            &[],
        ),
        (
            &["Clip c:\n    Note/(shrink([[1, 2, 3], [4, 5, 6]])[1][1])\n"],
            &[],
        ),
        (
            &["Clip c:\n    Note/(shrink([[1, 2, 3], [4, 5, 6]])[2][0])\n"],
            &[(Code::IndexOutOfRange, "2")],
        ),
        (
            &[
                "input xs: i32[..2] = []\n",
                "Clip c | three=(squeeze(xs)):\n    Note\n",
            ],
            &[(Code::CapacityExceeded, "squeeze(xs)")],
        ),
    ];
    support::assert_rows_in_every_order(&signature_vocabulary(), rows);
    support::assert_rows_in_every_order(
        &native_vocabulary(),
        &[
            (
                &["Clip c:\n    Note/(mirror([5, 6, 7])[2] + len(mirror([5, 6, 7])))\n"],
                &[],
            ),
            (
                &["Clip c:\n    Note/(mirror([5, 6, 7])[3])\n"],
                &[(Code::IndexOutOfRange, "3")],
            ),
        ],
    );
}
