use symphonyscript::{Code, Compiler, Severity, Source};
use symphonyscript_core::{Count, Vocabulary};

#[derive(Clone, Copy, PartialEq)]
enum Effect {
    Gives(&'static str),
    Needs,
    Rejects(Code, &'static str),
    Inert(Code, &'static str),
}

struct Position {
    name: &'static str,
    items: &'static [&'static str],
    reactive: Effect,
    compile_time: Effect,
}

const POSITIONS: &[Position] = &[
    Position {
        name: "an enum attribute",
        items: &["Osc o{n} | wave={x}"],
        reactive: Effect::Gives("Wave"),
        compile_time: Effect::Gives("Wave"),
    },
    Position {
        name: "an attribute of the other enum",
        items: &["Osc o{n} | mode={x}"],
        reactive: Effect::Gives("Mode"),
        compile_time: Effect::Gives("Mode"),
    },
    Position {
        name: "`is` with a qualified variant",
        items: &["Osc o{n} when {x} is Wave::Saw"],
        reactive: Effect::Gives("Wave"),
        compile_time: Effect::Gives("Wave"),
    },
    Position {
        name: "`is not` with a qualified variant",
        items: &["Osc o{n} when {x} is not Mode::Saw"],
        reactive: Effect::Gives("Mode"),
        compile_time: Effect::Gives("Mode"),
    },
    Position {
        name: "`is` with a typed value",
        items: &["Osc o{n} when {x} is w"],
        reactive: Effect::Gives("Wave"),
        compile_time: Effect::Gives("Wave"),
    },
    Position {
        name: "a `match` scrutinee and its labels",
        items: &[
            "let l{n} = match {x}:\n    Sine: 1.0\n    else: 0.5",
            "Osc o{n} | freq=l{n}",
        ],
        reactive: Effect::Needs,
        compile_time: Effect::Needs,
    },
    Position {
        name: "an `if` arm",
        items: &["Osc o{n} | wave=(if f then {x} else Wave::Saw)"],
        reactive: Effect::Gives("Wave"),
        compile_time: Effect::Gives("Wave"),
    },
    Position {
        name: "a `match` arm",
        items: &[
            "let q{n} = match f:\n    true: {x}\n    false: Mode::Sine",
            "Osc o{n} | mode=q{n}",
        ],
        reactive: Effect::Gives("Mode"),
        compile_time: Effect::Gives("Mode"),
    },
    Position {
        name: "a pick",
        items: &["Osc o{n} | wave=([{x}, Wave::Saw][i])"],
        reactive: Effect::Gives("Wave"),
        compile_time: Effect::Gives("Wave"),
    },
    Position {
        name: "an array element",
        items: &["Osc o{n} | mode=([{x}, Mode::Saw][0])"],
        reactive: Effect::Gives("Mode"),
        compile_time: Effect::Gives("Mode"),
    },
    Position {
        name: "a function argument used in the body",
        items: &[
            "fn g{n}(v): if v is Wave::Saw then 1.0 else 2.0",
            "Osc o{n} | freq=(g{n}({x}))",
        ],
        reactive: Effect::Gives("Wave"),
        compile_time: Effect::Gives("Wave"),
    },
    Position {
        name: "a pattern argument",
        items: &["pattern P{n}(v):\n    Osc | mode=v", "P{n} p{n} | v={x}"],
        reactive: Effect::Gives("Mode"),
        compile_time: Effect::Gives("Mode"),
    },
    Position {
        name: "an input initializer",
        items: &["input k{n} = {x}"],
        reactive: Effect::Inert(Code::ReactiveInitializer, "{x}"),
        compile_time: Effect::Rejects(Code::UnknownName, "{x}"),
    },
    Position {
        name: "a `for` iterable",
        items: &["Bank b{n}:\n    for e in [{x}]:\n        Osc"],
        reactive: Effect::Inert(Code::ReactiveIterable, "[{x}]"),
        compile_time: Effect::Rejects(Code::UnknownName, "{x}"),
    },
    Position {
        name: "a gate",
        items: &["Osc o{n} when {x}"],
        reactive: Effect::Rejects(Code::ConditionType, "{x}"),
        compile_time: Effect::Rejects(Code::ConditionType, "{x}"),
    },
    Position {
        name: "a numeric attribute",
        items: &["Osc o{n} | freq={x}"],
        reactive: Effect::Rejects(Code::TypeMismatch, "{x}"),
        compile_time: Effect::Rejects(Code::TypeMismatch, "{x}"),
    },
];

const REACTIVE: &str = "let x = if f then Sine else Saw";
const COMPILE_TIME: &str = "let x = Sine";

fn class_vocabulary() -> Vocabulary {
    let mut v = Vocabulary::builder("class");
    let wave = v.enumeration("Wave", ["Sine", "Saw", "Square"]);
    let mode = v.enumeration("Mode", ["Sine", "Saw", "Square"]);
    let osc = v.node("Osc");
    let bank = v.node("Bank");
    v.kind(osc)
        .top_level(Count::Any)
        .attr_enum("wave", &wave, "Sine")
        .attr_enum("mode", &mode, "Sine")
        .attr::<f64>("freq")
        .default(440.0);
    v.kind(bank).top_level(Count::Any).children(osc, Count::Any);
    v.build().expect("the class vocabulary builds")
}

type Found = Vec<(Code, String)>;

fn expected(reactive: bool, read: &str, uses: &[&Position]) -> Found {
    let effects: Vec<Effect> = uses
        .iter()
        .map(|position| {
            if reactive {
                position.reactive
            } else {
                position.compile_time
            }
        })
        .collect();
    let mut enums: Vec<&str> = Vec::new();
    for effect in &effects {
        if let Effect::Gives(name) = effect
            && !enums.contains(name)
        {
            enums.push(name);
        }
    }
    let at = |span: &str| span.replace("{x}", read);
    let mut found = Vec::new();
    for effect in &effects {
        if let Effect::Inert(code, span) = effect {
            found.push((*code, at(span)));
        }
    }
    let typed = enums.len() == 1;
    if reactive {
        let used = effects
            .iter()
            .any(|effect| !matches!(effect, Effect::Inert(..)));
        if typed {
            for effect in &effects {
                if let Effect::Rejects(code, span) = effect {
                    found.push((*code, at(span)));
                }
            }
        } else if used {
            found.push((Code::TypeMismatch, format!("{REACTIVE}\n")));
        }
    } else {
        for effect in &effects {
            match effect {
                Effect::Rejects(code, span) => found.push((*code, at(span))),
                Effect::Needs if !typed => found.push((Code::MatchScrutinee, read.to_string())),
                Effect::Needs | Effect::Gives(_) | Effect::Inert(..) => {}
            }
        }
    }
    found.sort();
    found
}

fn program(reactive: bool, through: bool, uses: &[&Position]) -> Vec<String> {
    let read = if through { "y" } else { "x" };
    let mut items = vec![
        "input f = true".to_string(),
        if reactive { REACTIVE } else { COMPILE_TIME }.to_string(),
    ];
    let written = |needle: &str| {
        uses.iter()
            .any(|position| position.items.iter().any(|item| item.contains(needle)))
    };
    if written(" is w") {
        items.push("input w = Wave::Saw".to_string());
    }
    if written("[i]") {
        items.push("input i = 0".to_string());
    }
    if through {
        items.push("let y = x".to_string());
    }
    for (index, position) in uses.iter().enumerate() {
        let n = (index + 1).to_string();
        items.extend(
            position
                .items
                .iter()
                .map(|item| item.replace("{x}", read).replace("{n}", &n)),
        );
    }
    items
}

fn orders(count: usize) -> Vec<Vec<usize>> {
    let mut order: Vec<usize> = (0..count).collect();
    let mut found = vec![order.clone()];
    let mut counters = vec![0; count];
    let mut level = 0;
    while level < count {
        if counters[level] < level {
            let other = if level % 2 == 0 { 0 } else { counters[level] };
            order.swap(other, level);
            found.push(order.clone());
            counters[level] += 1;
            level = 0;
        } else {
            counters[level] = 0;
            level += 1;
        }
    }
    found
}

fn errors_of(compiler: &Compiler, text: &str) -> Found {
    let source = Source {
        path: "test.sym".into(),
        text: text.into(),
    };
    let mut found: Found = compiler
        .check(&[source])
        .iter()
        .filter(|diagnostic| diagnostic.severity() == Severity::Error)
        .map(|diagnostic| {
            let range = diagnostic.span.range.clone();
            let span =
                usize::try_from(range.start).unwrap_or(0)..usize::try_from(range.end).unwrap_or(0);
            (diagnostic.code, text.get(span).unwrap_or("").to_string())
        })
        .collect();
    found.sort();
    found
}

fn every_flow_meets_its_uses(reactive: bool, through: bool) {
    let compiler = Compiler::new(&class_vocabulary()).expect("the vocabulary compiles");
    let read = if through { "y" } else { "x" };
    let mut cases: Vec<Vec<&Position>> = POSITIONS.iter().map(|position| vec![position]).collect();
    for (first, position) in POSITIONS.iter().enumerate() {
        for other in POSITIONS.iter().skip(first) {
            cases.push(vec![position, other]);
        }
    }
    let mut failures = Vec::new();
    for uses in cases {
        let items = program(reactive, through, &uses);
        let wanted = expected(reactive, read, &uses);
        for order in orders(items.len()) {
            let text: String = order
                .iter()
                .map(|index| format!("{}\n", items[*index]))
                .collect();
            let found = errors_of(&compiler, &text);
            if found != wanted {
                let names: Vec<&str> = uses.iter().map(|position| position.name).collect();
                failures.push(format!(
                    "{names:?}:\n{text}expected {wanted:?}\nfound {found:?}"
                ));
                break;
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} failures:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
fn a_reactive_flow_meets_its_uses_in_every_order() {
    every_flow_meets_its_uses(true, false);
}

#[test]
fn a_reactive_flow_through_a_let_meets_its_uses_in_every_order() {
    every_flow_meets_its_uses(true, true);
}

#[test]
fn a_compile_time_value_meets_its_uses_in_every_order() {
    every_flow_meets_its_uses(false, false);
}

#[test]
fn a_compile_time_value_through_a_let_meets_its_uses_in_every_order() {
    every_flow_meets_its_uses(false, true);
}
