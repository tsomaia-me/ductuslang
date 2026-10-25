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
