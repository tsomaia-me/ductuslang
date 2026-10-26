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

fn ordered_codes(items: &[&str]) -> Vec<(String, Vec<Code>)> {
    in_every_order(items)
        .into_iter()
        .map(|text| {
            let found = codes(&text);
            (text, found)
        })
        .collect()
}

fn assert_ordered_capacity(items: &[&str], value: &str, capacity: u32) {
    let mut probes = vec![(capacity, vec![])];
    if capacity > 1 {
        probes.push((capacity - 1, vec![Code::CapacityExceeded]));
    }
    for (held, expected) in probes {
        let probe = format!("let probe: i32[..{held}] = {value}\n");
        let mut all: Vec<&str> = items.to_vec();
        all.push(&probe);
        all.push("Clip c:\n    Note/(probe[0])\n");
        for (text, found) in ordered_codes(&all) {
            assert_eq!(found, expected, "{text}");
        }
    }
}

fn assert_ordered_codes(items: &[&str], expected: &[Code]) {
    for (text, found) in ordered_codes(items) {
        assert_eq!(found, expected, "{text}");
    }
}

const NOTE: &str = "input note = 60\n";

#[test]
fn the_state_rules_see_the_history_reads_an_inlined_call_reaches() {
    assert_ordered_codes(
        &[
            NOTE,
            "fn f(x): x + [note]\n",
            "state s = f(s{1, []})\n",
            "Clip c:\n    Note/(s[0])\n",
        ],
        &[Code::UnboundedState],
    );
    let push = [
        NOTE,
        "fn push(xs, x): (xs + [x])[^8..]\n",
        "state recent = push(recent{1, []}, note)\n",
    ];
    assert_ordered_capacity(&push, "recent", 8);
    let keep = [
        NOTE,
        "fn keep(xs): xs[^8..]\n",
        "state s = keep(s{1, []} + [note])\n",
    ];
    assert_ordered_capacity(&keep, "s", 8);
    assert_ordered_codes(
        &[
            NOTE,
            "fn id(x): x\n",
            "state[1] s = id(s{2, 0}) + note\n",
            "Clip c:\n    Note/(s)\n",
        ],
        &[Code::HistoryDepth],
    );
    let step = [
        NOTE,
        "fn step(prev, x): prev * 0.5 + x\n",
        "state s = step(s{1, 0.0}, note)\n",
    ];
    let mut gain = step.to_vec();
    gain.push("Clip c | gain=s:\n    Note\n");
    assert_ordered_codes(&gain, &[]);
    let mut pitch = step.to_vec();
    pitch.push("Clip c:\n    Note/(s)\n");
    assert_ordered_codes(&pitch, &[Code::FloatToInteger]);
}

#[test]
fn a_states_fixed_array_is_settled_over_its_group() {
    let recent = "state recent = (recent{1, []} + [note])[^8..]\n";
    let last = [NOTE, recent, "state last = recent{1, []}[^1..]\n"];
    let mut read = last.to_vec();
    read.push("Clip c:\n    Note/(last[0])\n");
    assert_ordered_codes(&read, &[]);
    assert_ordered_capacity(&last, "last", 1);
    assert_ordered_capacity(
        &[NOTE, recent, "state older = recent{2, []}[..4]\n"],
        "older",
        4,
    );
    assert_ordered_capacity(
        &[
            NOTE,
            "state p = (q{1, []} + [note])[^1000000000..]\n",
            "state q = p{1, []}\n",
        ],
        "q",
        1_000_000_000,
    );
    let ring = "state ring = (ring{1, []} + [note])[^6..]\n";
    for echo in [
        "state[3] echo = ring{3, []}[^5..]\n",
        "state[3] echo = ring{3, [0]}[^5..]\n",
    ] {
        assert_ordered_codes(
            &[
                NOTE,
                ring,
                echo,
                "Clip c:\n    Note/(echo[len(echo) div 2])\n",
            ],
            &[],
        );
        assert_ordered_capacity(&[NOTE, ring, echo], "echo", 5);
    }
    assert_ordered_codes(
        &[
            NOTE,
            "state recent = recent{1, []} + [note]\n",
            "Clip c:\n    Note/(recent[0])\n",
        ],
        &[Code::UnboundedState],
    );
    assert_ordered_codes(
        &[
            NOTE,
            "state p = q{1, []} + [note]\n",
            "state q = p{1, []}\n",
            "Clip c:\n    Note/(p[0])\n    Note/(q[0])\n",
        ],
        &[Code::UnboundedState],
    );
}

#[test]
fn a_compile_time_index_outside_a_fixed_history_read_is_an_error() {
    let fixed = "state s = (s{1, [0, 0]} + [note])[1..]\n";
    let text = format!("{NOTE}{fixed}state t = s{{1, [0, 0]}}[5] + 1\nClip c:\n    Note/(t)\n");
    support::expect_codes(&text, &[(Code::IndexOutOfRange, "5", 0)]);
    for index in ["1", "^1"] {
        assert_ordered_codes(
            &[
                NOTE,
                fixed,
                &format!("state t = s{{1, [0, 0]}}[{index}] + 1\n"),
                "Clip c:\n    Note/(t)\n",
            ],
            &[],
        );
    }
    assert_ordered_codes(
        &[
            NOTE,
            fixed,
            "state t = s{1, [0, 0]}[5] + 1\n",
            "Clip c:\n    Note/(t)\n",
        ],
        &[Code::IndexOutOfRange],
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
fn a_history_read_counts_as_an_operand_of_its_cells_integer_width() {
    let wide = "Clip c:\n    Note when s > 3000000000\n";
    let rows: &[support::Row] = &[
        (
            &[
                "input x = 3000000000\n",
                "state s = x{1, 0}\n",
                "Clip c:\n    Note when s > 0\n",
            ],
            &[],
        ),
        (&["input x = 1\n", "state s = x{1, 0}\n", wide], &[]),
        (
            &[
                "input x = 1\n",
                "state total = total{1, 0} + x{1}\n",
                "Clip c:\n    Note when total > 3000000000\n",
            ],
            &[],
        ),
        (
            &[
                "input x = 1\n",
                "state a = x{1, 0}\n",
                "state b = a{1, 0}\n",
                "Clip c:\n    Note when b > 3000000000\n",
            ],
            &[],
        ),
        (
            &[
                "input x = 1\n",
                "let l = x + 1\n",
                "state s = l{1, 0}\n",
                wide,
            ],
            &[],
        ),
        (
            &[
                "input x = 1\n",
                "state a = b{1, 0} + x{1}\n",
                "state b = a{1, 0}\n",
                "Clip c:\n    Note when b > 3000000000\n",
            ],
            &[],
        ),
        (&["state s = s{1, 0i64} + 1\n", wide], &[]),
        (
            &["state s = s{1, 0} + 1\n", wide],
            &[(Code::DoesNotFit, "3000000000")],
        ),
        (
            &["input x: i32 = 1\n", "state s = x{1, 0}\n", wide],
            &[(Code::DoesNotFit, "3000000000")],
        ),
        (
            &[
                "input x: u8 = 1\n",
                "state s = x{1, 0}\n",
                "Clip c:\n    Note when s > 300\n",
            ],
            &[(Code::DoesNotFit, "300")],
        ),
        (
            &[
                "input x = 1\n",
                "state s = x{1, 0} + x\n",
                "Clip c:\n    Note when s > 0\n",
            ],
            &[],
        ),
        (
            &[
                "input gate = false\n",
                "input level = 0.0\n",
                "input note = 60\n",
                "state rising = gate and not gate{1}\n",
                "state held = if rising then level else held{1, 0.0}\n",
                "state recent = (recent{1, []} + [note])[^8..]\n",
            ],
            &[],
        ),
    ];
    support::assert_rows_in_every_order(&symphonyscript_fixtures::vocabulary(), rows);
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

#[test]
fn a_state_settled_as_a_fixed_array_is_used_as_a_fixed_array() {
    let fixed = "state s = (s{1, [0, 0]} + [note])[1..]\n";
    let variable = "state r = (r{1, []} + [note])[^8..]\n";
    let rows: &[OrderedRow] = &[
        (
            &[
                NOTE,
                fixed,
                "Clip c:\n    for i in 0..len(s):\n        Note/(s[i])\n",
            ],
            &[],
        ),
        (
            &[
                NOTE,
                fixed,
                "let p = s\n",
                "let probe: i32[2] = p\n",
                "Clip c:\n    Note/(probe[1])\n",
            ],
            &[],
        ),
        (
            &[
                NOTE,
                fixed,
                "let p = s + [note]\n",
                "let probe: i32[3] = p\n",
                "Clip c:\n    Note/(probe[2])\n",
            ],
            &[],
        ),
        (
            &[
                NOTE,
                "let p = s\n",
                "let probe: i32[2] = p\n",
                "state s = (z{1, [0, 0]} + [note])[1..]\n",
                "state z = p\n",
                "Clip c:\n    Note/(probe[1])\n",
            ],
            &[],
        ),
        (&[NOTE, variable, "Clip c:\n    Note/(len(r))\n"], &[]),
        (
            &[
                NOTE,
                variable,
                "Clip c:\n    for i in 0..len(r):\n        Note/(r[i])\n",
            ],
            &[(Code::ReactiveIterable, "len(r)", 0)],
        ),
    ];
    for (items, expected) in rows {
        for text in in_every_order(items) {
            support::expect_codes(&text, expected);
        }
    }
}

#[test]
fn a_compile_time_condition_chooses_its_arm_before_layout_and_capacity() {
    let cells = [
        "input note = 60\n",
        "input a: i32[..5] = []\n",
        "input held: i32[..10] = []\n",
        "input flag = false\n",
    ];
    let rows: &[OrderedRow] = &[
        (
            &[
                "let x = if true then a else [1, 2, 3, 4, 5, 6, 7]\n",
                "let probe: i32[..5] = x\n",
                "Clip c:\n    Note/(probe[0])\n",
            ],
            &[],
        ),
        (
            &[
                "let x = if false then a else [note, note]\n",
                "let probe: i32[2] = x\n",
                "Clip c:\n    Note/(probe[1])\n",
            ],
            &[],
        ),
        (
            &[
                "let probe: i32[..5] = if true then a else held\n",
                "Clip c:\n    Note/(probe[0])\n",
            ],
            &[],
        ),
        (
            &[
                "let k = 1\n",
                "let m = match k:\n    0: a\n    1: [1, 2, 3]\n    else: [1]\n",
                "let probe: i32[3] = m\n",
                "Clip c:\n    Note/(probe[2])\n",
            ],
            &[],
        ),
        (
            &[
                "state s = if true then a else [1, 2, 3, 4, 5, 6, 7]\n",
                "let probe: i32[..5] = s\n",
                "Clip c:\n    Note/(probe[0])\n",
            ],
            &[],
        ),
        (
            &[
                "let probe: i32[3] = if flag then [note, note] else [note, note, note]\n",
                "Clip c:\n    Note/(probe[0])\n",
            ],
            &[(
                Code::VariableArray,
                "let probe: i32[3] = if flag then [note, note] else [note, note, note]\n",
                0,
            )],
        ),
        (
            &[
                "let probe: i32[..5] = if true then a else [1.5, true]\n",
                "Clip c:\n    Note/(probe[0])\n",
            ],
            &[(Code::TypeMismatch, "[1.5, true]", 0)],
        ),
        (
            &[
                "let probe: i32[..5] = if true then a else [[1], 2]\n",
                "Clip c:\n    Note/(probe[0])\n",
            ],
            &[(Code::RaggedArray, "[[1], 2]", 0)],
        ),
    ];
    for (declarations, expected) in rows {
        let mut items: Vec<&str> = cells.to_vec();
        items.extend_from_slice(declarations);
        for text in in_every_order(&items) {
            support::expect_codes(&text, expected);
        }
    }
    let choices = [
        ("if flag then a else [1, 2, 3, 4, 5, 6, 7]", 7),
        ("if flag then [note, note] else [note, note, note]", 3),
    ];
    for (value, capacity) in choices {
        assert_ordered_capacity(&cells, value, capacity);
    }
}

#[test]
fn a_compile_time_match_on_a_bare_variant_chooses_its_arm_before_layout_and_capacity() {
    let cell = "input a: i32[..5] = []\n";
    let probe = "let probe: i32[..5] = m\n";
    let clip = "Clip c:\n    Note/(probe[0])\n";
    let rows: &[OrderedRow] = &[
        (
            &[
                "let w = Saw\n",
                "Osc o | wave=w\n",
                "let m = match w:\n    Saw: a\n    else: [1, 2, 3, 4, 5, 6, 7]\n",
            ],
            &[],
        ),
        (
            &[
                "let w = Square\n",
                "Osc o | wave=w\n",
                "let m = match w:\n    Saw: [1, 2, 3, 4, 5, 6, 7]\n    else: a\n",
            ],
            &[],
        ),
        (
            &[
                "let w = if true then Square else Saw\n",
                "Osc o | wave=w\n",
                "let m = match w:\n    Square: a\n    else: [1, 2, 3, 4, 5, 6, 7]\n",
            ],
            &[],
        ),
        (
            &[
                "let w = Square\n",
                "Osc o | wave=w\n",
                "let m = match w:\n    Saw: a\n    else: [1, 2, 3, 4, 5, 6, 7]\n",
            ],
            &[(Code::CapacityExceeded, "let probe: i32[..5] = m\n", 0)],
        ),
    ];
    for (declarations, expected) in rows {
        let mut items: Vec<&str> = vec![cell, probe, clip];
        items.extend_from_slice(declarations);
        for text in in_every_order(&items) {
            support::expect_codes(&text, expected);
        }
    }
    assert_ordered_capacity(
        &[cell, "input v = Wave::Saw\n"],
        "match v:\n    Saw: a\n    else: [1, 2, 3, 4, 5, 6, 7]",
        7,
    );
}

#[test]
fn a_group_of_states_takes_its_least_capacities_together() {
    let pair = [
        NOTE,
        "state p = (q{1, []} + [note])[^5..]\n",
        "state q = (p{1, []} + [note, note])[^7..]\n",
    ];
    assert_ordered_capacity(&pair, "p", 5);
    assert_ordered_capacity(&pair, "q", 7);
    let ring = [
        NOTE,
        "state s0 = (s2{1, []} + [note])[^8..]\n",
        "state s1 = (s0{1, []} + [note])[^8..]\n",
        "state s2 = (s1{1, []} + [note])[^8..]\n",
    ];
    for state in ["s0", "s1", "s2"] {
        assert_ordered_capacity(&ring, state, 8);
    }
}

#[test]
fn a_capacity_reaching_two_to_the_thirty_one_draws_length_limit_where_it_arises() {
    let big = "input big: i32[..2000000000] = []\n";
    let rows: &[OrderedRow] = &[
        (
            &[
                NOTE,
                big,
                "let x = big + big\n",
                "Clip c:\n    Note/(len(x))\n",
            ],
            &[(Code::LengthLimit, "big + big", 0)],
        ),
        (
            &[
                NOTE,
                big,
                "state s = big + big\n",
                "Clip c:\n    Note/(len(s))\n",
            ],
            &[(Code::LengthLimit, "big + big", 0)],
        ),
        (
            &[
                NOTE,
                big,
                "state s = big[^2000000000..] + big\n",
                "Clip c:\n    Note/(len(s))\n",
            ],
            &[(Code::LengthLimit, "big[^2000000000..] + big", 0)],
        ),
        (
            &[
                NOTE,
                big,
                "state r = (r{1, []} + [note])[^2147483648..]\n",
                "Clip c:\n    Note/(len(r))\n",
            ],
            &[(Code::LengthLimit, "(r{1, []} + [note])[^2147483648..]", 0)],
        ),
        (
            &[
                NOTE,
                big,
                "state r = (r{1, []} + [note])[^2147483647..]\n",
                "Clip c:\n    Note/(len(r))\n",
            ],
            &[(Code::LengthLimit, "r{1, []} + [note]", 0)],
        ),
        (
            &[
                NOTE,
                big,
                "let probe: i32[..5] = big + big\n",
                "Clip c:\n    Note/(len(probe))\n",
            ],
            &[(Code::LengthLimit, "big + big", 0)],
        ),
    ];
    for (items, expected) in rows {
        for text in in_every_order(items) {
            support::expect_codes(&text, expected);
        }
    }
    assert_ordered_codes(
        &[
            NOTE,
            big,
            "state r = r{1, []} + [note]\n",
            "Clip c:\n    Note/(len(r))\n",
        ],
        &[Code::UnboundedState],
    );
    assert_ordered_capacity(&[NOTE, big], "big + [1]", 2_000_000_001);
    assert_ordered_capacity(
        &[NOTE, "state r = (r{1, []} + [note])[^2147483646..]\n"],
        "r",
        2_147_483_646,
    );
}
