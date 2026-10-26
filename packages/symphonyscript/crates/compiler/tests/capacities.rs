mod support;

use symphonyscript::Code;

const CELLS: &str = "input a: i32[..5] = []\ninput i = 1\ninput flag = false\ninput note = 60\n";

fn codes(text: &str) -> Vec<Code> {
    support::checked(text)
        .iter()
        .map(|diagnostic| diagnostic.code)
        .collect()
}

fn exceeds(prefix: &str, value: &str, capacity: u32) -> bool {
    let text = format!(
        "{CELLS}{prefix}let probe: i32[..{capacity}] = {value}\nClip c:\n    Note/(probe[0])\n"
    );
    let found = codes(&text);
    assert!(
        found.iter().all(|code| *code == Code::CapacityExceeded),
        "{text}{found:?}"
    );
    !found.is_empty()
}

fn assert_capacity(prefix: &str, value: &str, capacity: u32) {
    assert!(
        !exceeds(prefix, value, capacity),
        "{prefix}{value} holds more than {capacity}"
    );
    assert!(
        capacity == 1 || exceeds(prefix, value, capacity - 1),
        "{prefix}{value} holds fewer than {capacity}"
    );
}

#[test]
fn each_row_of_the_capacity_table_gives_its_capacity() {
    let rows: &[(&str, &str, u32)] = &[
        ("", "[1, 2, 3]", 3),
        ("", "a", 5),
        ("let b: i32[..6] = a\n", "b", 6),
        ("", "a + [1, 2]", 7),
        ("", "a + a", 10),
        ("", "a[^3..]", 3),
        ("", "a[1..]", 4),
        ("", "a[..2]", 2),
        ("", "a[1..^1]", 3),
        ("", "a[2..^1]", 2),
        ("", "a[i..]", 5),
        ("", "a[..i]", 5),
        ("", "a[^i..]", 5),
        ("", "[1, 2, 3, 4][i..]", 4),
        ("", "(a + a)[^3..]", 3),
        ("", "if flag then a else [1, 2, 3, 4, 5, 6, 7]", 7),
        ("", "if flag then [1] else a + a", 10),
        (
            "let m = match i:\n    0: a\n    1: [1, 2, 3, 4, 5, 6]\n    else: [1]\n",
            "m",
            6,
        ),
        ("state r = (r{1, []} + [note])[^8..]\n", "r", 8),
        ("state r = (r{1, []} + [note])[..8]\n", "r", 8),
        ("state r = (r{1, []} + a)[^4..]\n", "r", 4),
        ("state r = (r{1, [1, 2, 3]} + [note])[^2..]\n", "r", 2),
    ];
    for (prefix, value, capacity) in rows {
        assert_capacity(prefix, value, *capacity);
    }
}

#[test]
fn a_state_holds_a_fixed_array_when_its_initializer_keeps_its_fallbacks_length() {
    let fixed = "state s = (s{1, [0, 0]} + [note])[1..]\n";
    assert_eq!(
        codes(&format!("{CELLS}{fixed}Clip c:\n    Note/(s[1])\n")),
        vec![]
    );
    assert_eq!(
        codes(&format!("{CELLS}{fixed}Clip c:\n    Note/(s[2])\n")),
        vec![Code::IndexOutOfRange]
    );
    let variable = "state s = (s{1, [0]} + [note])[^2..]\n";
    assert_eq!(
        codes(&format!("{CELLS}{variable}Clip c:\n    Note/(s[5])\n")),
        vec![]
    );
}

#[test]
fn a_state_with_no_capacity_is_an_error() {
    for state in [
        "state s = s{1, [0, 0]} + [note]\n",
        "state s = s{1, []} + a\n",
        "state s = (s{1, []} + a)[1..]\n",
        "state p = q{1, []} + [note]\nstate q = p{1, []} + [note]\n",
    ] {
        let found = codes(&format!("{CELLS}{state}Clip c:\n    Note\n"));
        assert!(
            !found.is_empty() && found.iter().all(|code| *code == Code::UnboundedState),
            "{state}{found:?}"
        );
    }
}

#[test]
fn a_state_that_keeps_its_fallbacks_length_while_reading_a_variable_state_is_an_error() {
    assert_eq!(
        codes(&format!(
            "{CELLS}state p = q{{1, []}} + [note]\nstate q = p{{1, []}}\nClip c:\n    Note/(p[0])\n"
        )),
        vec![Code::VariableArray]
    );
}

#[test]
fn a_state_annotation_states_its_array() {
    assert_eq!(
        codes(&format!(
            "{CELLS}state s: i32[2] = a[..2]\nClip c:\n    Note/(s[0])\n"
        )),
        vec![Code::VariableArray]
    );
    assert_eq!(
        codes(&format!(
            "{CELLS}state s: i32[..3] = (s{{1, []}} + [note])[^8..]\nClip c:\n    Note/(s[0])\n"
        )),
        vec![Code::CapacityExceeded]
    );
    assert_eq!(
        codes(&format!(
            "{CELLS}state s: i32[..8] = (s{{1, []}} + [note])[^8..]\nClip c:\n    Note/(s[0])\n"
        )),
        vec![]
    );
}

#[test]
fn history_reads_form_no_cycle_and_a_present_read_of_itself_does() {
    assert_eq!(
        codes(&format!(
            "{CELLS}state p = q{{1, 0}} + note\nstate q = p{{1, 0}} + 1\nClip c:\n    Note/(p + q)\n"
        )),
        vec![]
    );
    assert_eq!(
        codes(&format!("{CELLS}state p = p + 1\nClip c:\n    Note/p\n")),
        vec![Code::DeclarationCycle]
    );
}

#[test]
fn a_state_s_own_history_reads_count_as_their_fallbacks_for_its_kind() {
    assert_eq!(
        codes(&format!(
            "{CELLS}state s = s{{1, 0.5}} + 1\nClip c | gain=s:\n    Note\n"
        )),
        vec![]
    );
    assert_eq!(
        codes(&format!(
            "{CELLS}state s = s{{1, true}} or flag\nClip c:\n    when s:\n        Note\n"
        )),
        vec![]
    );
}

#[test]
fn a_history_read_of_an_input_takes_its_initial_value_as_its_fallback() {
    assert_eq!(
        codes(&format!(
            "{CELLS}state rising = flag and not flag{{1}}\nClip c:\n    when rising:\n        Note\n"
        )),
        vec![]
    );
}

type OrderedRow<'a> = (&'a [&'a str], &'a [(Code, &'a str, usize)]);

fn in_every_order(items: &[&str]) -> Vec<String> {
    support::permutations(items)
        .into_iter()
        .map(|order| order.concat())
        .collect()
}

#[test]
fn a_history_reads_fallback_takes_the_cells_type() {
    let rows: &[OrderedRow] = &[
        (
            &[
                "input w = Wave::Saw\n",
                "state s = w{1, Sine}\n",
                "Clip c:\n    Note when s is Saw\n",
            ],
            &[],
        ),
        (
            &[
                "input w = Wave::Saw\n",
                "let k = w\n",
                "state s2 = k{1, Sine}\n",
                "Clip c:\n    Note when s2 is Saw\n",
            ],
            &[],
        ),
        (
            &[
                "input x = 1\n",
                "state s = x{1, 0.5}\n",
                "Clip c:\n    Note/(s)\n",
            ],
            &[(Code::TypeMismatch, "0.5", 0)],
        ),
        (
            &[
                "input w = Wave::Saw\n",
                "state s = w{1}\n",
                "Osc o | wave=s\n",
            ],
            &[],
        ),
        (
            &[
                "input x = 1\n",
                "input y = 3000000000\n",
                "let l = y + 0\n",
                "state s = x{1, l}\n",
                "Clip c:\n    Note/(s)\n",
            ],
            &[],
        ),
    ];
    for (items, expected) in rows {
        for text in in_every_order(items) {
            support::expect_codes(&text, expected);
        }
    }
}

#[test]
fn an_annotation_holds_only_an_initializer_of_its_rows() {
    let cells = [
        "input a: i32[..5] = []\n",
        "input g: i32[3][..10] = []\n",
        "input note = 60\n",
    ];
    let rows: &[(&str, &str, &[Code])] = &[
        (
            "let probe: i32[2][..5] = a\n",
            "Clip c:\n    Note/(probe[0][0])\n",
            &[Code::TypeMismatch],
        ),
        (
            "let probe: i32[..10] = g\n",
            "Clip c:\n    Note/(probe[0])\n",
            &[Code::TypeMismatch],
        ),
        (
            "let probe: i32[2][..10] = g\n",
            "Clip c:\n    Note/(probe[0][2])\n",
            &[Code::TypeMismatch],
        ),
        (
            "state s: i32[2][..5] = a\n",
            "Clip c:\n    Note/(s[0][0])\n",
            &[Code::TypeMismatch],
        ),
        (
            "state s: i32[2] = [note, note, note]\n",
            "Clip c:\n    Note/(s[0])\n",
            &[Code::TypeMismatch],
        ),
        ("let b: i32[..6] = a\n", "Clip c:\n    Note/(b[0])\n", &[]),
        (
            "let probe: i32[3][..11] = g + [[1, 2, 3]]\n",
            "Clip c:\n    Note/(probe[0][2])\n",
            &[],
        ),
        (
            "state s: i32[..8] = (s{1, []} + [note])[^8..]\n",
            "Clip c:\n    Note/(s[0])\n",
            &[],
        ),
        (
            "let probe: f32[..5] = a\n",
            "Clip c | gain=(probe[0]):\n    Note\n",
            &[],
        ),
    ];
    for (declaration, clip, expected) in rows {
        let mut items: Vec<&str> = cells.to_vec();
        items.push(declaration);
        items.push(clip);
        for text in in_every_order(&items) {
            assert_eq!(codes(&text), expected.to_vec(), "{text}");
        }
    }
}
