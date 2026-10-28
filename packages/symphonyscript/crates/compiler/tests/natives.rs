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
            "input n = 0\nClip c:\n    Note/(tag(\"abc\", n))\n",
            &[Code::CompileTimeNative],
        ),
        ("Clip c:\n    Note/(tag(\"abc\", 2))\n", &[]),
        (
            "input i = 1\nClip c:\n    Note/(text_length(\"a{i}\"))\n",
            &[Code::ReactiveString],
        ),
    ];
    for (text, expected) in rows {
        assert_eq!(codes(text), *expected, "{text}");
    }
    let clauses: &[Row] = &[
        (
            &["input n = 0\n", "Clip c:\n    Note/(tag(\"abc\", n))\n"],
            &[(Code::CompileTimeNative, "tag(\"abc\", n)")],
        ),
        (&["Clip c:\n    Note/(tag(\"abc\", 2))\n"], &[]),
        (
            &[
                "input n = 0\n",
                "Clip c:\n    Note/(text_length(label(n)))\n",
            ],
            &[(Code::CompileTimeNative, "label(n)")],
        ),
        (&["Clip c:\n    Note/(text_length(label(2)))\n"], &[]),
    ];
    support::assert_rows_in_every_order(&native_vocabulary(), clauses);
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
            &[(Code::FloatToInteger, "f")],
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

#[test]
fn a_capacity_of_result_over_a_fixed_array_has_its_length_as_its_capacity_whether_or_not_it_reads_a_cell()
 {
    let probe = "Clip c:\n    Note/(probe[0][0])\n";
    let arguments: [(&[&str], &str, &str); 3] = [
        (
            &[],
            "[[1, 2, 3], [4, 5, 6]]",
            "[[1, 2, 3], [4, 5, 6], [7, 8, 9]]",
        ),
        (
            &["input x = 1\n"],
            "[[x, 2, 3], [4, 5, 6]]",
            "[[x, 2, 3], [4, 5, 6], [7, 8, 9]]",
        ),
        (&["input r: i32[3][..2] = []\n"], "r", "r"),
    ];
    let vocabulary = signature_vocabulary();
    for (cells, two, three) in arguments {
        let wide = if three == "r" {
            "input r: i32[3][..3] = []\n".to_string()
        } else {
            cells.concat()
        };
        let cases = [
            (
                cells.concat(),
                format!("let probe: i32[2][..1] = shrink({two})\n"),
                true,
            ),
            (
                cells.concat(),
                format!("let probe: i32[2][..2] = shrink({two})\n"),
                false,
            ),
            (
                wide.clone(),
                format!("Clip k | rows=(shrink({three})):\n    Note\n"),
                true,
            ),
            (
                cells.concat(),
                format!("Clip k | rows=(shrink({two})):\n    Note\n"),
                false,
            ),
            (
                cells.concat(),
                format!("state probe: i32[2][..1] = shrink({two})\n"),
                true,
            ),
            (
                cells.concat(),
                format!("state probe: i32[2][..2] = shrink({two})\n"),
                false,
            ),
            (
                cells.concat(),
                format!("let probe: i32[2][..3] = shrink({two}) + shrink({two})\n"),
                true,
            ),
            (
                cells.concat(),
                format!("let probe: i32[2][..4] = shrink({two}) + shrink({two})\n"),
                false,
            ),
            (
                cells.concat(),
                format!("let probe: i32[2][..1] = shrink({two})[..2]\n"),
                true,
            ),
            (
                cells.concat(),
                format!("let probe: i32[2][..1] = shrink({two})[..1]\n"),
                false,
            ),
        ];
        for (declared, value, exceeds) in cases {
            let uses = if value.starts_with("Clip") { "" } else { probe };
            let items: Vec<&str> = [declared.as_str(), value.as_str(), uses]
                .into_iter()
                .filter(|item| !item.is_empty())
                .collect();
            for order in support::permutations(&items) {
                let text = order.concat();
                let found: Vec<Code> = support::checked_with(&vocabulary, &text)
                    .iter()
                    .map(|diagnostic| diagnostic.code)
                    .collect();
                let expected = if exceeds {
                    vec![Code::CapacityExceeded]
                } else {
                    Vec::new()
                };
                assert_eq!(found, expected, "{text}");
            }
        }
    }
    let text = "input x = 1\nlet probe: i32[2][..1] = shrink([[x, 2, 3], [4, 5, 6]])\nClip c:\n    Note/(probe[0][0])\n";
    let found = support::checked_with(&vocabulary, text);
    assert!(
        found
            .iter()
            .all(|diagnostic| diagnostic.summary.contains("up to 2 elements")),
        "{found:?}"
    );
}

#[test]
fn a_cell_reading_string_at_a_u8_array_parameter_draws_one_reactive_string() {
    let input = "input n = 1\n";
    support::assert_rows_in_every_order(
        &native_vocabulary(),
        &[
            (
                &[input, "Clip c:\n    Note/(checksum(\"a{n}\"))\n"],
                &[(Code::ReactiveString, "n")],
            ),
            (&["Clip c:\n    Note/(checksum(\"GET\"))\n"], &[]),
        ],
    );
    support::assert_rows_in_every_order(
        &signature_vocabulary(),
        &[
            (
                &[input, "Clip c:\n    Note/(fixed3(\"a{n}b\"))\n"],
                &[(Code::ReactiveString, "n")],
            ),
            (&["Clip c:\n    Note/(fixed3(\"GET\"))\n"], &[]),
        ],
    );
}

#[test]
fn an_unset_variable_array_attribute_reads_as_the_fixed_array_of_its_default() {
    let unset = "Clip b:\n    Note\n";
    let seq = "Clip b | seq=[7, 8]:\n    Note\n";
    let sequence = "Clip b | sequence=[]:\n    Note\n";
    let joined = "Clip a | seq=b.seq + [1, 2, 3]:\n    Note\n";
    let rows: &[Row] = &[
        (&["Clip c:\n    Note/(len(b.sequence))\n", unset], &[]),
        (&["Clip c | sequence=b.sequence:\n    Note\n", unset], &[]),
        (
            &["Clip c:\n    Note/(d.sequence[0])\n", "Clip d:\n    Note\n"],
            &[(Code::IndexOutOfRange, "0")],
        ),
        (
            &[
                "Clip c:\n    Note/(d.sequence[0])\n",
                "Clip d | sequence=[]:\n    Note\n",
            ],
            &[(Code::IndexOutOfRange, "0")],
        ),
        (&["Clip a | gain=len(b.seq):\n    Note\n", unset], &[]),
        (&["Clip a | seq=b.seq:\n    Note\n", unset], &[]),
        (&["Clip a:\n    Note/(b.seq[1])\n", unset], &[]),
        (
            &["Clip a:\n    Note/(b.seq[7])\n", unset],
            &[(Code::IndexOutOfRange, "7")],
        ),
        (
            &["Clip a:\n    Note/(b.seq[7])\n", seq],
            &[(Code::IndexOutOfRange, "7")],
        ),
        (
            &["Clip a | gain=b.seq:\n    Note\n", unset],
            &[(Code::TypeMismatch, "b.seq")],
        ),
        (
            &["Clip a | gain=[7, 8]:\n    Note\n"],
            &[(Code::TypeMismatch, "[7, 8]")],
        ),
        (
            &[
                "Clip a | seq=b.seq:\n    Note/(len(b.rows) + b.rows[1][0])\n",
                unset,
            ],
            &[],
        ),
        (&["Clip a | seq=b.sequence:\n    Note\n", unset], &[]),
        (&["Clip a | seq=b.sequence:\n    Note\n", sequence], &[]),
        (
            &[joined, unset],
            &[(Code::CapacityExceeded, "b.seq + [1, 2, 3]")],
        ),
        (
            &[joined, seq],
            &[(Code::CapacityExceeded, "b.seq + [1, 2, 3]")],
        ),
        (
            &["Clip c | sequence=[1, 2]:\n    Note/(len(c.sequence))\n"],
            &[],
        ),
    ];
    support::assert_rows_in_every_order(&native_vocabulary(), rows);
    let compiler = Compiler::new(&native_vocabulary()).expect("the vocabulary compiles");
    for b in [unset, seq] {
        let found = support::errors(compiler.check(&[support::source(&format!("{joined}{b}"))]));
        let [diagnostic] = found.as_slice() else {
            panic!("{b}{found:?}");
        };
        assert!(
            diagnostic.summary.contains("up to 5 elements"),
            "{b}{diagnostic:?}"
        );
    }
}
