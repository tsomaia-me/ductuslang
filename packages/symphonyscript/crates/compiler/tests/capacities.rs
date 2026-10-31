mod support;

use symphonyscript::{Code, Compiler};
use symphonyscript_core::Vocabulary;
use symphonyscript_fixtures::vocabulary;

const CELLS: &str = "state a([]): i32[..5]\nstate i(1)\nstate flag(false)\nstate note(60)\n";

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
        ("", "a[^8..]", 5),
        ("", "a[..min(i, 4)]", 4),
        ("", "a[..min(i, 7)]", 5),
        ("", "a[max(i, 2)..]", 3),
        ("", "a[..a.size - 1]", 4),
        ("", "a[^min(i, 3)..]", 3),
        ("", "a[^max(i, 2)..]", 5),
        ("", "a[max(i, 3)..min(i, 4)]", 1),
        ("", "a[..max(min(i, 2), min(i, 3))]", 3),
        ("", "a[..2 - i]", 5),
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
        ("state r = (r.past(1, []) + [note])[^8..]\n", "r", 8),
        ("state r = (r.past(1, []) + [note])[..8]\n", "r", 8),
        ("state r = (r.past(1, []) + a)[^4..]\n", "r", 4),
        ("state r = (r.past(1, [1, 2, 3]) + [note])[^2..]\n", "r", 2),
    ];
    for (prefix, value, capacity) in rows {
        assert_capacity(prefix, value, *capacity);
    }
}

#[test]
fn a_state_holds_a_fixed_array_when_its_initializer_keeps_its_fallbacks_length() {
    let fixed = "state s = (s.past(1, [0, 0]) + [note])[1..]\n";
    assert_eq!(
        codes(&format!("{CELLS}{fixed}Clip c:\n    Note/(s[1])\n")),
        vec![]
    );
    assert_eq!(
        codes(&format!("{CELLS}{fixed}Clip c:\n    Note/(s[2])\n")),
        vec![Code::IndexOutOfRange]
    );
    let variable = "state s = (s.past(1, [0]) + [note])[^2..]\n";
    assert_eq!(
        codes(&format!("{CELLS}{variable}Clip c:\n    Note/(s[5])\n")),
        vec![]
    );
}

#[test]
fn a_state_with_no_capacity_is_an_error() {
    for state in [
        "state s = s.past(1, [0, 0]) + [note]\n",
        "state s = s.past(1, []) + a\n",
        "state s = (s.past(1, []) + a)[1..]\n",
        "state p = q.past(1, []) + [note]\nstate q = p.past(1, []) + [note]\n",
    ] {
        let found = codes(&format!("{CELLS}{state}Clip c:\n    Note\n"));
        assert!(
            !found.is_empty() && found.iter().all(|code| *code == Code::UnboundedState),
            "{state}{found:?}"
        );
    }
}

fn ordered_codes(orders: support::Orders, items: &[&str]) -> Vec<(String, Vec<Code>)> {
    in_orders(orders, items)
        .into_iter()
        .map(|text| {
            let found = codes(&text);
            (text, found)
        })
        .collect()
}

fn assert_ordered_capacity(items: &[&str], value: &str, capacity: u32) {
    assert_ordered_capacity_in(support::Orders::Every, items, value, capacity);
}

fn assert_ordered_capacity_in(orders: support::Orders, items: &[&str], value: &str, capacity: u32) {
    let mut probes = vec![(capacity, vec![])];
    if capacity > 1 {
        probes.push((capacity - 1, vec![Code::CapacityExceeded]));
    }
    for (held, expected) in probes {
        let probe = format!("let probe: i32[..{held}] = {value}\n");
        let mut all: Vec<&str> = items.to_vec();
        all.push(&probe);
        all.push("Clip c:\n    Note/(probe[0])\n");
        for (text, found) in ordered_codes(orders, &all) {
            assert_eq!(found, expected, "{text}");
        }
    }
}

fn assert_ordered_codes(items: &[&str], expected: &[Code]) {
    assert_ordered_codes_in(support::Orders::Every, items, expected);
}

fn assert_ordered_codes_in(orders: support::Orders, items: &[&str], expected: &[Code]) {
    for (text, found) in ordered_codes(orders, items) {
        assert_eq!(found, expected, "{text}");
    }
}

const NOTE: &str = "state note(60)\n";

fn the_state_rules_see_the_history_reads_an_inlined_call_reaches_in(orders: support::Orders) {
    assert_ordered_codes_in(
        orders,
        &[
            NOTE,
            "fn f(x): x + [note]\n",
            "state s = f(s.past(1, []))\n",
            "Clip c:\n    Note/(s[0])\n",
        ],
        &[Code::UnboundedState],
    );
    let push = [
        NOTE,
        "fn push(xs, x): (xs + [x])[^8..]\n",
        "state recent = push(recent.past(1, []), note)\n",
    ];
    assert_ordered_capacity_in(orders, &push, "recent", 8);
    let keep = [
        NOTE,
        "fn keep(xs): xs[^8..]\n",
        "state s = keep(s.past(1, []) + [note])\n",
    ];
    assert_ordered_capacity_in(orders, &keep, "s", 8);
    let step = [
        NOTE,
        "fn step(prev, x): prev * 0.5 + x\n",
        "state s = step(s.past(1, 0.0), note)\n",
    ];
    let mut gain = step.to_vec();
    gain.push("Clip c | gain=s:\n    Note\n");
    assert_ordered_codes_in(orders, &gain, &[]);
    let mut pitch = step.to_vec();
    pitch.push("Clip c:\n    Note/(s)\n");
    assert_ordered_codes_in(orders, &pitch, &[Code::FloatToInteger]);
}

#[test]
fn the_state_rules_see_the_history_reads_an_inlined_call_reaches() {
    the_state_rules_see_the_history_reads_an_inlined_call_reaches_in(
        support::Orders::ForwardAndReversed,
    );
}

#[test]
#[ignore = "exhaustive"]
fn exhaustive_the_state_rules_see_the_history_reads_an_inlined_call_reaches() {
    the_state_rules_see_the_history_reads_an_inlined_call_reaches_in(support::Orders::Every);
}

fn a_states_fixed_array_is_settled_over_its_group_in(orders: support::Orders) {
    let recent = "state recent = (recent.past(1, []) + [note])[^8..]\n";
    let last = [NOTE, recent, "state last = recent.past(1, [])[^1..]\n"];
    let mut read = last.to_vec();
    read.push("Clip c:\n    Note/(last[0])\n");
    assert_ordered_codes_in(orders, &read, &[]);
    assert_ordered_capacity_in(orders, &last, "last", 1);
    assert_ordered_capacity_in(
        orders,
        &[NOTE, recent, "state older = recent.past(2, [])[..4]\n"],
        "older",
        4,
    );
    assert_ordered_capacity_in(
        orders,
        &[
            NOTE,
            "state p = (q.past(1, []) + [note])[^1000000000..]\n",
            "state q = p.past(1, [])\n",
        ],
        "q",
        1_000_000_000,
    );
    let ring = "state ring = (ring.past(1, []) + [note])[^6..]\n";
    for echo in [
        "state echo = ring.past(3, [])[^5..]\n",
        "state echo = ring.past(3, [0])[^5..]\n",
    ] {
        assert_ordered_codes_in(
            orders,
            &[
                NOTE,
                ring,
                echo,
                "Clip c:\n    Note/(echo[echo.size div 2])\n",
            ],
            &[],
        );
        assert_ordered_capacity_in(orders, &[NOTE, ring, echo], "echo", 5);
    }
    assert_ordered_codes_in(
        orders,
        &[
            NOTE,
            "state recent = recent.past(1, []) + [note]\n",
            "Clip c:\n    Note/(recent[0])\n",
        ],
        &[Code::UnboundedState],
    );
    assert_ordered_codes_in(
        orders,
        &[
            NOTE,
            "state p = q.past(1, []) + [note]\n",
            "state q = p.past(1, [])\n",
            "Clip c:\n    Note/(p[0])\n    Note/(q[0])\n",
        ],
        &[Code::UnboundedState],
    );
}

#[test]
fn a_states_fixed_array_is_settled_over_its_group() {
    a_states_fixed_array_is_settled_over_its_group_in(support::Orders::ForwardAndReversed);
}

#[test]
#[ignore = "exhaustive"]
fn exhaustive_a_states_fixed_array_is_settled_over_its_group() {
    a_states_fixed_array_is_settled_over_its_group_in(support::Orders::Every);
}

#[test]
fn a_compile_time_index_outside_a_fixed_history_read_is_an_error() {
    let fixed = "state s = (s.past(1, [0, 0]) + [note])[1..]\n";
    let text = format!("{NOTE}{fixed}state t = s.past(1, [0, 0])[5] + 1\nClip c:\n    Note/(t)\n");
    support::expect_codes(&text, &[(Code::IndexOutOfRange, "5", 0)]);
    for index in ["1", "^1"] {
        assert_ordered_codes(
            &[
                NOTE,
                fixed,
                &format!("state t = s.past(1, [0, 0])[{index}] + 1\n"),
                "Clip c:\n    Note/(t)\n",
            ],
            &[],
        );
    }
    assert_ordered_codes(
        &[
            NOTE,
            fixed,
            "state t = s.past(1, [0, 0])[5] + 1\n",
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
            "{CELLS}state s: i32[..3] = (s.past(1, []) + [note])[^8..]\nClip c:\n    Note/(s[0])\n"
        )),
        vec![Code::CapacityExceeded]
    );
    assert_eq!(
        codes(&format!(
            "{CELLS}state s: i32[..8] = (s.past(1, []) + [note])[^8..]\nClip c:\n    Note/(s[0])\n"
        )),
        vec![]
    );
}

#[test]
fn history_reads_form_no_cycle_and_a_present_read_of_itself_does() {
    assert_eq!(
        codes(&format!(
            "{CELLS}state p = q.past(1, 0) + note\nstate q = p.past(1, 0) + 1\nClip c:\n    Note/(p + q)\n"
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
            "{CELLS}state s = s.past(1, 0.5) + 1\nClip c | gain=s:\n    Note\n"
        )),
        vec![]
    );
    assert_eq!(
        codes(&format!(
            "{CELLS}state s = s.past(1, true) or flag\nClip c:\n    when s:\n        Note\n"
        )),
        vec![]
    );
}

#[test]
fn a_history_read_of_an_input_takes_its_initial_value_as_its_fallback() {
    assert_eq!(
        codes(&format!(
            "{CELLS}state rising = flag and not flag.past(1)\nClip c:\n    when rising:\n        Note\n"
        )),
        vec![]
    );
}

type OrderedRow<'a> = (&'a [&'a str], &'a [(Code, &'a str, usize)]);

fn in_every_order(items: &[&str]) -> Vec<String> {
    in_orders(support::Orders::Every, items)
}

fn in_orders(orders: support::Orders, items: &[&str]) -> Vec<String> {
    orders
        .of(items)
        .into_iter()
        .map(|order| order.concat())
        .collect()
}

#[test]
fn a_history_reads_fallback_takes_the_cells_type() {
    let rows: &[OrderedRow] = &[
        (
            &[
                "state w(Wave::Saw)\n",
                "state s = w.past(1, Sine)\n",
                "Clip c:\n    Note when s is Saw\n",
            ],
            &[],
        ),
        (
            &[
                "state w(Wave::Saw)\n",
                "let k = w\n",
                "state s2 = k.past(1, Sine)\n",
                "Clip c:\n    Note when s2 is Saw\n",
            ],
            &[],
        ),
        (
            &[
                "state x(1)\n",
                "state s = x.past(1, 0.5)\n",
                "Clip c:\n    Note/(s)\n",
            ],
            &[(Code::TypeMismatch, "0.5", 0)],
        ),
        (
            &[
                "state w(Wave::Saw)\n",
                "state s = w.past(1)\n",
                "Osc o | wave=s\n",
            ],
            &[],
        ),
        (
            &[
                "state x(1)\n",
                "state y(3000000000)\n",
                "let l = y + 0\n",
                "state s = x.past(1, l)\n",
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

fn a_history_read_has_its_cells_kind_for_an_input_a_let_and_a_state_in(orders: support::Orders) {
    let rows: &[support::Row] = &[
        (
            &[
                "state x(1.5)\n",
                "let y = x * 2\n",
                "state s = y.past(1, 0)\n",
                "Clip c:\n    Note/(s)\n",
            ],
            &[(Code::FloatToInteger, "s")],
        ),
        (
            &[
                "state x(1.5)\n",
                "let y = x * 2\n",
                "state s = y.past(1, 0)\n",
                "Clip k | gain=s:\n    Note/(i32(s))\n",
            ],
            &[],
        ),
        (
            &[
                "state x(1.5)\n",
                "state f = x + 1\n",
                "state s = f.past(1, 0)\n",
                "Clip c:\n    Note/(s)\n",
            ],
            &[(Code::FloatToInteger, "s")],
        ),
        (
            &[
                "state x(1.5)\n",
                "state f = x + 1\n",
                "state s = f.past(1, 0)\n",
                "Clip k | gain=s:\n    Note/(i32(s))\n",
            ],
            &[],
        ),
        (
            &[
                "state x(1.5)\n",
                "state p = q.past(1, 0) + 1.5\n",
                "state q = p.past(1, 0)\n",
                "Clip c:\n    Note/(q)\n",
            ],
            &[(Code::FloatToInteger, "q")],
        ),
        (
            &[
                "state x(1.5)\n",
                "state p = q.past(1, 0) + 1.5\n",
                "state q = p.past(1, 0)\n",
                "Clip k | gain=q:\n    Note/(i32(q))\n",
            ],
            &[],
        ),
        (
            &[
                "state p = q.past(1, 0)\n",
                "state q = p.past(1, 0.5)\n",
                "Clip c:\n    Note/(p)\n",
            ],
            &[(Code::FloatToInteger, "p")],
        ),
        (
            &[
                "state p = q.past(1, 0)\n",
                "state q = p.past(1, 0.5)\n",
                "Clip k | gain=p:\n    Note/(i32(p))\n",
            ],
            &[],
        ),
        (
            &[
                "state x(1.5)\n",
                "let t = s * x\n",
                "state s = t.past(1, 0)\n",
                "Clip c:\n    Note/(s)\n",
            ],
            &[(Code::FloatToInteger, "s")],
        ),
        (
            &[
                "state x(1.5)\n",
                "let t = s * x\n",
                "state s = t.past(1, 0)\n",
                "Clip k | gain=s:\n    Note/(i32(s))\n",
            ],
            &[],
        ),
        (
            &[
                "state x(1.5)\n",
                "state s = x.past(1, 0)\n",
                "Clip c:\n    Note/(s)\n",
            ],
            &[(Code::FloatToInteger, "s")],
        ),
        (
            &[
                "state x(1.5)\n",
                "state s = x.past(1, 0)\n",
                "Clip k | gain=s:\n    Note/(i32(s))\n",
            ],
            &[],
        ),
        (
            &[
                "state p = q.past(1, 0)\n",
                "state q = p.past(1, 0.5)\n",
                "Clip c:\n    Note/(p)\n    Note/(q)\n",
            ],
            &[(Code::FloatToInteger, "p"), (Code::FloatToInteger, "q")],
        ),
        (
            &[
                "state x(1.5)\n",
                "state p = q.past(1, 0) + 1.5\n",
                "state q = p.past(1, 0)\n",
                "let m = match q:\n    1: 2\n    else: 3\n",
                "Clip c:\n    Note/(m)\n",
            ],
            &[(Code::MatchScrutinee, "q")],
        ),
        (
            &[
                "state x(1.5)\n",
                "state p = q.past(1, 0) + 1.5\n",
                "state q = p.past(1, 0)\n",
                "Clip c:\n    Note/([1, 2, 3][q])\n",
            ],
            &[(Code::IndexType, "q")],
        ),
        (
            &[
                "state x(1.5)\n",
                "state p = q.past(1, 0) + 1.5\n",
                "state q = p.past(1, 0)\n",
                "state r = q.past(1, 0) + 1\n",
                "Clip c:\n    Note/(r)\n",
            ],
            &[(Code::FloatToInteger, "r")],
        ),
        (
            &[
                "state x(1.5)\n",
                "let t = s * x\n",
                "state s = t.past(1, 0)\n",
                "let u = s + 1\n",
                "Clip c:\n    Note/(u)\n",
            ],
            &[(Code::FloatToInteger, "u")],
        ),
        (
            &[
                "state x(1)\n",
                "let y = x + 1\n",
                "state s = y.past(1, 0.5)\n",
                "Clip c:\n    Note/(s)\n",
            ],
            &[(Code::TypeMismatch, "0.5")],
        ),
        (
            &[
                "state gate(false)\n",
                "let h = not gate\n",
                "state s = h.past(1, 0)\n",
                "Clip c:\n    Note when s\n",
            ],
            &[(Code::TypeMismatch, "0")],
        ),
        (
            &[
                "state p = q.past(1, 0)\n",
                "state q = p.past(1, 0.5)\n",
                "Clip c:\n    Note | brightness=p\n",
            ],
            &[],
        ),
        (
            &[
                "state p = q.past(1, 0)\n",
                "state q = p.past(1, 0)\n",
                "Clip c:\n    Note/(p)\n",
            ],
            &[],
        ),
        (
            &[
                "state s = s.past(1, 0.5)\n",
                "Clip c:\n    Note | brightness=s\n",
            ],
            &[],
        ),
        (
            &[
                "state gate(false)\n",
                "state level(0.0)\n",
                "state note(60)\n",
                "state rising = gate and not gate.past(1)\n",
                "state held = if rising then level else held.past(1, 0.0)\n",
                "state recent = (recent.past(1, []) + [note])[^8..]\n",
                "Clip c:\n    Note/(recent[0]) | brightness=held\n    when rising:\n        Note\n",
            ],
            &[],
        ),
    ];
    support::assert_rows_in_orders(orders, &vocabulary(), rows);
}

#[test]
fn a_history_read_has_its_cells_kind_for_an_input_a_let_and_a_state() {
    a_history_read_has_its_cells_kind_for_an_input_a_let_and_a_state_in(
        support::Orders::ForwardAndReversed,
    );
}

#[test]
#[ignore = "exhaustive"]
fn exhaustive_a_history_read_has_its_cells_kind_for_an_input_a_let_and_a_state() {
    a_history_read_has_its_cells_kind_for_an_input_a_let_and_a_state_in(support::Orders::Every);
}

fn a_history_reads_provisional_kind_leaves_no_diagnostic_behind_in(orders: support::Orders) {
    let x = "state x(1)\n";
    let gate = "state gate(false)\n";
    let held = "state held([]): i32[..10]\n";
    let probe = "let probe = held[s..]\n";
    let read = "Clip c:\n    Note/(probe[0])\n";
    let rows: &[support::Row] = &[
        (
            &[
                x,
                held,
                "let y = x + 1\n",
                "state s = y.past(1, 0.5)\n",
                probe,
                read,
            ],
            &[(Code::TypeMismatch, "0.5")],
        ),
        (
            &[
                gate,
                "let y = not gate\n",
                "state s = y.past(1, 0)\n",
                "let b = not s\n",
                "Clip c:\n    Note when b\n",
            ],
            &[(Code::TypeMismatch, "0")],
        ),
        (
            &[
                x,
                "let y = x + 1\n",
                "state s = y.past(1, 0.5)\n",
                "let m = match s:\n    0: 1\n    else: 2\n",
                "Clip c:\n    Note/(m)\n",
            ],
            &[(Code::TypeMismatch, "0.5")],
        ),
        (
            &[
                x,
                held,
                "state p = q.past(1, 0.5)\n",
                "state q = x + 1\n",
                "let probe = held[p..]\n",
                read,
            ],
            &[(Code::TypeMismatch, "0.5")],
        ),
        (
            &[
                x,
                held,
                "state s = y.past(1, 0)\n",
                "let y = x + 1\n",
                probe,
                read,
            ],
            &[],
        ),
        (
            &[
                x,
                held,
                "let y = x * 0.5\n",
                "state s = y.past(1, 0.0)\n",
                probe,
                read,
            ],
            &[(Code::IndexType, "s")],
        ),
    ];
    support::assert_rows_in_orders(orders, &vocabulary(), rows);
}

#[test]
fn a_history_reads_provisional_kind_leaves_no_diagnostic_behind() {
    a_history_reads_provisional_kind_leaves_no_diagnostic_behind_in(
        support::Orders::ForwardAndReversed,
    );
}

#[test]
#[ignore = "exhaustive"]
fn exhaustive_a_history_reads_provisional_kind_leaves_no_diagnostic_behind() {
    a_history_reads_provisional_kind_leaves_no_diagnostic_behind_in(support::Orders::Every);
}

#[test]
fn a_history_read_of_a_let_whose_initializer_drew_an_error_draws_nothing_of_its_own() {
    let rows: &[support::Row] = &[
        (
            &[
                "state note(60)\n",
                "let t = zzz\n",
                "state s = (t.past(1, []) + [note])[^8..]\n",
                "Clip c:\n    Note/(s.size)\n",
            ],
            &[(Code::UnknownName, "zzz")],
        ),
        (
            &[
                "state note(60)\n",
                "let t = note + true\n",
                "state s = (t.past(1, []) + [note])[^8..]\n",
                "Clip c:\n    Note/(s.size)\n",
            ],
            &[(Code::OperandType, "note + true")],
        ),
        (
            &[
                "state note(60)\n",
                "let t = zzz + [note]\n",
                "state s = (t.past(1, []) + [note])[^8..]\n",
                "Clip c:\n    Note/(s.size)\n",
            ],
            &[(Code::UnknownName, "zzz")],
        ),
        (
            &[
                "let t = 3\n",
                "state s = t.past(1, 0)\n",
                "Clip c:\n    Note/(s)\n",
            ],
            &[(Code::HistoryTarget, "t.past(1, 0)")],
        ),
        (
            &[
                "let t = 3\n",
                "state s = t.past(1, 0)\n",
                "Clip c:\n    Note/(60 + s)\n",
            ],
            &[(Code::HistoryTarget, "t.past(1, 0)")],
        ),
    ];
    support::assert_rows_in_every_order(&vocabulary(), rows);
}

fn a_history_read_of_a_reactive_let_has_the_lets_layout_in(orders: support::Orders) {
    let held = "state held([]): i32[..10]\n";
    let played = "let played = held + [note]\n";
    assert_ordered_capacity_in(
        orders,
        &[
            held,
            NOTE,
            played,
            "state s = (played.past(1, []))[^20..]\n",
        ],
        "s",
        11,
    );
    assert_ordered_capacity_in(
        orders,
        &[
            held,
            NOTE,
            played,
            "let t = s[1..]\n",
            "state s = (t.past(1, []) + [note, note])[^8..]\n",
        ],
        "s",
        8,
    );
    assert_ordered_capacity_in(
        orders,
        &[NOTE, "state recent = (recent.past(1, []) + [note])[^8..]\n"],
        "recent",
        8,
    );
    let first = "Clip c:\n    Note/(s[0])\n";
    let rows: &[support::Row] = &[
        (
            &[held, NOTE, played, "state s = played.past(1, [])\n", first],
            &[],
        ),
        (
            &[
                held,
                NOTE,
                played,
                "fn keep(xs): xs[^8..]\n",
                "state s = keep(played.past(1, []))\n",
                first,
            ],
            &[],
        ),
        (
            &[
                held,
                NOTE,
                played,
                "state s = played.past(1, [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0])\n",
                first,
            ],
            &[(
                Code::CapacityExceeded,
                "[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]",
            )],
        ),
        (
            &[
                "state held([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]): i32[..10]\n",
                "Clip c:\n    Note/(held[0])\n",
            ],
            &[(Code::CapacityExceeded, "[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]")],
        ),
        (
            &[
                held,
                NOTE,
                played,
                "state s = played.past(1, [[1, 2]])\n",
                first,
            ],
            &[(Code::TypeMismatch, "played.past(1, [[1, 2]])")],
        ),
        (
            &[
                NOTE,
                "let pair = [note, note]\n",
                "state s = pair.past(1, [0, 0])[1..]\n",
                "Clip c:\n    Note/(s[1])\n",
            ],
            &[(Code::IndexOutOfRange, "1")],
        ),
        (
            &[
                NOTE,
                "let pair = [note, note]\n",
                "state s = pair.past(1, [0])\n",
                first,
            ],
            &[(Code::TypeMismatch, "pair.past(1, [0])")],
        ),
    ];
    support::assert_rows_in_orders(orders, &vocabulary(), rows);
}

#[test]
fn a_history_read_of_a_reactive_let_has_the_lets_layout() {
    a_history_read_of_a_reactive_let_has_the_lets_layout_in(support::Orders::ForwardAndReversed);
}

#[test]
#[ignore = "exhaustive"]
fn exhaustive_a_history_read_of_a_reactive_let_has_the_lets_layout() {
    a_history_read_of_a_reactive_let_has_the_lets_layout_in(support::Orders::Every);
}

#[test]
fn a_chain_of_slices_keeps_its_capacity_and_its_length() {
    assert_ordered_capacity(
        &[
            NOTE,
            "state s = (s.past(1, []) + [note])[^8..][^8..][^8..]\n",
        ],
        "s",
        8,
    );
    let rows: &[support::Row] = &[
        (
            &[
                NOTE,
                "state s = [note, note, note]\n",
                "let t = s[1..][1..][1..]\n",
                "Clip c:\n    Note/(t[0])\n",
            ],
            &[(Code::IndexOutOfRange, "0")],
        ),
        (
            &[
                NOTE,
                "state s = [note, note, note]\n",
                "let t = s[1..]\n",
                "Clip c:\n    Note/(t[2])\n",
            ],
            &[(Code::IndexOutOfRange, "2")],
        ),
        (
            &[
                NOTE,
                "state s = [note, note, note]\n",
                "let t = s[1..]\n",
                "Clip c:\n    Note/(t[1] + t.size)\n",
            ],
            &[],
        ),
    ];
    support::assert_rows_in_every_order(&vocabulary(), rows);
}

fn a_history_read_counts_as_an_operand_of_its_cells_integer_width_in(orders: support::Orders) {
    let wide = "Clip c:\n    Note when s > 3000000000\n";
    let rows: &[support::Row] = &[
        (
            &[
                "state x(3000000000)\n",
                "state s = x.past(1, 0)\n",
                "Clip c:\n    Note when s > 0\n",
            ],
            &[],
        ),
        (&["state x(1)\n", "state s = x.past(1, 0)\n", wide], &[]),
        (
            &[
                "state x(1)\n",
                "state total = total.past(1, 0) + x.past(1)\n",
                "Clip c:\n    Note when total > 3000000000\n",
            ],
            &[],
        ),
        (
            &[
                "state x(1)\n",
                "state a = x.past(1, 0)\n",
                "state b = a.past(1, 0)\n",
                "Clip c:\n    Note when b > 3000000000\n",
            ],
            &[],
        ),
        (
            &[
                "state x(1)\n",
                "let l = x + 1\n",
                "state s = l.past(1, 0)\n",
                wide,
            ],
            &[],
        ),
        (
            &[
                "state x(1)\n",
                "state a = b.past(1, 0) + x.past(1)\n",
                "state b = a.past(1, 0)\n",
                "Clip c:\n    Note when b > 3000000000\n",
            ],
            &[],
        ),
        (&["state s = s.past(1, 0i64) + 1\n", wide], &[]),
        (
            &["state s = s.past(1, 0) + 1\n", wide],
            &[(Code::DoesNotFit, "3000000000")],
        ),
        (
            &["state x(1): i32\n", "state s = x.past(1, 0)\n", wide],
            &[(Code::DoesNotFit, "3000000000")],
        ),
        (
            &[
                "state x(1): u8\n",
                "state s = x.past(1, 0)\n",
                "Clip c:\n    Note when s > 300\n",
            ],
            &[(Code::DoesNotFit, "300")],
        ),
        (
            &[
                "state x(1)\n",
                "state s = x.past(1, 0) + x\n",
                "Clip c:\n    Note when s > 0\n",
            ],
            &[],
        ),
        (
            &[
                "state gate(false)\n",
                "state level(0.0)\n",
                "state note(60)\n",
                "state rising = gate and not gate.past(1)\n",
                "state held = if rising then level else held.past(1, 0.0)\n",
                "state recent = (recent.past(1, []) + [note])[^8..]\n",
            ],
            &[],
        ),
    ];
    support::assert_rows_in_orders(orders, &symphonyscript_fixtures::vocabulary(), rows);
}

#[test]
fn a_history_read_counts_as_an_operand_of_its_cells_integer_width() {
    a_history_read_counts_as_an_operand_of_its_cells_integer_width_in(
        support::Orders::ForwardAndReversed,
    );
}

#[test]
#[ignore = "exhaustive"]
fn exhaustive_a_history_read_counts_as_an_operand_of_its_cells_integer_width() {
    a_history_read_counts_as_an_operand_of_its_cells_integer_width_in(support::Orders::Every);
}

fn the_arms_of_a_choice_are_type_checked_against_one_another_whatever_chooses_in(
    orders: support::Orders,
) {
    let a = "state a([]): i32[..5]\n";
    let flag = "state flag(false)\n";
    let rows: &[support::Row] = &[
        (
            &[
                a,
                "let k = 1\n",
                "let m = match k:\n    1: a\n    else: 5\n",
                "Clip c:\n    Note/(m[0])\n",
            ],
            &[(Code::TypeMismatch, "match k:\n    1: a\n    else: 5\n")],
        ),
        (
            &[
                a,
                "let m = match true:\n    true: 5\n    false: a\n",
                "Clip c:\n    Note/(m)\n",
            ],
            &[(
                Code::TypeMismatch,
                "match true:\n    true: 5\n    false: a\n",
            )],
        ),
        (
            &[
                a,
                flag,
                "let x = if true then (if flag then a else 5) else a\n",
                "Clip c:\n    Note/(x[0])\n",
            ],
            &[(Code::TypeMismatch, "if flag then a else 5")],
        ),
        (
            &[
                "state note(60)\n",
                "let x = if true then [note, note] else 5\n",
                "Clip c:\n    Note/(x)\n",
            ],
            &[(Code::TypeMismatch, "if true then [note, note] else 5")],
        ),
    ];
    support::assert_rows_in_orders(orders, &symphonyscript_fixtures::vocabulary(), rows);
    let held = "state held([1, 2]): i32[..4]\n";
    let note = "state note(60)\n";
    let direct = |value: &'static str| -> [&'static str; 3] { [held, note, value] };
    let rows: &[support::Row] = &[
        (
            &direct("Clip c:\n    Note/(if false then held else note)\n"),
            &[(Code::TypeMismatch, "if false then held else note")],
        ),
        (
            &direct("Clip c:\n    Note/(if false then held[..2] else 5)\n"),
            &[(Code::TypeMismatch, "if false then held[..2] else 5")],
        ),
        (
            &[
                held,
                note,
                "fn g(i): if i > 5 then held else note\n",
                "Clip c:\n    Note/(g(0))\n",
            ],
            &[(Code::TypeMismatch, "if i > 5 then held else note")],
        ),
        (
            &[
                held,
                note,
                "fn g(i): if i > 5 then [1, 2] else held[i] + 1\n",
                "Clip c:\n    Note/(g(0))\n",
            ],
            &[(Code::TypeMismatch, "if i > 5 then [1, 2] else held[i] + 1")],
        ),
        (
            &[
                held,
                note,
                "fn g(i):\n    match i:\n        7: held\n        else: note\n",
                "Clip c:\n    Note/(g(0))\n",
            ],
            &[(
                Code::TypeMismatch,
                "match i:\n        7: held\n        else: note\n",
            )],
        ),
        (
            &[
                held,
                note,
                "fn g(i):\n    match i:\n        7: [1, 2]\n        else: held[i] + 1\n",
                "Clip c:\n    Note/(g(0))\n",
            ],
            &[(
                Code::TypeMismatch,
                "match i:\n        7: [1, 2]\n        else: held[i] + 1\n",
            )],
        ),
        (
            &[
                held,
                note,
                "fn f(i): if i > 1 then [1] else f(i + 1) + [i]\n",
                "Clip c:\n    Note/(if false then f(0) else 5)\n",
            ],
            &[(Code::TypeMismatch, "if false then f(0) else 5")],
        ),
    ];
    support::assert_rows_in_orders(orders, &symphonyscript_fixtures::vocabulary(), rows);
    let flag = "state flag(true)\n";
    let grid = "state rows([[1, 2, 3]]): i32[3][..10]\n";
    let first = "Clip c:\n    Note/(g(0)[0])\n";
    let read_a = "Clip c:\n    Note/(a[0])\n";
    let rows: &[support::Row] = &[
        (
            &[held, "fn g(i): if i > 5 then [[1]] else held\n", first],
            &[(Code::TypeMismatch, "if i > 5 then [[1]] else held")],
        ),
        (
            &[held, "fn g(i): if i > 5 then held else [[1]]\n", first],
            &[(Code::TypeMismatch, "if i > 5 then held else [[1]]")],
        ),
        (
            &[
                held,
                "fn g(i): if i > 5 then [[1]] else [held[i]] + [1]\n",
                first,
            ],
            &[(
                Code::TypeMismatch,
                "if i > 5 then [[1]] else [held[i]] + [1]",
            )],
        ),
        (
            &[
                held,
                "fn g(i): if i > 5 then [held[i]] + [1] else [[1]]\n",
                first,
            ],
            &[(
                Code::TypeMismatch,
                "if i > 5 then [held[i]] + [1] else [[1]]",
            )],
        ),
        (
            &[grid, "fn g(i): if i > 5 then rows else [1, 2]\n", first],
            &[(Code::TypeMismatch, "if i > 5 then rows else [1, 2]")],
        ),
        (
            &[grid, "Clip c:\n    Note/(if false then rows[0] else 5)\n"],
            &[(Code::TypeMismatch, "if false then rows[0] else 5")],
        ),
        (
            &[
                grid,
                note,
                "Clip c:\n    Note/(if false then rows[0] else note)\n",
            ],
            &[(Code::TypeMismatch, "if false then rows[0] else note")],
        ),
        (
            &[
                held,
                "fn g(i): if i > 5 then [[1]] else held\n",
                "let a = g(0)\n",
                read_a,
            ],
            &[(Code::TypeMismatch, "if i > 5 then [[1]] else held")],
        ),
        (
            &[
                grid,
                "let x = if false then rows[0] else 5\n",
                "Clip c:\n    Note/(x)\n",
            ],
            &[(Code::TypeMismatch, "if false then rows[0] else 5")],
        ),
        (
            &["fn g(i): if i > 5 then [[1]] else [1, 2]\n", first],
            &[(Code::TypeMismatch, "if i > 5 then [[1]] else [1, 2]")],
        ),
        (
            &[
                "fn g(i): if i > 5 then [[1]] else [1, 2]\n",
                "let a = g(0)\n",
                read_a,
            ],
            &[(Code::TypeMismatch, "if i > 5 then [[1]] else [1, 2]")],
        ),
        (
            &[
                "fn g(i):\n    match i:\n        7: [[1]]\n        else: [1, 2]\n",
                first,
            ],
            &[(
                Code::TypeMismatch,
                "match i:\n        7: [[1]]\n        else: [1, 2]\n",
            )],
        ),
        (
            &[
                held,
                "Clip c:\n    Note/((if false then held.bytes else [[1]])[0][0])\n",
            ],
            &[(Code::TypeMismatch, "if false then held.bytes else [[1]]")],
        ),
        (
            &[
                held,
                "Clip c:\n    Note/((if false then (if true then held else [1, 2]) else [[1]])[0][0])\n",
            ],
            &[(
                Code::TypeMismatch,
                "if false then (if true then held else [1, 2]) else [[1]]",
            )],
        ),
        (
            &[
                held,
                flag,
                "fn g(i): if i > 5 then (if flag then held else [1, 2]) else [[1]]\n",
                "Clip c:\n    Note/(g(0)[0][0])\n",
            ],
            &[(
                Code::TypeMismatch,
                "if i > 5 then (if flag then held else [1, 2]) else [[1]]",
            )],
        ),
        (
            &[
                held,
                note,
                "fn h(i):\n    match i:\n        7: held\n        else: [1, 2]\n",
                "fn g(i): if i > 5 then h(note) else [[1]]\n",
                "Clip c:\n    Note/(g(0)[0][0])\n",
            ],
            &[(Code::TypeMismatch, "if i > 5 then h(note) else [[1]]")],
        ),
        (
            &[
                held,
                flag,
                "let inner = if flag then held else held\n",
                "Clip c:\n    Note/((if false then inner else [[1]])[0][0])\n",
            ],
            &[(Code::TypeMismatch, "if false then inner else [[1]]")],
        ),
        (
            &[
                held,
                "let b = held.bytes\n",
                "Clip c:\n    Note/((if false then b else [[1]])[0][0])\n",
            ],
            &[(Code::TypeMismatch, "if false then b else [[1]]")],
        ),
        (
            &[
                held,
                "fn g(i): if i > 5 then held.bytes else 5\n",
                "Clip c:\n    Note/(g(0))\n",
            ],
            &[(Code::TypeMismatch, "if i > 5 then held.bytes else 5")],
        ),
        (
            &[
                held,
                "state s = (if false then held.past(1) else [[1]])[0][0]\n",
                "Clip c:\n    Note/(s)\n",
            ],
            &[(Code::TypeMismatch, "if false then held.past(1) else [[1]]")],
        ),
        (
            &[
                held,
                "state t = held\n",
                "state s = (if false then t.past(1, []) else [[1]])[0][0]\n",
                "Clip c:\n    Note/(s)\n",
            ],
            &[(Code::TypeMismatch, "if false then t.past(1, []) else [[1]]")],
        ),
        (
            &[
                held,
                "state s = (if false then held.past(1) else [1])[0]\n",
                "Clip c:\n    Note/(s)\n",
            ],
            &[],
        ),
        (
            &[
                "fn f(n): if n > 3 then [1] else [1] + f(n + 1)\n",
                "Clip c:\n    Note/((if false then f(0) else [[1]])[0][0])\n",
            ],
            &[(Code::TypeMismatch, "if false then f(0) else [[1]]")],
        ),
        (
            &[
                "fn f(n): if n > 3 then [1] else [1] + f(n + 1)\n",
                "Clip c:\n    Note/((if false then [[1]] else f(0))[0][0])\n",
            ],
            &[(Code::TypeMismatch, "if false then [[1]] else f(0)")],
        ),
        (
            &[
                held,
                "fn f(i): if i > 2 then [0] else [i] + f(i + 1)\n",
                "Clip c:\n    Note/((if false then f(0) else [[1]])[0][0])\n",
            ],
            &[(Code::TypeMismatch, "if false then f(0) else [[1]]")],
        ),
        (
            &[
                "fn f(i): if i > 1 then [1] else f(i + 1) + [i]\n",
                "Clip c:\n    Note/(if false then f(0) else 5)\n",
            ],
            &[(Code::TypeMismatch, "if false then f(0) else 5")],
        ),
        (
            &[
                "fn f(n): if n > 3 then [[1]] else [1] + f(n + 1)\n",
                "Clip c:\n    Note/(f(0)[0])\n",
            ],
            &[(
                Code::TypeMismatch,
                "if n > 3 then [[1]] else [1] + f(n + 1)",
            )],
        ),
        (
            &[
                "fn f(n): if n > 3 then [1] else [1] + f(n + 1)\n",
                "Clip c:\n    Note/((if false then f(0) else [1])[0])\n",
            ],
            &[],
        ),
        (
            &[
                "fn f(n): if n > 3 then [1] else [1] + f(n + 1)\n",
                "Clip c:\n    Note/(f(0)[0])\n",
            ],
            &[],
        ),
        (
            &[
                "fn f(n): if n > 3 then [1] else [1] + f(n + 1)\n",
                "fn k(n): if n > 0 then k(n - 1) else (if false then f(n) else [[1]])[0][0]\n",
                "Clip c:\n    Note/(k(1))\n",
            ],
            &[(Code::TypeMismatch, "if false then f(n) else [[1]]")],
        ),
        (
            &[
                "fn f(n):\n    match n:\n        0: [1]\n        else: f(n - 1)\n",
                "fn k(n): if n > 0 then k(n - 1) else (if false then f(n) else [[1]])[0][0]\n",
                "Clip c:\n    Note/(k(1))\n",
            ],
            &[(Code::TypeMismatch, "if false then f(n) else [[1]]")],
        ),
        (
            &[
                "fn f(n): if n > 3 then f(n - 1) else [1]\n",
                "fn k(n): if n > 0 then k(n - 1) else (if false then f(n) else [[1]])[0][0]\n",
                "Clip c:\n    Note/(k(1))\n",
            ],
            &[(Code::TypeMismatch, "if false then f(n) else [[1]]")],
        ),
        (
            &[
                "fn g(n): if n > 0 then g(n - 1) else [1]\n",
                "fn f(n): if n > 3 then g(n) else f(n + 1)\n",
                "Clip c:\n    Note/((if false then f(0) else [[1]])[0][0])\n",
            ],
            &[(Code::TypeMismatch, "if false then f(0) else [[1]]")],
        ),
        (
            &[
                "fn g(n): if n > 0 then g(n - 1) else [1]\n",
                "fn f(n): if n > 3 then g(n) else f(n + 1)\n",
                "Clip c:\n    Note/((if false then f(0) else [1])[0])\n",
            ],
            &[],
        ),
        (
            &[
                held,
                flag,
                "fn f(i): if i > 2 then [0] else [i] + f(i + 1)\n",
                "Clip c:\n    Note/((if flag then f(0) else [[1]])[0][0])\n",
            ],
            &[(Code::TypeMismatch, "if flag then f(0) else [[1]]")],
        ),
        (
            &[
                flag,
                "fn f(n): if n > 3 then [1] else [1] + f(n + 1)\n",
                "Clip c:\n    Note/((if flag then f(0) else [[1]])[0][0])\n",
            ],
            &[(Code::TypeMismatch, "if flag then f(0) else [[1]]")],
        ),
        (
            &[
                flag,
                "fn f(n): if n > 3 then [1] else [1] + f(n + 1)\n",
                "Clip c:\n    Note/((if flag then [[1]] else f(0))[0][0])\n",
            ],
            &[(Code::TypeMismatch, "if flag then [[1]] else f(0)")],
        ),
        (
            &[
                flag,
                "fn f(n): if n > 3 then [1] else [1] + f(n + 1)\n",
                "let r = f(0)\n",
                "Clip c:\n    Note/((if flag then r else [[1]])[0][0])\n",
            ],
            &[(Code::TypeMismatch, "if flag then r else [[1]]")],
        ),
        (
            &[
                "fn f(n): if n > 3 then [1] else [1] + f(n + 1)\n",
                "let r = f(0)\n",
                "Clip c:\n    Note/((if false then r else [[1]])[0][0])\n",
            ],
            &[(Code::TypeMismatch, "if false then r else [[1]]")],
        ),
        (
            &[
                flag,
                "fn f(n): if n > 3 then [1] else [1] + f(n + 1)\n",
                "Clip c:\n    Note/((if flag then f(0) else [2])[0])\n",
            ],
            &[],
        ),
    ];
    support::assert_rows_in_orders(orders, &symphonyscript_fixtures::vocabulary(), rows);
    let rows: &[support::Row] = &[
        (
            &[
                held,
                "Song s:\n    Clip as c:\n        Note/((if false then mirror(held) else [[1]])[0][0])\n",
            ],
            &[(Code::TypeMismatch, "if false then mirror(held) else [[1]]")],
        ),
        (
            &[
                held,
                "Song s:\n    Clip as c:\n        Note/((if false then mirror(held) else [1])[0])\n",
            ],
            &[],
        ),
    ];
    support::assert_rows_in_orders(orders, &symphonyscript_fixtures::native_vocabulary(), rows);
    let sec = "state sec(0)\n";
    let rows: &[support::Row] = &[
        (
            &[
                sec,
                "Clip a:\n    Plays | gain=((if false then to.seq else [[1]])[0][0]): [b, c][sec]\n",
                "Clip b\n",
                "Clip c\n",
            ],
            &[(Code::TypeMismatch, "if false then to.seq else [[1]]")],
        ),
        (
            &[
                sec,
                "Clip a:\n    Plays | gain=((if false then to.seq else [1])[0]): [b, c][sec]\n",
                "Clip b\n",
                "Clip c\n",
            ],
            &[],
        ),
    ];
    support::assert_rows_in_orders(orders, &support::to_array_vocabulary(), rows);
}

#[test]
fn the_arms_of_a_choice_are_type_checked_against_one_another_whatever_chooses() {
    the_arms_of_a_choice_are_type_checked_against_one_another_whatever_chooses_in(
        support::Orders::ForwardAndReversed,
    );
}

#[test]
#[ignore = "exhaustive"]
fn exhaustive_the_arms_of_a_choice_are_type_checked_against_one_another_whatever_chooses() {
    the_arms_of_a_choice_are_type_checked_against_one_another_whatever_chooses_in(
        support::Orders::Every,
    );
}

fn an_annotation_holds_only_an_initializer_of_its_rows_in(orders: support::Orders) {
    let cells = [
        "state a([]): i32[..5]\n",
        "state g([]): i32[3][..10]\n",
        "state note(60)\n",
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
            "state s: i32[..8] = (s.past(1, []) + [note])[^8..]\n",
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
        for text in in_orders(orders, &items) {
            assert_eq!(codes(&text), expected.to_vec(), "{text}");
        }
    }
}

#[test]
fn an_annotation_holds_only_an_initializer_of_its_rows() {
    an_annotation_holds_only_an_initializer_of_its_rows_in(support::Orders::ForwardAndReversed);
}

#[test]
#[ignore = "exhaustive"]
fn exhaustive_an_annotation_holds_only_an_initializer_of_its_rows() {
    an_annotation_holds_only_an_initializer_of_its_rows_in(support::Orders::Every);
}

#[test]
fn a_type_whose_length_or_capacity_is_missing_or_malformed_is_a_placeholder() {
    let read = "Clip c:\n    Note/(g[0])\n";
    support::assert_rows_in_every_order(
        &vocabulary(),
        &[
            (
                &["state g([1, 2]): i32[..]\n", read],
                &[(Code::TypeSyntax, "]")],
            ),
            (
                &["let g: i32[..] = [1, 2]\n", read],
                &[(Code::TypeSyntax, "]")],
            ),
            (
                &["state g([1, 2]): i32[]\n", read],
                &[(Code::TypeSyntax, "]")],
            ),
            (
                &["state g([1, 2]): i32[..x]\n", read],
                &[(Code::UnbuiltConstruct, "x")],
            ),
            (
                &["state x(1)\n", "state g: i32[..] = [x, x]\n", read],
                &[(Code::TypeSyntax, "]")],
            ),
            (&["state g([1, 2]): i32[..2]\n", read], &[]),
            (
                &["state g([1, 2]): i32[..1]\n", read],
                &[(Code::CapacityExceeded, "[1, 2]")],
            ),
            (
                &["state g([1, 2]): i32[..0x10]\n", read],
                &[(Code::UnbuiltConstruct, "0x10")],
            ),
            (
                &["state g([1, 2, 3]): i32[0b11]\n", read],
                &[(Code::UnbuiltConstruct, "0b11")],
            ),
            (
                &["state g([1, 2, 3]): i32[3i32]\n", read],
                &[(Code::UnbuiltConstruct, "3i32")],
            ),
            (
                &["state g([1]): i32[..1.5]\n", read],
                &[(Code::UnbuiltConstruct, "1.5")],
            ),
            (
                &["let g: i32[..0x10] = [1, 2]\n", read],
                &[(Code::UnbuiltConstruct, "0x10")],
            ),
            (
                &["let g: i32[0b11] = [1, 2, 3]\n", read],
                &[(Code::UnbuiltConstruct, "0b11")],
            ),
            (
                &["let g: i32[3i32] = [1, 2, 3]\n", read],
                &[(Code::UnbuiltConstruct, "3i32")],
            ),
            (
                &["let g: i32[..1.5] = [1]\n", read],
                &[(Code::UnbuiltConstruct, "1.5")],
            ),
            (
                &["state g([1]): i32[..99999999999999999999]\n", read],
                &[(Code::LengthLimit, "99999999999999999999")],
            ),
            (
                &["state g([1]): i32[..2147483648]\n", read],
                &[(Code::LengthLimit, "2147483648")],
            ),
            (&["state g([1, 2]): i32[..1_0]\n", read], &[]),
            (&["state g([1]): i32[..2147483647]\n", read], &[]),
            (
                &["state g([1, 2]): i32^[2]\n", read],
                &[(Code::MisplacedCaret, "^")],
            ),
            (
                &["state g([1, 2]): i32[2]^\n", read],
                &[(Code::MisplacedCaret, "^")],
            ),
            (
                &["state x(1)\n", "state g: i32^[2] = [x, x]\n", read],
                &[(Code::MisplacedCaret, "^")],
            ),
            (
                &["let g: i32^[2] = [1, 2]\n", read],
                &[(Code::MisplacedCaret, "^")],
            ),
        ],
    );
    let sequence = "Clip c | sequence=(mirror(held) + [1]):\n    Note\n";
    support::assert_rows_in_every_order(
        &symphonyscript_fixtures::native_vocabulary(),
        &[
            (
                &["state held([]): i32^[..4]\n", sequence],
                &[(Code::MisplacedCaret, "^")],
            ),
            (
                &["let held: i32^[..4] = []\n", sequence],
                &[(Code::MisplacedCaret, "^")],
            ),
        ],
    );
}

fn a_state_settled_as_a_fixed_array_is_used_as_a_fixed_array_in(orders: support::Orders) {
    let fixed = "state s = (s.past(1, [0, 0]) + [note])[1..]\n";
    let variable = "state r = (r.past(1, []) + [note])[^8..]\n";
    let rows: &[OrderedRow] = &[
        (
            &[
                NOTE,
                fixed,
                "Clip c:\n    for i in 0..s.size:\n        Note/(s[i])\n",
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
                "state s = (z.past(1, [0, 0]) + [note])[1..]\n",
                "state z = p\n",
                "Clip c:\n    Note/(probe[1])\n",
            ],
            &[],
        ),
        (&[NOTE, variable, "Clip c:\n    Note/(r.size)\n"], &[]),
        (
            &[
                NOTE,
                variable,
                "Clip c:\n    for i in 0..r.size:\n        Note/(r[i])\n",
            ],
            &[(Code::ReactiveIterable, "r.size", 0)],
        ),
    ];
    for (items, expected) in rows {
        for text in in_orders(orders, items) {
            support::expect_codes(&text, expected);
        }
    }
}

#[test]
fn a_state_settled_as_a_fixed_array_is_used_as_a_fixed_array() {
    a_state_settled_as_a_fixed_array_is_used_as_a_fixed_array_in(
        support::Orders::ForwardAndReversed,
    );
}

#[test]
#[ignore = "exhaustive"]
fn exhaustive_a_state_settled_as_a_fixed_array_is_used_as_a_fixed_array() {
    a_state_settled_as_a_fixed_array_is_used_as_a_fixed_array_in(support::Orders::Every);
}

fn compile_time_condition_rows(orders: support::Orders, part: usize, parts: usize) {
    let cells = [
        "state note(60)\n",
        "state a([]): i32[..5]\n",
        "state held([]): i32[..10]\n",
        "state flag(false)\n",
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
    for (declarations, expected) in rows.iter().skip(part).step_by(parts) {
        let mut items: Vec<&str> = cells.to_vec();
        items.extend_from_slice(declarations);
        for text in in_orders(orders, &items) {
            support::expect_codes(&text, expected);
        }
    }
    let choices = [
        ("if flag then a else [1, 2, 3, 4, 5, 6, 7]", 7),
        ("if flag then [note, note] else [note, note, note]", 3),
    ];
    for (value, capacity) in choices.iter().skip(part).step_by(parts) {
        assert_ordered_capacity_in(orders, &cells, value, *capacity);
    }
}

#[test]
fn a_compile_time_condition_chooses_its_arm_before_layout_and_capacity_forward_and_reversed() {
    compile_time_condition_rows(support::Orders::ForwardAndReversed, 0, 1);
}

#[test]
#[ignore = "exhaustive"]
fn exhaustive_a_compile_time_condition_chooses_its_arm_before_layout_and_capacity_in_its_odd_rows()
{
    compile_time_condition_rows(support::Orders::Every, 0, 2);
}

#[test]
#[ignore = "exhaustive"]
fn exhaustive_a_compile_time_condition_chooses_its_arm_before_layout_and_capacity_in_its_even_rows()
{
    compile_time_condition_rows(support::Orders::Every, 1, 2);
}

fn a_compile_time_match_on_a_bare_variant_chooses_its_arm_before_layout_and_capacity_in(
    orders: support::Orders,
) {
    let cell = "state a([]): i32[..5]\n";
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
        for text in in_orders(orders, &items) {
            support::expect_codes(&text, expected);
        }
    }
    assert_ordered_capacity_in(
        orders,
        &[cell, "state v(Wave::Saw)\n"],
        "match v:\n    Saw: a\n    else: [1, 2, 3, 4, 5, 6, 7]",
        7,
    );
}

#[test]
fn a_compile_time_match_on_a_bare_variant_chooses_its_arm_before_layout_and_capacity() {
    a_compile_time_match_on_a_bare_variant_chooses_its_arm_before_layout_and_capacity_in(
        support::Orders::ForwardAndReversed,
    );
}

#[test]
#[ignore = "exhaustive"]
fn exhaustive_a_compile_time_match_on_a_bare_variant_chooses_its_arm_before_layout_and_capacity() {
    a_compile_time_match_on_a_bare_variant_chooses_its_arm_before_layout_and_capacity_in(
        support::Orders::Every,
    );
}

fn a_group_of_states_takes_its_least_capacities_together_in(orders: support::Orders) {
    let pair = [
        NOTE,
        "state p = (q.past(1, []) + [note])[^5..]\n",
        "state q = (p.past(1, []) + [note, note])[^7..]\n",
    ];
    assert_ordered_capacity_in(orders, &pair, "p", 5);
    assert_ordered_capacity_in(orders, &pair, "q", 7);
    let ring = [
        NOTE,
        "state s0 = (s2.past(1, []) + [note])[^8..]\n",
        "state s1 = (s0.past(1, []) + [note])[^8..]\n",
        "state s2 = (s1.past(1, []) + [note])[^8..]\n",
    ];
    for state in ["s0", "s1", "s2"] {
        assert_ordered_capacity_in(orders, &ring, state, 8);
    }
}

#[test]
fn a_group_of_states_takes_its_least_capacities_together() {
    a_group_of_states_takes_its_least_capacities_together_in(support::Orders::ForwardAndReversed);
}

#[test]
#[ignore = "exhaustive"]
fn exhaustive_a_group_of_states_takes_its_least_capacities_together() {
    a_group_of_states_takes_its_least_capacities_together_in(support::Orders::Every);
}

#[test]
fn a_capacity_reaching_two_to_the_thirty_one_draws_length_limit_where_it_arises() {
    let big = "state big([]): i32[..2000000000]\n";
    let rows: &[OrderedRow] = &[
        (
            &[
                NOTE,
                big,
                "let x = big + big\n",
                "Clip c:\n    Note/(x.size)\n",
            ],
            &[(Code::LengthLimit, "big + big", 0)],
        ),
        (
            &[
                NOTE,
                big,
                "state s = big + big\n",
                "Clip c:\n    Note/(s.size)\n",
            ],
            &[(Code::LengthLimit, "big + big", 0)],
        ),
        (
            &[
                NOTE,
                big,
                "state s = big[^2000000000..] + big\n",
                "Clip c:\n    Note/(s.size)\n",
            ],
            &[(Code::LengthLimit, "big[^2000000000..] + big", 0)],
        ),
        (
            &[
                NOTE,
                big,
                "state r = (r.past(1, []) + [note])[^2147483648..]\n",
                "Clip c:\n    Note/(r.size)\n",
            ],
            &[(
                Code::LengthLimit,
                "(r.past(1, []) + [note])[^2147483648..]",
                0,
            )],
        ),
        (
            &[
                NOTE,
                big,
                "state r = (r.past(1, []) + [note])[^2147483647..]\n",
                "Clip c:\n    Note/(r.size)\n",
            ],
            &[(Code::LengthLimit, "r.past(1, []) + [note]", 0)],
        ),
        (
            &[
                NOTE,
                big,
                "let probe: i32[..5] = big + big\n",
                "Clip c:\n    Note/(probe.size)\n",
            ],
            &[(Code::LengthLimit, "big + big", 0)],
        ),
        (
            &[
                NOTE,
                "state r = (r.past(1, []) + [note])[^4611686018427387905..]\n",
                "Clip c:\n    Note/(r.size)\n",
            ],
            &[(
                Code::LengthLimit,
                "(r.past(1, []) + [note])[^4611686018427387905..]",
                0,
            )],
        ),
        (
            &[
                NOTE,
                "state r = (r.past(1, []) + [note])[^9223372036854775807..]\n",
                "Clip c:\n    Note/(r.size)\n",
            ],
            &[(
                Code::LengthLimit,
                "(r.past(1, []) + [note])[^9223372036854775807..]",
                0,
            )],
        ),
        (
            &[
                NOTE,
                "state r = (r.past(1, []) + [note])[..9223372036854775807]\n",
                "Clip c:\n    Note/(r.size)\n",
            ],
            &[(
                Code::LengthLimit,
                "(r.past(1, []) + [note])[..9223372036854775807]",
                0,
            )],
        ),
        (
            &[
                NOTE,
                "state r = (r.past(1, []) + [note])[^9223372036854775808..]\n",
                "Clip c:\n    Note/(r.size)\n",
            ],
            &[(Code::IndexOutOfRange, "9223372036854775808", 0)],
        ),
        (
            &[
                NOTE,
                "state r = (r.past(1, []) + [note])[^8..]\n",
                "Clip c:\n    Note/(r.size)\n",
            ],
            &[],
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
            "state a = b.past(1, []) + [note]\n",
            "state b = a[^2147483648..]\n",
            "Clip c:\n    Note/(a.size)\n",
        ],
        &[Code::LengthLimit],
    );
    for (bound, capacity) in [
        ("[^4611686018427387905..]", "4611686018427387905"),
        ("[^9223372036854775807..]", "9223372036854775807"),
        ("[..9223372036854775807]", "9223372036854775807"),
    ] {
        let text = format!(
            "{NOTE}state r = (r.past(1, []) + [note]){bound}\nClip c:\n    Note/(r.size)\n"
        );
        let found = support::checked(&text);
        assert!(
            found.len() == 1 && found[0].summary.contains(capacity),
            "{text}{found:?}"
        );
    }
    assert_ordered_codes(
        &[
            NOTE,
            big,
            "state r = r.past(1, []) + [note]\n",
            "Clip c:\n    Note/(r.size)\n",
        ],
        &[Code::UnboundedState],
    );
    assert_ordered_capacity(&[NOTE, big], "big + [1]", 2_000_000_001);
    assert_ordered_capacity(
        &[NOTE, "state r = (r.past(1, []) + [note])[^2147483646..]\n"],
        "r",
        2_147_483_646,
    );
}

fn a_runtime_choice_between_a_variable_and_a_fixed_array_is_one_variable_array_wherever_read_in(
    orders: support::Orders,
) {
    let held = "state held([]): i32[..10]\n";
    let gate = "state gate(false)\n";
    let step = "state step(0)\n";
    let chosen = "let s = if gate then held else [0]\n";
    let top = "Clip c:\n    Note/(top)\n";
    let rows: &[support::Row] = &[
        (&[held, gate, chosen, "let top = s[0]\n", top], &[]),
        (&[held, gate, step, chosen, "let top = s[step]\n", top], &[]),
        (&[held, gate, chosen, "let top = s[^1]\n", top], &[]),
        (
            &[
                held,
                gate,
                "state s = if gate then held else s.past(1, [0])\n",
                "let top = s[^1]\n",
                top,
            ],
            &[],
        ),
        (
            &[
                held,
                gate,
                "let fb = [0]\n",
                "state s = if gate then held else fb\n",
                "let top = s[0]\n",
                top,
            ],
            &[],
        ),
        (
            &[
                held,
                "state mode(0)\n",
                "let s = match mode:\n    0: held\n    else: [0]\n",
                "let top = s[0]\n",
                top,
            ],
            &[],
        ),
        (&[held, gate, chosen, "Clip c:\n    Note/(s.size)\n"], &[]),
        (
            &[held, gate, chosen, "Clip c:\n    Note/(s[1..][0])\n"],
            &[],
        ),
        (
            &[
                held,
                gate,
                "let top = (if gate then [0] else held)[0]\n",
                top,
            ],
            &[],
        ),
        (
            &[
                held,
                gate,
                chosen,
                "fn first(xs):\n    let x = xs[0]\n    x + 1\n",
                "Clip c:\n    Note/(first(s))\n",
            ],
            &[],
        ),
        (
            &[
                held,
                gate,
                chosen,
                "pattern P(default xs):\n    Note/(xs[0])\n",
                "Clip c:\n    P/s\n",
            ],
            &[],
        ),
        (
            &[
                held,
                gate,
                chosen,
                "state t = s[0] + t.past(1, 0)\n",
                "Clip c:\n    Note/(t)\n",
            ],
            &[],
        ),
        (
            &[
                gate,
                "let v = if gate then [60, 64] else [62, 65]\n",
                "let x = v[1]\n",
                "Clip c:\n    Note/(x)\n",
            ],
            &[],
        ),
    ];
    support::assert_rows_in_orders(orders, &symphonyscript_fixtures::vocabulary(), rows);
    let state = "state s = if gate then held else s.past(1, [0])\n";
    for (value, capacity) in [
        ("if gate then held else [0]", 10),
        ("if gate then held else [0, 1, 2]", 10),
    ] {
        assert_capacity(
            &format!("state held([]): i32[..10]\nstate gate(false)\nlet s = {value}\n"),
            "s",
            capacity,
        );
    }
    assert_capacity(
        &format!("state held([]): i32[..10]\nstate gate(false)\n{state}"),
        "s",
        10,
    );
}

#[test]
fn a_runtime_choice_between_a_variable_and_a_fixed_array_is_one_variable_array_wherever_read() {
    a_runtime_choice_between_a_variable_and_a_fixed_array_is_one_variable_array_wherever_read_in(
        support::Orders::ForwardAndReversed,
    );
}

#[test]
#[ignore = "exhaustive"]
fn exhaustive_a_runtime_choice_between_a_variable_and_a_fixed_array_is_one_variable_array_wherever_read()
 {
    a_runtime_choice_between_a_variable_and_a_fixed_array_is_one_variable_array_wherever_read_in(
        support::Orders::Every,
    );
}

#[test]
fn an_array_where_a_single_value_is_expected_is_refused_in_the_positions_own_words() {
    let held = "state a([]): i32[..5]\n";
    let rows: &[(&[&str], Code, &str, &[&str])] = &[
        (
            &[held, "Clip c:\n    Note/(a)\n"],
            Code::TypeMismatch,
            "a",
            &["`pitch`", "one `i32`", "variable array"],
        ),
        (
            &[held, "Clip c:\n    Note/([1, 2])\n"],
            Code::TypeMismatch,
            "[1, 2]",
            &["`pitch`", "one `i32`", "an array"],
        ),
        (
            &[held, "Clip c:\n    Note when a\n"],
            Code::ConditionType,
            "a",
            &["one `bool`", "variable array"],
        ),
        (
            &[held, "Clip c:\n    Note when [true, false]\n"],
            Code::ConditionType,
            "[true, false]",
            &["one `bool`", "an array"],
        ),
        (
            &[held, "let b = a\n", "Clip c:\n    Note | velocity=b\n"],
            Code::TypeMismatch,
            "b",
            &["`velocity`", "one `f32`", "variable array"],
        ),
        (
            &["let b = [1, 2]\n", "Clip c:\n    Note/b\n"],
            Code::TypeMismatch,
            "b",
            &["`pitch`", "one `i32`", "an array"],
        ),
    ];
    for (items, code, spanned, words) in rows {
        for text in in_every_order(items) {
            let found = support::checked(&text);
            let [diagnostic] = found.as_slice() else {
                panic!("{text}{found:?}");
            };
            let range = &diagnostic.span.range;
            assert_eq!(
                (
                    diagnostic.code,
                    &text[range.start as usize..range.end as usize]
                ),
                (*code, *spanned),
                "{text}"
            );
            for word in *words {
                assert!(diagnostic.summary.contains(word), "{text}{diagnostic:?}");
            }
        }
    }
    support::assert_rows_in_every_order(
        &symphonyscript_fixtures::vocabulary(),
        &[(&[held, "Clip c:\n    Note/(a[0])\n"], &[])],
    );
    let bytes = "state h([]): u8[..3]\n";
    support::assert_rows_in_every_order(
        &support::signature_vocabulary(),
        &[(&[bytes, "Clip c | data=h:\n    Note\n"], &[])],
    );
    let fixed = support::checked_with(
        &support::signature_vocabulary(),
        &format!("{bytes}Clip c | fixed=h:\n    Note\n"),
    );
    assert!(
        fixed
            .iter()
            .any(|diagnostic| diagnostic.code == Code::VariableArray),
        "{fixed:?}"
    );
}

type Helped = (
    fn() -> Vocabulary,
    &'static [&'static str],
    Code,
    &'static str,
);

#[test]
fn every_diagnostic_carries_a_help_that_states_its_fix() {
    let rows: &[Helped] = &[
        (
            vocabulary,
            &[
                "state note(60)\n",
                "state recent = recent.past(1, []) + [note]\n",
                "Clip c:\n    Note/(recent[0])\n",
            ],
            Code::UnboundedState,
            "slice the rule's value, as in `(s.past(1, []) + [x])[^8..]`, to keep its newest elements",
        ),
        (
            vocabulary,
            &[
                "state n(5)\n",
                "fn fib(k): if k < 2 then k else fib(k - 1) + fib(k - 2)\n",
                "Clip c:\n    Note/(fib(n))\n",
            ],
            Code::ReactiveRecursion,
            "write the recursion over the cell as a `for` expression, as in `for x in held into low from 127: min(low, x)`",
        ),
        (
            vocabulary,
            &[
                "state x(1)\n",
                "state[2] s = x + s.past(3, 0)\n",
                "Clip c:\n    Note when s > 0\n",
            ],
            Code::CellCapacity,
            "remove `[N]`, which states the pending capacity of a host-fed `stream` or `channel` alone",
        ),
        (
            vocabulary,
            &[
                "state x(1)\n",
                "state s: i32[..2] = [x, x, x]\n",
                "Clip c:\n    Note when s[0] > 0\n",
            ],
            Code::CapacityExceeded,
            "slice the value to its newest 2 elements with `[^2..]`",
        ),
        (
            support::signature_vocabulary,
            &[
                "state x(1)\n",
                "Clip c | data=[x, x, x, x, x, x, x, x, x]:\n    Note\n",
            ],
            Code::CapacityExceeded,
            "slice the value to its newest 8 elements with `[^8..]`",
        ),
        (
            support::signature_vocabulary,
            &["state h([]): u8[..3]\n", "Clip c | fixed=h:\n    Note\n"],
            Code::VariableArray,
            "pass a fixed `u8[3]` here, such as a cell annotated `u8[3]`",
        ),
        (
            support::signature_vocabulary,
            &[
                "state xs([]): u8[..3]\n",
                "Clip c:\n    Note/(fixed3(xs))\n",
            ],
            Code::VariableArray,
            "pass a fixed `u8[3]` here, such as a cell annotated `u8[3]`",
        ),
        (
            support::signature_vocabulary,
            &["state f(1.5)\n", "Clip c | three=[f, 2]:\n    Note\n"],
            Code::FloatToInteger,
            "convert explicitly, as in `i32(x)`",
        ),
        (
            vocabulary,
            &[
                "fn f(x):\n    state s = 1\n    x\n",
                "Clip c:\n    Note/(f(1))\n",
            ],
            Code::CellSyntax,
            "declare the cell at top level as `state`, `stream` or `channel`, a name, an init in parentheses, a rule after `=` or both, as in `state level(0)` or `state count = level + 1`",
        ),
        (
            vocabulary,
            &[
                "state x(1)\n",
                "state s = x.past(1, 0.5)\n",
                "Clip c:\n    Note when s > 0\n",
            ],
            Code::TypeMismatch,
            "give the position a value of the type it expects",
        ),
        (
            vocabulary,
            &["let a = sin\n", "Clip c:\n    Note/a\n"],
            Code::NotAValue,
            "use a value here, such as a number, a `let` or a call",
        ),
        (
            vocabulary,
            &["Clip c:\n    Note/(Note)\n"],
            Code::NotAValue,
            "use a value here, such as a number, a `let` or a call",
        ),
        (
            vocabulary,
            &["fn f(x): x\n", "Clip c:\n    Note/(f(y=1))\n"],
            Code::UnknownParameter,
            "name a parameter `f` declares: `x`",
        ),
        (
            vocabulary,
            &[
                "pattern P(default root):\n    Note/root\n",
                "Clip c:\n    P/60 | nope=1\n",
            ],
            Code::UnknownParameter,
            "set a parameter the pattern declares",
        ),
        (
            vocabulary,
            &["fn f(a, b): a\n", "Clip c:\n    Note/(f(1))\n"],
            Code::MissingArgument,
            "pass `b` in the call, as in `f(a, b)`",
        ),
        (
            vocabulary,
            &["pattern P(root):\n    Note/root\n", "Clip c:\n    P\n"],
            Code::MissingArgument,
            "supply the parameter, as in `P | root=60`",
        ),
        (
            vocabulary,
            &["Clip c:\n    Note/60 as n\n"],
            Code::PlacementSyntax,
            "write `as` and the name right after the kind, as in `Note as n /60`",
        ),
        (
            vocabulary,
            &["Clip c:\n    Note/(1 + 2\n"],
            Code::DefaultArgSyntax,
            "add the missing `)` that closes the `/(` value",
        ),
        (
            vocabulary,
            &["Clip c:\n    Note/to\n"],
            Code::DefaultArgSyntax,
            "follow `to` with `.` and an attribute name, as in `to.gain`",
        ),
        (
            vocabulary,
            &["Clip\n"],
            Code::PlacementSyntax,
            "name the top-level instance, as in `Clip intro`",
        ),
        (
            vocabulary,
            &["Clip c:\n    60\n"],
            Code::PlacementSyntax,
            "write the kind, then `as` and a name, a `/` value, a `when` gate and `|` attributes, in that order, as in `Note as n /60 | velocity=0.9`",
        ),
        (
            vocabulary,
            &["Clip c:\n"],
            Code::PlacementSyntax,
            "indent the body's items on the lines after the `:`",
        ),
        (
            vocabulary,
            &["Clip c:\n    (Note\n"],
            Code::PlacementSyntax,
            "add the missing `)` that closes the parenthesized placement",
        ),
        (
            vocabulary,
            &["Clip c:\n    for i in [Saw, 1]:\n        Note\n"],
            Code::UnknownName,
            "write `Wave::Saw`, or use `Saw` in a position that expects a `Wave`",
        ),
        (
            vocabulary,
            &["let z = Saw.gain\n", "Clip c | gain=z\n"],
            Code::UnknownName,
            "write `Wave::Saw`, or use `Saw` in a position that expects a `Wave`",
        ),
        (
            vocabulary,
            &["Clip c | gain=Saw(1)\n"],
            Code::UnknownName,
            "write `Wave::Saw`, or use `Saw` in a position that expects a `Wave`",
        ),
        (
            vocabulary,
            &["Clip c:\n    Note/(nothing)\n"],
            Code::UnknownName,
            "declare the name, or correct its spelling",
        ),
        (
            vocabulary,
            &[
                "state a(0)\n",
                "state s = a.past(1, 0 ) }\n",
                "Clip c:\n    Note/(s)\n",
            ],
            Code::UnexpectedCharacter,
            "remove the `}`, or match it with an opening `{`",
        ),
        (
            vocabulary,
            &[
                "state a(0)\n",
                "state s = a.past(1, 0 ) )\n",
                "Clip c:\n    Note/(s)\n",
            ],
            Code::CellSyntax,
            "remove the `)`, or match it with an opening `(`",
        ),
        (
            vocabulary,
            &["Clip c:\n    Note/60 ]\n"],
            Code::PlacementSyntax,
            "remove the `]`, or match it with an opening `[`",
        ),
        (
            vocabulary,
            &["Clip c:\n    Note/(1 }\n        + 2)\n"],
            Code::DefaultArgSyntax,
            "remove the `}`, or replace it with the `)` that closes the open `(`",
        ),
        (
            vocabulary,
            &["let f = [1, 2)\n", "Clip c:\n    Note/(f[0])\n"],
            Code::ExpressionSyntax,
            "remove the `)`, or replace it with the `]` that closes the open `[`",
        ),
        (
            vocabulary,
            &["state x(\"GET\"): u8[..2]\n", "Clip c:\n    Note/(x[0])\n"],
            Code::CapacityExceeded,
            "write a string of at most 2 bytes, or annotate a capacity of at least 3, as in `u8[..3]`",
        ),
        (
            vocabulary,
            &["let x: u8[..1] = \"GE\"\n", "Clip c:\n    Note/(x[0])\n"],
            Code::CapacityExceeded,
            "write a string of at most 1 byte, or annotate a capacity of at least 2, as in `u8[..2]`",
        ),
        (
            support::signature_vocabulary,
            &["Clip c | data=\"GET GET GET\":\n    Note\n"],
            Code::CapacityExceeded,
            "write a string of at most 8 bytes",
        ),
        (
            support::signature_vocabulary,
            &[
                "state f(false)\n",
                "Clip c | data=(if f then \"GET\" else \"ABCDEFGHI\"):\n    Note\n",
            ],
            Code::CapacityExceeded,
            "write a string of at most 8 bytes",
        ),
        (
            support::signature_vocabulary,
            &[
                "state f(false)\n",
                "let r: u8[..2] = if f then \"GET\" else \"AB\"\n",
                "Clip c | data=r:\n    Note\n",
            ],
            Code::CapacityExceeded,
            "write a string of at most 2 bytes, or annotate a capacity of at least 3, as in `u8[..3]`",
        ),
        (
            support::signature_vocabulary,
            &[
                "state held([]): i32[..9]\n",
                "Clip c | data=held:\n    Note\n",
            ],
            Code::CapacityExceeded,
            "slice the value to its newest 8 elements with `[^8..]`",
        ),
        (
            vocabulary,
            &["state x([1, 2]): i32[..1]\n", "Clip c:\n    Note/(x[0])\n"],
            Code::CapacityExceeded,
            "slice the value to its newest element with `[^1..]`",
        ),
        (
            vocabulary,
            &[
                "state a(0)\n",
                "state[x] t = a.past(1)\n",
                "Clip c:\n    Note/(t)\n",
            ],
            Code::CellSyntax,
            "write the capacity in digits, as in `stream[16] notes(0)`",
        ),
    ];
    for (vocabulary, items, code, help) in rows {
        let compiler = Compiler::new(&vocabulary()).expect("the vocabulary compiles");
        for order in support::permutations(items) {
            let text = order.concat();
            let found = support::errors(compiler.check(&[support::source(&text)]));
            let [diagnostic] = found.as_slice() else {
                panic!("{text}{found:?}");
            };
            assert_eq!(
                (diagnostic.code, diagnostic.help.as_deref()),
                (*code, Some(*help)),
                "{text}"
            );
        }
    }
    let natives = Compiler::new(&symphonyscript_fixtures::native_vocabulary())
        .expect("the native vocabulary compiles");
    for (read, summary, help) in [
        (
            "Clip a:\n    Note/(b.sequence[0])\n",
            "the index 0 is outside the array of length 0",
            "the array is empty: give it elements, or read the element only where its `size` is above 0",
        ),
        (
            "Clip a:\n    Note/(b.seq[7])\n",
            "the index 7 is outside the array of length 2",
            "keep the index inside the array, from 0 to its length minus one",
        ),
    ] {
        for order in support::permutations(&[read, "Clip b:\n    Note\n"]) {
            let text = order.concat();
            let found = support::errors(natives.check(&[support::source(&text)]));
            let [diagnostic] = found.as_slice() else {
                panic!("{text}{found:?}");
            };
            assert_eq!(
                (
                    diagnostic.code,
                    diagnostic.summary.as_str(),
                    diagnostic.help.as_deref()
                ),
                (Code::IndexOutOfRange, summary, Some(help)),
                "{text}"
            );
        }
    }
    let compiler = Compiler::new(&vocabulary()).expect("the vocabulary compiles");
    let text = "Clip c:\n    for w in [Saw, Square]:\n        Note\n";
    let found = support::errors(compiler.check(&[support::source(text)]));
    let helped: Vec<(Code, &str, Option<&str>)> = found
        .iter()
        .map(|diagnostic| {
            let range = &diagnostic.span.range;
            let spanned = text
                .get(range.start as usize..range.end as usize)
                .unwrap_or_default();
            (diagnostic.code, spanned, diagnostic.help.as_deref())
        })
        .collect();
    assert_eq!(
        helped,
        [
            (
                Code::UnknownName,
                "Saw",
                Some("write `Wave::Saw`, or use `Saw` in a position that expects a `Wave`")
            ),
            (
                Code::UnknownName,
                "Square",
                Some("write `Wave::Square`, or use `Square` in a position that expects a `Wave`")
            ),
        ],
        "{found:?}"
    );
    support::assert_rows_in_every_order(
        &vocabulary(),
        &[
            (
                &[
                    "state a(0)\n",
                    "state s = a.past(1, 0 )\n",
                    "Clip c:\n    Note/(s)\n",
                ],
                &[],
            ),
            (&["Clip c:\n    Note/(1\n        + 2)\n"], &[]),
            (
                &["state x(\"GET\"): u8[..3]\n", "Clip c:\n    Note/(x[0])\n"],
                &[],
            ),
            (
                &["state x(\"GE\"): u8[..2]\n", "Clip c:\n    Note/(x[0])\n"],
                &[],
            ),
            (
                &[
                    "state b([]): i32[3][..10]\n",
                    "Clip c:\n    Note/(b[0][0])\n",
                ],
                &[],
            ),
            (
                &[
                    "state a(0)\n",
                    "state t = a.past(1)\n",
                    "Clip c:\n    Note/(t)\n",
                ],
                &[],
            ),
        ],
    );
    support::assert_rows_in_every_order(
        &support::signature_vocabulary(),
        &[
            (
                &[
                    "state f(false)\n",
                    "Clip c | data=(if f then \"GET\" else \"ABCDEFGH\"):\n    Note\n",
                ],
                &[],
            ),
            (
                &[
                    "state f(false)\n",
                    "let r: u8[..3] = if f then \"GET\" else \"AB\"\n",
                    "Clip c | data=r:\n    Note\n",
                ],
                &[],
            ),
        ],
    );
    let depth = [
        "state a(0)\n",
        "state[0] t = a.past(1)\n",
        "Clip c:\n    Note/(t)\n",
    ];
    for order in support::permutations(&depth) {
        let text = order.concat();
        let found = support::errors(compiler.check(&[support::source(&text)]));
        let summaries: Vec<(Code, &str)> = found
            .iter()
            .map(|diagnostic| (diagnostic.code, diagnostic.summary.as_str()))
            .collect();
        assert_eq!(
            summaries,
            [(
                Code::CellCapacity,
                "`[N]` states the capacity of a host-fed `stream` or `channel`, and this cell is derived"
            )],
            "{text}"
        );
    }
    for (call, summary) in [
        ("f(1, 2)", "`f` takes at most 1 argument, not 2"),
        ("g(1, 2, 3)", "`g` takes at most 2 arguments, not 3"),
    ] {
        let items = [
            "fn f(x): x\n",
            "fn g(x, y = 1): x + y\n",
            &format!("Clip c:\n    Note/({call})\n"),
        ];
        for order in support::permutations(&items) {
            let text = order.concat();
            let found = support::errors(compiler.check(&[support::source(&text)]));
            let summaries: Vec<(Code, &str)> = found
                .iter()
                .map(|diagnostic| (diagnostic.code, diagnostic.summary.as_str()))
                .collect();
            assert_eq!(summaries, [(Code::ArgumentCount, summary)], "{text}");
        }
    }
}

#[test]
fn an_iterable_that_reads_a_cell_draws_reactive_iterable_whatever_its_layout() {
    let held = "state held([1, 2]): i32[..4]\n";
    let n = "state n(1)\n";
    let looping = |iterable: &str| format!("Clip c:\n    for w in {iterable}:\n        Note/w\n");
    let (whole, sliced, joined, stated, cut) = (
        looping("held"),
        looping("held[..1]"),
        looping("held + [1]"),
        looping("s"),
        looping("xs[n..]"),
    );
    let rows: &[support::Row] = &[
        (&[held, &whole], &[(Code::ReactiveIterable, "held")]),
        (&[held, &sliced], &[(Code::ReactiveIterable, "held[..1]")]),
        (&[held, &joined], &[(Code::ReactiveIterable, "held + [1]")]),
        (
            &[n, "state s = (s.past(1, []) + [n])[^2..]\n", &stated],
            &[(Code::ReactiveIterable, "s")],
        ),
        (
            &[n, "let xs = [1, 2, 3]\n", &cut],
            &[(Code::ReactiveIterable, "xs[n..]")],
        ),
        (
            &[held, "Clip c:\n    [for w in held: Note/w]\n"],
            &[(Code::ReactiveIterable, "held")],
        ),
        (
            &[
                held,
                "pattern P(xs):\n    for w in xs:\n        Note/w\n",
                "Clip c:\n    P | xs=held\n",
            ],
            &[(Code::ReactiveIterable, "xs")],
        ),
        (
            &[
                "state n([1, 2])\n",
                "Clip c:\n    for w in n:\n        Note/w\n",
            ],
            &[(Code::ReactiveIterable, "n")],
        ),
        (
            &[
                "let xs = [1, 2, 3]\n",
                "Clip c:\n    for w in xs[1..]:\n        Note/w\n",
            ],
            &[],
        ),
        (
            &[
                n,
                "state s = [n, n]\n",
                "Clip c:\n    for i in 0..s.size:\n        Note/i\n",
            ],
            &[],
        ),
    ];
    support::assert_rows_in_every_order(&vocabulary(), rows);
    let native_rows: &[support::Row] = &[
        (
            &[n, &looping("mirror([n, 2])")],
            &[(Code::ReactiveIterable, "mirror([n, 2])")],
        ),
        (
            &[held, &looping("mirror(held)")],
            &[(Code::ReactiveIterable, "mirror(held)")],
        ),
        (&[&looping("mirror([1, 2, 3])")], &[]),
        (
            &["Clip c:\n    for i in 0..text_length(\"abc\"):\n        Note/i\n"],
            &[],
        ),
    ];
    support::assert_rows_in_every_order(&symphonyscript_fixtures::native_vocabulary(), native_rows);
    let placing = |values: &str| {
        format!(
            "state held([{values}]): i32[..4]\nRoot r:\n    Link: r\n    for w in held:\n        Item\n"
        )
    };
    let (two, three) = (placing("1, 2"), placing("1, 2, 3"));
    let bounded_rows: &[support::Row] = &[
        (&[&two], &[(Code::ReactiveIterable, "held")]),
        (&[&three], &[(Code::ReactiveIterable, "held")]),
    ];
    support::assert_rows_in_every_order(
        &symphonyscript_fixtures::bounded_vocabulary(),
        bounded_rows,
    );
}
