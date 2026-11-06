use crate::compiler::Compiler;
use crate::diagnostics::Code;
use crate::syntax::front_end_goldens::source;

const READS: [&str; 30] = [
    "state x(0)\nClip c:\n    Note/(x.past(1))\n",
    "state x(0)\nClip c:\n    Note/(x.past(1, 5))\n",
    "state x(0)\nClip c:\n    Note/(x.past(1, 1.5))\n",
    "state x(0)\nClip c:\n    Note/(x.past(1, true))\n",
    "state x(0.5)\nClip c:\n    Note/(x.past(1, 2))\n",
    "state x(0)\nstate y(1)\nClip c:\n    Note/(x.past(1, y))\n",
    "state x(0)\nClip c | gain=x.past(1, 1.5):\n    Note/1\n",
    "state x(0): u8\nClip c:\n    Note/(x.past(1, 300))\n",
    "state x([1, 2])\nClip c:\n    Note/(x.past(1, 3)[0])\n",
    "state x([1, 2])\nClip c:\n    Note/(x.past(1, [1, 2, 3])[0])\n",
    "state x(0)\nClip c:\n    Note/(x.past(1, 2)[0])\n",
    "state x(0)\nClip c:\n    Note/(x.past(1, 1 + true))\n",
    "state x(0)\nClip c:\n    Note/(x.past(1, x.past(2)))\n",
    "state x(0.5)\nstate p = x.past(1, -1)\nClip c | gain=p:\n    Note/1\n",
    "state x(0)\nstate y = x + 1\nClip c:\n    Note/(y.past(2))\n",
    "state x(1.5)\nstate d = x.past(1) * 2\nClip c:\n    Note/d\n",
    "state x(1)\nstate d = x.past(1) * 2\nClip c | gain=d:\n    Note/d\n",
    "state x(0)\nlet y = x.past(1)\nClip c:\n    Note/y\n",
    "state x(0)\nfn f(): x.past(1) + 1\nClip c:\n    Note/(f())\n",
    "state x(0)\nClip c:\n    Note/(for i in 0..3 into a from 0: a + x.past(1))\n",
    "state x(1.5)\nClip c:\n    Note/(i32(x.past(1)))\n",
    "state x(0)\nClip c:\n    Note/(x.past(1) + 1.5)\n",
    "state x(0)\nClip c:\n    Note/(x.stepped)\n",
    "state x(0)\nClip c | gain=(if x.stepped then 1.0 else 0.0):\n    Note/1\n",
    "state x(0)\nstate y = x.stepped\nClip c:\n    Note/(if y then 1 else 2)\n",
    "state x(0)\nlet k = x.stepped\nClip c:\n    Note/1\n",
    "state x(0)\nClip c:\n    Note/(x.past(1) + x.stepped)\n",
    "state x(0)\nstate y = x\nClip c:\n    Note/(y.past(1))\n",
    "state x(0)\nClip c:\n    Note/(x.past(2147483647))\n",
    "state x(0)\nClip c | gain=x.past(1):\n    Note/(x.past(1))\n",
];

const FAULTS: [&str; 17] = [
    "state x(0)\nClip c:\n    Note/(x.past(0))\n",
    "state x(0)\nClip c:\n    Note/(x.past(-1))\n",
    "state x(0)\nClip c:\n    Note/(x.past(x))\n",
    "state x(0)\nClip c:\n    Note/(x.past(1.5))\n",
    "state x(0)\nClip c:\n    Note/(x.past(true))\n",
    "state x(0)\nClip c:\n    Note/(x.past(2147483648))\n",
    "state x(0)\nClip c:\n    Note/(x.past(4294967297))\n",
    "let y = 3\nClip c:\n    Note/(y.past(1))\n",
    "fn f(v): v.past(1)\nClip c:\n    Note/(f(3))\n",
    "state x(0)\nClip c:\n    Note/(q.past(1))\n",
    "state x(0)\nClip c:\n    Note/(c.past(1))\n",
    "state x(0)\nClip c:\n    for i in 0..3:\n        Note/(i.past(1))\n",
    "state x(0)\nClip c:\n    Note/(for i in 0..3 into a from 0: a + i.past(1))\n",
    "Clip c:\n    Note/(Note.past(1))\n",
    "fn f(): 1\nClip c:\n    Note/(f.past(1))\n",
    "fn f(): 1\nClip c:\n    Note/(if f.stepped then 1 else 2)\n",
    "Clip c:\n    Note/(if abs.stepped then 1 else 2)\n",
];

#[test]
fn a_history_read_and_stepped_read_their_cell_and_draw_their_target_offset_and_fallback_faults_as_the_old_engine_does()
 {
    let vocabulary = symphonyscript_fixtures::vocabulary();
    let old = Compiler::new(&vocabulary)
        .expect("the vocabulary compiles")
        .on_the_old_engine();
    let new = Compiler::new(&vocabulary).expect("the vocabulary compiles");
    for text in READS.iter().chain(FAULTS.iter()) {
        let sources = [source(text)];
        let found = new
            .checked_by_new_core(&sources)
            .unwrap_or_else(|| panic!("the new core takes {text}"));
        assert_eq!(found, old.check(&sources), "{text}");
    }
    let cases: [(&str, &[Code]); 10] = [
        (READS[0], &[]),
        (READS[2], &[Code::TypeMismatch]),
        (READS[1], &[]),
        (READS[7], &[Code::DoesNotFit]),
        ("state x(0): u8\nClip c:\n    Note/(x.past(1, 200))\n", &[]),
        (FAULTS[0], &[Code::HistoryOffset]),
        ("state x(0)\nClip c:\n    Note/(x.past(1))\n", &[]),
        (FAULTS[10], &[Code::HistoryTarget]),
        (READS[22], &[Code::TypeMismatch]),
        (READS[23], &[]),
    ];
    for (text, codes) in cases {
        let found = new
            .checked_by_new_core(&[source(text)])
            .unwrap_or_else(|| panic!("the new core takes {text}"));
        let found: Vec<Code> = found
            .iter()
            .map(|diagnostic| diagnostic.code)
            .filter(|code| *code != Code::NoEntryInstance && *code != Code::UnreachableInstance)
            .collect();
        assert_eq!(found, codes, "{text}");
    }
}

const INITS: [&str; 12] = [
    "state n(2)\nstate s(0) = n + 1\nClip c:\n    Note/(s)\n",
    "state n(2)\nstate s(true) = n + 1\nClip c:\n    Note/(s)\n",
    "state n(2)\nstate s(300): u8 = n\nClip c:\n    Note/(s)\n",
    "state n(2)\nstate s(1.5) = n + 1\nClip c:\n    Note/(s)\n",
    "state n(2.5)\nstate s(1) = n + 1\nClip c:\n    Note/(s)\n",
    "state n(2)\nstate s([1, 2]) = (if n > 1 then [1, 2, 3] else [4, 5, 6])\nClip c:\n    Note/(s[0])\n",
    "state n(2)\nstate s(n) = n + 1\nClip c:\n    Note/(s)\n",
    "state n(2)\nstate s(n)\nClip c:\n    Note/(s)\n",
    "state n(2)\nstate s(n + 1)\nClip c:\n    Note/(s)\n",
    "state n(2)\nstate s(1 + true) = n\nClip c:\n    Note/(s)\n",
    "state n(2)\nstate s(3000000000) = n\nClip c:\n    Note/(s)\n",
    "state n(2)\nstate s([1]) = n\nClip c:\n    Note/(s)\n",
];

const GROUPS: [&str; 20] = [
    "state n(2)\nstate s(0) = s.past(1) + n\nClip c:\n    Note/(s)\n",
    "state n(2)\nstate s(0) = s.past(1, 5) + n\nClip c:\n    Note/(s)\n",
    "state n(2)\nstate s = s.past(1, 0) + n\nClip c:\n    Note/(s)\n",
    "state n(2)\nstate s(0.5) = s.past(1) + n\nClip c:\n    Note/(s)\n",
    "state n(2)\nstate s = s.past(1) + n\nClip c:\n    Note/(s)\n",
    "state n(2)\nstate a(0) = b.past(1) + 1\nstate b = a.past(1) + n\nClip c:\n    Note/(a + b)\n",
    "state n(2)\nstate a = b.past(1) + 1\nstate b(0) = a.past(1) + n\nClip c:\n    Note/(a + b)\n",
    "state n(2)\nstate t(0) = r.past(1)\nstate r = t + n\nClip c:\n    Note/(t)\n",
    "state n(2)\nstate t(0.5) = r.past(1)\nstate r = t + n\nClip c:\n    Note/(r)\n",
    "state t(0) = r.past(1)\nstate r(0.5) = t + go\nstate go(0.0): f32\nClip c:\n    Note | brightness=r\n",
    "state t(0.5) = r.past(1)\nstate r = t + go\nstate go(0.0): f32\nClip c:\n    Note/(t)\n",
    "state c(0) = c.past(1) + 1\nClip k:\n    Note/c\n",
    "state a(0) = b.past(1) + 1\nstate b = a + 1\nClip k:\n    Note/b\n",
    "state n(2)\nstate a = b.past(1) + 1\nstate b = a.past(1) + n\nClip c:\n    Note/(a + b)\n",
    "state n(2)\nstate a = b + n\nstate b = a.past(1)\nClip c:\n    Note/(a + b)\n",
    "state n(2)\nstate s(0) = s.past(1) + n\nClip c:\n    Note/(s.past(2))\n",
    "state n(2)\nstate s(0) = s.past(1) * 1.5 + n\nClip c:\n    Note/(s)\n",
    "state n(true)\nstate s(false) = s.past(1) and n\nClip c:\n    Note/(if s then 1 else 2)\n",
    "state n(2)\nstate s(0) = s.past(1) + n\nClip c | gain=s:\n    Note/1\n",
    "state n(2)\nstate s(0): u8 = s.past(1) + n\nClip c:\n    Note/(s)\n",
];

#[test]
fn a_derived_cells_init_and_a_group_of_cells_reading_one_anothers_history_settle_their_kinds_together_as_the_old_engine_does()
 {
    let vocabulary = symphonyscript_fixtures::vocabulary();
    let old = Compiler::new(&vocabulary)
        .expect("the vocabulary compiles")
        .on_the_old_engine();
    let new = Compiler::new(&vocabulary).expect("the vocabulary compiles");
    for text in INITS.iter().chain(GROUPS.iter()) {
        let sources = [source(text)];
        let found = new
            .checked_by_new_core(&sources)
            .unwrap_or_else(|| panic!("the new core takes {text}"));
        assert_eq!(found, old.check(&sources), "{text}");
    }
    let cases: [(&str, &[Code]); 10] = [
        (INITS[0], &[]),
        (INITS[1], &[Code::TypeMismatch]),
        (INITS[6], &[Code::ReactiveInitializer]),
        (INITS[7], &[Code::ReactiveInitializer]),
        (GROUPS[0], &[]),
        (GROUPS[4], &[Code::HistoryFallback]),
        (GROUPS[2], &[]),
        (GROUPS[11], &[Code::UnrunCell]),
        (GROUPS[12], &[Code::UnrunCell]),
        (GROUPS[3], &[Code::IntegerCell, Code::FloatToInteger]),
    ];
    for (text, codes) in cases {
        let found = new
            .checked_by_new_core(&[source(text)])
            .unwrap_or_else(|| panic!("the new core takes {text}"));
        let found: Vec<Code> = found
            .iter()
            .map(|diagnostic| diagnostic.code)
            .filter(|code| *code != Code::NoEntryInstance && *code != Code::UnreachableInstance)
            .collect();
        assert_eq!(found, codes, "{text}");
    }
}

fn ring(cells: usize, forward: bool) -> String {
    let last = cells - 1;
    let mut text = String::from("state x(1)\n");
    if forward {
        text.push_str(&format!("state r0(0.5) = r{last}.past(1) + x\n"));
        for at in 1..cells {
            text.push_str(&format!("state r{at} = r{}.past(1) + 1\n", at - 1));
        }
    } else {
        text.push_str(&format!("state r{last}(0.5) = r0.past(1) + x\n"));
        for at in 0..last {
            text.push_str(&format!("state r{at} = r{}.past(1) + 1\n", at + 1));
        }
    }
    text.push_str(&format!("Clip c:\n    Note/(r{})\n", cells / 2));
    text
}

#[test]
fn a_float_init_on_a_ring_of_history_reads_makes_every_member_a_float_whichever_way_the_ring_runs()
{
    let vocabulary = symphonyscript_fixtures::vocabulary();
    let old = Compiler::new(&vocabulary)
        .expect("the vocabulary compiles")
        .on_the_old_engine();
    let new = Compiler::new(&vocabulary).expect("the vocabulary compiles");
    for forward in [true, false] {
        let text = ring(10, forward);
        let sources = [source(&text)];
        let found = new
            .checked_by_new_core(&sources)
            .unwrap_or_else(|| panic!("the new core takes {text}"));
        assert_eq!(found, old.check(&sources), "{text}");
        let codes: Vec<Code> = found
            .iter()
            .map(|diagnostic| diagnostic.code)
            .filter(|code| *code != Code::NoEntryInstance)
            .collect();
        assert_eq!(codes, [Code::IntegerCell, Code::FloatToInteger], "{text}");
    }
    let twin = ring(10, true).replace("r0(0.5)", "r0(0)");
    let found = new
        .checked_by_new_core(&[source(&twin)])
        .unwrap_or_else(|| panic!("the new core takes {twin}"));
    assert!(
        found
            .iter()
            .all(|diagnostic| diagnostic.code == Code::NoEntryInstance),
        "{twin}"
    );
}

const HOST_CYCLES: [&str; 5] = [
    "state recent(recent + 1)\nClip c:\n    Note/(recent)\n",
    "state a(b + 1)\nstate b(a)\nClip c:\n    Note/(a)\n",
    "state a(b + 1)\nstate b = a\nClip c:\n    Note/(a)\n",
    "state recent(recent + 1)\nstate x = recent\nClip c:\n    Note/(x)\n",
    "state a(1)\nstate b(a)\nClip c:\n    Note/(b)\n",
];

#[test]
fn a_host_fed_init_reaching_its_own_cell_in_the_present_draws_one_declaration_cycle_as_the_old_engine_does()
 {
    let vocabulary = symphonyscript_fixtures::vocabulary();
    let old = Compiler::new(&vocabulary)
        .expect("the vocabulary compiles")
        .on_the_old_engine();
    let new = Compiler::new(&vocabulary).expect("the vocabulary compiles");
    for text in HOST_CYCLES {
        let sources = [source(text)];
        let found = new
            .checked_by_new_core(&sources)
            .unwrap_or_else(|| panic!("the new core takes {text}"));
        assert_eq!(found, old.check(&sources), "{text}");
        let codes: Vec<Code> = found
            .iter()
            .map(|diagnostic| diagnostic.code)
            .filter(|code| *code != Code::NoEntryInstance)
            .collect();
        let expected: &[Code] = if text.starts_with("state a(1)") {
            &[Code::ReactiveInitializer]
        } else {
            &[Code::DeclarationCycle]
        };
        assert_eq!(codes, expected, "{text}");
    }
}

#[test]
fn a_let_on_a_line_holding_a_front_end_fault_draws_no_reactive_let_beside_a_group_of_cells() {
    let vocabulary = symphonyscript_fixtures::vocabulary();
    let new = Compiler::new(&vocabulary).expect("the vocabulary compiles");
    let group = "state held = if level > 1 then level else held.past(1, 0.0)\nClip c | gain=held:\n    Note\n";
    let cases = [
        (
            format!("state level(0.0)\nlet l = level^ * 2\n{group}"),
            Code::MisplacedCaret,
        ),
        (
            format!("state level(0.0)\nlet  = level * 2\n{group}"),
            Code::LetSyntax,
        ),
        (
            format!("state level(0.0)\nlet l = level * 2\n{group}"),
            Code::ReactiveLet,
        ),
    ];
    for (text, code) in cases {
        let found = new
            .checked_by_new_core(&[source(&text)])
            .unwrap_or_else(|| panic!("the new core takes {text}"));
        let codes: Vec<Code> = found
            .iter()
            .map(|diagnostic| diagnostic.code)
            .filter(|code| *code != Code::NoEntryInstance)
            .collect();
        assert_eq!(codes, [code], "{text}");
    }
}

fn items(text: &str) -> Vec<String> {
    let mut items: Vec<String> = Vec::new();
    for line in text.lines() {
        match items.last_mut() {
            Some(item) if line.starts_with(' ') => {
                item.push('\n');
                item.push_str(line);
            }
            _ => items.push(line.to_string()),
        }
    }
    items
}

fn orders(items: &[String]) -> Vec<String> {
    let mut found = Vec::new();
    let mut order: Vec<usize> = (0..items.len()).collect();
    let mut counts = vec![0usize; items.len()];
    let joined = |order: &[usize]| {
        order
            .iter()
            .filter_map(|at| items.get(*at))
            .fold(String::new(), |text, item| text + item + "\n")
    };
    found.push(joined(&order));
    let mut at = 0;
    while at < order.len() {
        let count = counts.get(at).copied().unwrap_or(0);
        if count < at {
            if at % 2 == 0 {
                order.swap(0, at);
            } else {
                order.swap(count, at);
            }
            found.push(joined(&order));
            if let Some(slot) = counts.get_mut(at) {
                *slot += 1;
            }
            at = 0;
        } else {
            if let Some(slot) = counts.get_mut(at) {
                *slot = 0;
            }
            at += 1;
        }
    }
    found
}

const REACTIVE_LET_TARGETS: [(&str, &[Code]); 7] = [
    (
        "state x(1)\nlet y = x + 1\nstate s = y.past(1, 0.5)\nClip k:\n    Note/(s)\n",
        &[Code::ReactiveLet, Code::HistoryTarget, Code::TypeMismatch],
    ),
    (
        "state x(1)\nlet y = x + 1\nstate p0 = y.past(1, 0)\nstate p1 = y.past(1, 0) + p0\nstate p2 = p1.past(1, 0) + y.past(2, 0)\nClip k:\n    Note/(p2)\n",
        &[
            Code::ReactiveLet,
            Code::HistoryTarget,
            Code::HistoryTarget,
            Code::HistoryTarget,
        ],
    ),
    (
        "state x(1)\nlet y = x + 1\nstate p0 = p1.past(1, 0) + y.past(1, 0)\nstate p1 = p0.past(1, 0)\nClip k:\n    Note/(p1)\n",
        &[Code::ReactiveLet, Code::HistoryTarget],
    ),
    (
        "state note(60)\nlet pair = [note, note]\nstate s = pair.past(1, [0, 0])[1..]\nClip k:\n    Note/(s[0])\n",
        &[Code::ReactiveLet, Code::HistoryTarget],
    ),
    (
        "state x(1)\nlet y = x + 1\nClip k:\n    Note/(if y.stepped then 1 else 2)\n",
        &[Code::ReactiveLet, Code::HistoryTarget],
    ),
    (
        "let y = 5\nstate s = y.past(1, 0.5)\nClip k:\n    Note/(s)\n",
        &[Code::HistoryTarget],
    ),
    (
        "state x(1)\nlet y = 5\nstate s = y.past(1, 0)\nClip k:\n    Note/(s)\n",
        &[Code::HistoryTarget],
    ),
];

#[test]
fn a_history_read_of_a_let_draws_history_target_beside_its_reactive_let_in_every_order_of_the_items_487()
 {
    let vocabulary = symphonyscript_fixtures::vocabulary();
    let new = Compiler::new(&vocabulary).expect("the vocabulary compiles");
    for (text, codes) in REACTIVE_LET_TARGETS {
        for order in orders(&items(text)) {
            let found = new
                .checked_by_new_core(&[source(&order)])
                .unwrap_or_else(|| panic!("the new core takes {order}"));
            let mut found: Vec<Code> = found
                .iter()
                .map(|diagnostic| diagnostic.code)
                .filter(|code| *code != Code::NoEntryInstance)
                .collect();
            found.sort_by_key(|code| format!("{code:?}"));
            let mut expected = codes.to_vec();
            expected.sort_by_key(|code| format!("{code:?}"));
            assert_eq!(found, expected, "{order}");
        }
    }
    let twin = "state x(1)\nstate y = x + 1\nstate s = y.past(1, 0)\nClip k:\n    Note/(s)\n";
    for order in orders(&items(twin)) {
        let found = new
            .checked_by_new_core(&[source(&order)])
            .unwrap_or_else(|| panic!("the new core takes {order}"));
        assert!(
            found
                .iter()
                .all(|diagnostic| diagnostic.code == Code::NoEntryInstance),
            "{order}"
        );
    }
}

fn every_order(items: &[&str]) -> Vec<String> {
    if items.len() <= 1 {
        return vec![items.concat()];
    }
    let mut found = Vec::new();
    for (at, first) in items.iter().enumerate() {
        let mut rest = items.to_vec();
        rest.remove(at);
        for tail in every_order(&rest) {
            found.push(format!("{first}{tail}"));
        }
    }
    found
}

#[test]
fn a_host_fed_init_reading_a_cells_history_draws_reactive_initializer_and_no_declaration_cycle_in_every_order_489()
 {
    let cases: [(&[&str], &[Code]); 5] = [
        (
            &[
                "state recent(recent.past(1, 0) + 1)\n",
                "Clip c:\n    Note/(recent)\n",
            ],
            &[Code::ReactiveInitializer],
        ),
        (
            &[
                "state note(0)\n",
                "state recent(recent.past(1, 0) + note)\n",
                "Clip c:\n    Note/(recent)\n",
            ],
            &[Code::ReactiveInitializer],
        ),
        (
            &[
                "state a(b.past(1, 0) + 1)\n",
                "state b(a.past(1, 0))\n",
                "Clip c:\n    Note/(a + b)\n",
            ],
            &[Code::ReactiveInitializer, Code::ReactiveInitializer],
        ),
        (
            &[
                "state a(b.past(1, 0) + 1)\n",
                "state b(a)\n",
                "Clip c:\n    Note/(a + b)\n",
            ],
            &[Code::ReactiveInitializer, Code::ReactiveInitializer],
        ),
        (
            &[
                "state note(0)\n",
                "state recent(0) = recent.past(1, 0) + note\n",
                "Clip c:\n    Note/(recent)\n",
            ],
            &[],
        ),
    ];
    let new =
        Compiler::new(&symphonyscript_fixtures::vocabulary()).expect("the vocabulary compiles");
    for (items, codes) in cases {
        for text in every_order(items) {
            let found: Vec<Code> = new
                .checked_by_new_core(&[source(&text)])
                .unwrap_or_else(|| panic!("the new core takes {text}"))
                .iter()
                .map(|diagnostic| diagnostic.code)
                .filter(|code| *code != Code::NoEntryInstance && *code != Code::UnreachableInstance)
                .collect();
            assert_eq!(found, codes, "{text}");
        }
    }
}
