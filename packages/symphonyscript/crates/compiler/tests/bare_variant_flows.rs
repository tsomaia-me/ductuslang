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
    foreign: Option<(Code, &'static str)>,
}

const POSITIONS: &[Position] = &[
    Position {
        name: "an enum attribute",
        items: &["Osc o{n} | wave={x}"],
        reactive: Effect::Gives("Wave"),
        compile_time: Effect::Gives("Wave"),
        foreign: None,
    },
    Position {
        name: "an attribute of the other enum",
        items: &["Osc o{n} | mode={x}"],
        reactive: Effect::Gives("Mode"),
        compile_time: Effect::Gives("Mode"),
        foreign: Some((Code::TypeMismatch, "{x}")),
    },
    Position {
        name: "`is` with a qualified variant",
        items: &["Osc o{n} when {x} is Wave::Saw"],
        reactive: Effect::Gives("Wave"),
        compile_time: Effect::Gives("Wave"),
        foreign: None,
    },
    Position {
        name: "`is not` with a qualified variant",
        items: &["Osc o{n} when {x} is not Mode::{m}"],
        reactive: Effect::Gives("Mode"),
        compile_time: Effect::Gives("Mode"),
        foreign: Some((Code::OperandType, "{x} is not Mode::{m}")),
    },
    Position {
        name: "`is` with a typed value",
        items: &["Osc o{n} when {x} is w"],
        reactive: Effect::Gives("Wave"),
        compile_time: Effect::Gives("Wave"),
        foreign: None,
    },
    Position {
        name: "`is` with a bare partner",
        items: &["Osc o{n} when {x} is Saw"],
        reactive: Effect::Needs,
        compile_time: Effect::Needs,
        foreign: None,
    },
    Position {
        name: "a `match` scrutinee and its labels",
        items: &[
            "let l{n} = match {x}:\n    Sine: 1.0\n    else: 0.5",
            "Osc o{n} | freq=l{n}",
        ],
        reactive: Effect::Needs,
        compile_time: Effect::Needs,
        foreign: None,
    },
    Position {
        name: "an `if` arm",
        items: &["Osc o{n} | wave=(if f then {x} else Wave::Saw)"],
        reactive: Effect::Gives("Wave"),
        compile_time: Effect::Gives("Wave"),
        foreign: None,
    },
    Position {
        name: "a `match` arm",
        items: &[
            "let q{n} = match f:\n    true: {x}\n    false: Mode::{m}",
            "Osc o{n} | mode=q{n}",
        ],
        reactive: Effect::Gives("Mode"),
        compile_time: Effect::Gives("Mode"),
        foreign: Some((
            Code::TypeMismatch,
            "match f:\n    true: {x}\n    false: Mode::{m}\n",
        )),
    },
    Position {
        name: "an array element the index passes over",
        items: &["Osc o{n} | wave=([{x}, Wave::Square][1])"],
        reactive: Effect::Gives("Wave"),
        compile_time: Effect::Gives("Wave"),
        foreign: None,
    },
    Position {
        name: "a pick",
        items: &["Osc o{n} | wave=([{x}, Wave::Saw][i])"],
        reactive: Effect::Gives("Wave"),
        compile_time: Effect::Gives("Wave"),
        foreign: None,
    },
    Position {
        name: "an array element",
        items: &["Osc o{n} | mode=([{x}, Mode::{m}][0])"],
        reactive: Effect::Gives("Mode"),
        compile_time: Effect::Gives("Mode"),
        foreign: Some((Code::TypeMismatch, "{x}")),
    },
    Position {
        name: "a function argument used in the body",
        items: &[
            "fn g{n}(v): if v is Wave::Saw then 1.0 else 2.0",
            "Osc o{n} | freq=(g{n}({x}))",
        ],
        reactive: Effect::Gives("Wave"),
        compile_time: Effect::Gives("Wave"),
        foreign: None,
    },
    Position {
        name: "a pattern argument",
        items: &["pattern P{n}(v):\n    Osc | mode=v", "P{n} p{n} | v={x}"],
        reactive: Effect::Gives("Mode"),
        compile_time: Effect::Gives("Mode"),
        foreign: Some((Code::TypeMismatch, "v")),
    },
    Position {
        name: "an interpolation",
        items: &["Tag t{n} | label=\"{{x}}\""],
        reactive: Effect::Inert(Code::ReactiveString, "{x}"),
        compile_time: Effect::Needs,
        foreign: None,
    },
    Position {
        name: "an input initializer",
        items: &["input k{n} = {x}"],
        reactive: Effect::Inert(Code::ReactiveInitializer, "{x}"),
        compile_time: Effect::Needs,
        foreign: None,
    },
    Position {
        name: "a `for` iterable",
        items: &["Bank b{n}:\n    for e in [{x}]:\n        Osc"],
        reactive: Effect::Inert(Code::ReactiveIterable, "[{x}]"),
        compile_time: Effect::Needs,
        foreign: None,
    },
    Position {
        name: "a gate",
        items: &["Osc o{n} when {x}"],
        reactive: Effect::Rejects(Code::ConditionType, "{x}"),
        compile_time: Effect::Rejects(Code::ConditionType, "{x}"),
        foreign: None,
    },
    Position {
        name: "a numeric attribute",
        items: &["Osc o{n} | freq={x}"],
        reactive: Effect::Rejects(Code::TypeMismatch, "{x}"),
        compile_time: Effect::Rejects(Code::TypeMismatch, "{x}"),
        foreign: None,
    },
];

#[derive(Clone, Copy, PartialEq, Debug)]
enum Route {
    Direct,
    Alias,
    Carrier,
    Writing,
    WritingCompiled,
    Compared,
    Partnered,
    Typed,
    Cycle,
}

const ROUTES: [Route; 9] = [
    Route::Direct,
    Route::Alias,
    Route::Carrier,
    Route::Writing,
    Route::WritingCompiled,
    Route::Compared,
    Route::Partnered,
    Route::Typed,
    Route::Cycle,
];

struct Flow {
    reactive: bool,
    route: Route,
}

impl Flow {
    fn writer(&self) -> &'static str {
        if self.reactive {
            "let x = if f then Sine else Saw"
        } else {
            "let x = Sine"
        }
    }

    fn written(&self) -> Vec<&'static str> {
        let mut variants = if self.reactive {
            vec!["Sine", "Saw"]
        } else {
            vec!["Sine"]
        };
        match self.route {
            Route::Writing | Route::WritingCompiled => variants.push("Square"),
            Route::Compared if self.reactive => variants.extend(["Saw", "Square"]),
            Route::Compared => variants.push("Saw"),
            Route::Direct
            | Route::Alias
            | Route::Carrier
            | Route::Partnered
            | Route::Typed
            | Route::Cycle => {}
        }
        variants
    }

    fn route_items(&self) -> Vec<&'static str> {
        match (self.route, self.reactive) {
            (Route::Direct, _) => Vec::new(),
            (Route::Alias, _) => vec!["let y = x"],
            (Route::Carrier, _) => vec!["let y = if f then x else x"],
            (Route::Writing, _) => vec!["let z = if f then x else Square"],
            (Route::WritingCompiled, _) => vec!["let z = [x, Square][1]"],
            (Route::Partnered, true) => vec![
                "let z = if f then Saw else Square",
                "Osc oz when x is z | wave=z",
            ],
            (Route::Partnered, false) => vec!["let z = Saw", "Osc oz when x is z | wave=z"],
            (Route::Compared, true) => {
                vec!["let z = if f then Saw else Square", "Osc oz when x is z"]
            }
            (Route::Compared, false) => vec!["let z = Saw", "Osc oz when x is z"],
            (Route::Typed, _) => vec!["input t = Wave::Saw", "let z = if f then x else t"],
            (Route::Cycle, true) => vec!["let c = if f then x else c"],
            (Route::Cycle, false) => vec!["let c = [x, c][0]"],
        }
    }

    fn writers(&self) -> Vec<String> {
        let mut writers = vec![self.writer().to_string()];
        if matches!(
            self.route,
            Route::Writing | Route::WritingCompiled | Route::Compared
        ) {
            writers.extend(
                self.route_items()
                    .into_iter()
                    .filter(|item| item.starts_with("let z"))
                    .map(str::to_string),
            );
        }
        writers
    }

    fn read(&self) -> &'static str {
        match self.route {
            Route::Direct | Route::Compared | Route::Partnered | Route::Cycle => "x",
            Route::Alias | Route::Carrier => "y",
            Route::Writing | Route::WritingCompiled | Route::Typed => "z",
        }
    }

    fn reads_a_cell(&self) -> bool {
        match self.route {
            Route::WritingCompiled => false,
            Route::Carrier | Route::Writing | Route::Typed => true,
            Route::Direct | Route::Alias | Route::Compared | Route::Partnered | Route::Cycle => {
                self.reactive
            }
        }
    }
}

struct Vocab {
    vocabulary: Vocabulary,
    mode_variant: &'static str,
    shared: bool,
}

fn vocabulary(shared: bool) -> Vocab {
    let mut v = Vocabulary::builder("flows");
    let wave = v.enumeration("Wave", ["Sine", "Saw", "Square"]);
    let mode = if shared {
        v.enumeration("Mode", ["Sine", "Saw", "Square"])
    } else {
        v.enumeration("Mode", ["Low", "High", "Mid"])
    };
    let osc = v.node("Osc");
    let bank = v.node("Bank");
    let tag = v.node("Tag");
    v.kind(osc)
        .top_level(Count::Any)
        .attr_enum("wave", &wave, "Sine")
        .attr_enum("mode", &mode, if shared { "Sine" } else { "Low" })
        .attr::<f64>("freq")
        .default(440.0);
    v.kind(bank).top_level(Count::Any).children(osc, Count::Any);
    v.kind(tag)
        .top_level(Count::Any)
        .attr::<&'static str>("label")
        .default("");
    Vocab {
        vocabulary: v.build().expect("the flow vocabulary builds"),
        mode_variant: if shared { "Saw" } else { "Low" },
        shared,
    }
}

type Found = Vec<(Code, String)>;

fn expected(flow: &Flow, vocab: &Vocab, uses: &[&Position], text: &str) -> Found {
    let read = flow.read();
    let at = |span: &str| span.replace("{x}", read).replace("{m}", vocab.mode_variant);
    let reactive_read = flow.reads_a_cell();
    let effects: Vec<(Effect, &Position)> = uses
        .iter()
        .map(|position| {
            let effect = if reactive_read {
                position.reactive
            } else {
                position.compile_time
            };
            (effect, *position)
        })
        .collect();
    let mut found = Vec::new();
    for (effect, _) in &effects {
        if let Effect::Inert(code, span) = effect {
            found.push((*code, at(span)));
        }
    }
    if flow.route == Route::Cycle {
        found.push((Code::DeclarationCycle, "c".to_string()));
    }
    if flow.route == Route::Typed {
        for (effect, position) in &effects {
            match effect {
                Effect::Gives("Wave") | Effect::Needs | Effect::Inert(..) => {}
                Effect::Gives(_) => {
                    if let Some((code, span)) = position.foreign {
                        found.push((code, at(span)));
                    }
                }
                Effect::Rejects(code, span) => found.push((*code, at(span))),
            }
        }
        found.sort();
        return found;
    }
    let mut enums: Vec<&str> = Vec::new();
    for (effect, _) in &effects {
        if let Effect::Gives(name) = effect
            && !enums.contains(name)
        {
            enums.push(name);
        }
    }
    if flow.route == Route::Partnered && enums.is_empty() {
        enums.push("Wave");
    }
    let used = matches!(flow.route, Route::Compared | Route::Partnered)
        || effects
            .iter()
            .any(|(effect, _)| !matches!(effect, Effect::Inert(..)));
    if let [single] = enums.as_slice() {
        for (effect, _) in &effects {
            if let Effect::Rejects(code, span) = effect {
                found.push((*code, at(span)));
            }
        }
        if flow.route == Route::Partnered && *single == "Mode" {
            found.push((Code::OperandType, "x is z".to_string()));
        }
        if *single == "Mode" && !vocab.shared {
            for variant in flow.written() {
                found.push((Code::UnknownName, variant.to_string()));
            }
        }
    } else if used {
        let first = flow
            .writers()
            .into_iter()
            .min_by_key(|writer| text.find(&format!("{writer}\n")).unwrap_or(usize::MAX))
            .unwrap_or_default();
        found.push((Code::TypeMismatch, format!("{first}\n")));
    }
    found.sort();
    found
}

fn program(flow: &Flow, vocab: &Vocab, uses: &[&Position]) -> Vec<String> {
    let mut items = vec![flow.writer().to_string()];
    items.extend(flow.route_items().into_iter().map(str::to_string));
    for (index, position) in uses.iter().enumerate() {
        let n = (index + 1).to_string();
        items.extend(position.items.iter().map(|item| {
            item.replace("{x}", flow.read())
                .replace("{n}", &n)
                .replace("{m}", vocab.mode_variant)
        }));
    }
    let written = |needle: &str| items.iter().any(|item| item.contains(needle));
    let mut inputs = Vec::new();
    if written(" f ") || written("if f") || written("match f") {
        inputs.push("input f = true".to_string());
    }
    if written(" is w") {
        inputs.push("input w = Wave::Saw".to_string());
    }
    if written("[i]") {
        inputs.push("input i = 0".to_string());
    }
    inputs.extend(items);
    inputs
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

fn gives(position: &Position, reactive: bool) -> Option<&'static str> {
    match if reactive {
        position.reactive
    } else {
        position.compile_time
    } {
        Effect::Gives(name) => Some(name),
        Effect::Needs | Effect::Rejects(..) | Effect::Inert(..) => None,
    }
}

fn cases(flow: &Flow, shared: bool) -> Vec<Vec<&'static Position>> {
    let reactive = flow.reads_a_cell();
    let anchors: Vec<&Position> = POSITIONS
        .iter()
        .filter(|position| {
            matches!(
                position.items,
                ["Osc o{n} | wave={x}"] | ["Osc o{n} | mode={x}"]
            )
        })
        .collect();
    let mut found = Vec::new();
    for position in POSITIONS {
        if !shared && gives(position, reactive) != Some("Mode") {
            continue;
        }
        found.push(vec![position]);
    }
    for position in POSITIONS {
        for anchor in &anchors {
            let pair = vec![*anchor, position];
            let enums: Vec<Option<&str>> = pair.iter().map(|use_| gives(use_, reactive)).collect();
            if !shared && !matches!(enums.as_slice(), [Some(a), Some(b)] if a != b) {
                continue;
            }
            found.push(pair);
        }
    }
    if shared && flow.route == Route::Direct {
        let unanchored: Vec<&Position> = POSITIONS
            .iter()
            .filter(|position| gives(position, reactive).is_none())
            .collect();
        for (first, position) in unanchored.iter().enumerate() {
            for other in unanchored.iter().skip(first) {
                found.push(vec![*position, *other]);
            }
        }
    }
    found
}

fn every_flow_meets_its_uses(reactive: bool, shared: bool) {
    let failures: Vec<String> = std::thread::scope(|threads| {
        let routes: Vec<_> = ROUTES
            .into_iter()
            .filter(|route| !(reactive && *route == Route::Carrier))
            .map(|route| threads.spawn(move || route_meets_its_uses(reactive, shared, route)))
            .collect();
        routes
            .into_iter()
            .flat_map(|route| {
                route
                    .join()
                    .unwrap_or_else(|_| vec!["a route panicked".to_string()])
            })
            .collect()
    });
    assert!(
        failures.is_empty(),
        "{} failures:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

fn route_meets_its_uses(reactive: bool, shared: bool, route: Route) -> Vec<String> {
    let vocab = vocabulary(shared);
    let compiler = Compiler::new(&vocab.vocabulary).expect("the vocabulary compiles");
    let flow = Flow { reactive, route };
    let mut failures = Vec::new();
    for uses in cases(&flow, shared) {
        let items = program(&flow, &vocab, &uses);
        for order in orders(items.len()) {
            let text: String = order
                .iter()
                .map(|index| format!("{}\n", items[*index]))
                .collect();
            let wanted = expected(&flow, &vocab, &uses, &text);
            let found = errors_of(&compiler, &text);
            if found != wanted {
                let names: Vec<&str> = uses.iter().map(|position| position.name).collect();
                failures.push(format!(
                    "{route:?} {names:?}:\n{text}expected {wanted:?}\nfound {found:?}"
                ));
                break;
            }
        }
    }
    failures
}

#[test]
fn a_reactive_flow_meets_its_uses_in_every_order() {
    every_flow_meets_its_uses(true, true);
}

#[test]
fn a_compile_time_flow_meets_its_uses_in_every_order() {
    every_flow_meets_its_uses(false, true);
}

#[test]
fn a_reactive_flow_meets_enums_of_different_variants_in_every_order() {
    every_flow_meets_its_uses(true, false);
}

#[test]
fn a_compile_time_flow_meets_enums_of_different_variants_in_every_order() {
    every_flow_meets_its_uses(false, false);
}

fn distinct_vocabulary() -> Vocabulary {
    let mut v = Vocabulary::builder("distinct");
    let wave = v.enumeration("Wave", ["Sine", "Saw"]);
    let color = v.enumeration("Color", ["Red", "Blue"]);
    let both = v.node("K");
    v.kind(both)
        .top_level(Count::Any)
        .attr_enum("wave", &wave, "Sine")
        .attr_enum("color", &color, "Red")
        .attr::<bool>("on")
        .default(false);
    v.build().expect("the distinct vocabulary builds")
}

fn located_of(compiler: &Compiler, text: &str) -> Vec<(Code, String, Vec<String>)> {
    let source = Source {
        path: "test.sym".into(),
        text: text.into(),
    };
    let span = |range: std::ops::Range<u32>| {
        let at = usize::try_from(range.start).unwrap_or(0)..usize::try_from(range.end).unwrap_or(0);
        text.get(at).unwrap_or("").to_string()
    };
    compiler
        .check(&[source])
        .iter()
        .filter(|diagnostic| diagnostic.severity() == Severity::Error)
        .map(|diagnostic| {
            let mut secondary: Vec<String> = diagnostic
                .secondary
                .iter()
                .map(|loc| span(loc.range.clone()))
                .collect();
            secondary.sort();
            (
                diagnostic.code,
                span(diagnostic.span.range.clone()),
                secondary,
            )
        })
        .collect()
}

#[test]
fn a_comparison_across_two_enums_and_a_compile_time_flows_lets_are_located_in_every_order() {
    let compiler = Compiler::new(&distinct_vocabulary()).expect("the vocabulary compiles");
    let at = |code: Code, primary: &str, secondary: &[&str]| {
        vec![(
            code,
            primary.to_string(),
            secondary
                .iter()
                .map(|span| (*span).to_string())
                .collect::<Vec<_>>(),
        )]
    };
    let mut failures = Vec::new();
    for (items, wanted) in [
        (
            vec![
                "let a = Sine",
                "let b = Red",
                "K k1 | wave=a color=b on=(a is b)",
            ],
            at(Code::OperandType, "a is b", &[]),
        ),
        (
            vec!["let a = Sine", "let b = Saw", "K k1 | wave=a on=(a is b)"],
            Vec::new(),
        ),
        (
            vec!["let a = Sine", "let b = a", "K k1 | wave=a color=b"],
            at(Code::TypeMismatch, "let a = Sine\n", &["let b = a\n"]),
        ),
        (
            vec![
                "let a = Sine",
                "let b = a",
                "let c = b",
                "K k1 | wave=a color=c",
            ],
            at(
                Code::TypeMismatch,
                "let a = Sine\n",
                &["let b = a\n", "let c = b\n"],
            ),
        ),
    ] {
        for order in orders(items.len()) {
            let text: String = order
                .iter()
                .map(|index| format!("{}\n", items[*index]))
                .collect();
            let found = located_of(&compiler, &text);
            if found != wanted {
                failures.push(format!("{text}found {found:?}"));
                break;
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn two_enums_draw_one_type_mismatch_whatever_their_variants() {
    let compiler = Compiler::new(&distinct_vocabulary()).expect("the vocabulary compiles");
    let conflict = vec![(Code::TypeMismatch, "let a = Sine\n".to_string())];
    let mut failures = Vec::new();
    for items in [
        vec!["let a = Sine", "K k1 | wave=a color=a"],
        vec!["let a = Sine", "let b = a", "K k1 | wave=a color=b"],
    ] {
        for order in orders(items.len()) {
            let text: String = order
                .iter()
                .map(|index| format!("{}\n", items[*index]))
                .collect();
            let found = errors_of(&compiler, &text);
            if found != conflict {
                failures.push(format!("{text}found {found:?}"));
                break;
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn an_untyped_compile_time_flow_draws_one_diagnostic_at_its_writer_in_every_order() {
    let vocab = vocabulary(true);
    let compiler = Compiler::new(&vocab.vocabulary).expect("the vocabulary compiles");
    let written = "let w = Saw";
    let conflict = vec![(Code::TypeMismatch, format!("{written}\n"))];
    let cases: [(&[&str], Found); 7] = [
        (&["Osc o | wave=w", "Osc p | mode=w"], conflict.clone()),
        (&["Osc o | wave=w", "Osc p | wave=w"], Vec::new()),
        (
            &[
                "let l = match w:\n    Saw: 1.0\n    else: 0.5",
                "Osc o | freq=l",
            ],
            conflict.clone(),
        ),
        (&["Osc q when w"], conflict.clone()),
        (
            &[
                "input f = true",
                "let v = if f then w else w",
                "Osc o | mode=v",
                "Osc p | wave=w",
            ],
            conflict.clone(),
        ),
        (
            &["Osc o | wave=w", "Osc p | mode=w", "input k = w"],
            conflict.clone(),
        ),
        (
            &[
                "Osc o | wave=w",
                "Osc p | mode=w",
                "let l = match w:\n    Saw: 1.0\n    else: 0.5",
                "Osc q | freq=l",
            ],
            conflict,
        ),
    ];
    let mut failures = Vec::new();
    for (uses, wanted) in cases {
        let items: Vec<&str> = std::iter::once(written)
            .chain(uses.iter().copied())
            .collect();
        for order in orders(items.len()) {
            let text: String = order
                .iter()
                .map(|index| format!("{}\n", items[*index]))
                .collect();
            let found = errors_of(&compiler, &text);
            if found != wanted {
                failures.push(format!("{text}expected {wanted:?}\nfound {found:?}"));
                break;
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
