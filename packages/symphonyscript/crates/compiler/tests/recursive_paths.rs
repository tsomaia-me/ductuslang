mod support;

use std::collections::{BTreeMap, BTreeSet};

use symphonyscript::{Code, Compiler};
use symphonyscript_fixtures::native_vocabulary;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Path {
    Attribute,
    FirstElement,
    ParameterElement,
    Child,
    Wire,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Binding {
    Constant,
    Default,
    Input,
    ReactiveLet,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Passing {
    Unchanged,
    LocalLet,
    SecondFunction,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Argument {
    Instance,
    LetElement,
    Choice,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Stand {
    Attribute,
    TopLet,
    Pitch,
    InlinedArgument,
}

const PATHS: [Path; 5] = [
    Path::Attribute,
    Path::FirstElement,
    Path::ParameterElement,
    Path::Child,
    Path::Wire,
];

const BINDINGS: [Binding; 4] = [
    Binding::Constant,
    Binding::Default,
    Binding::Input,
    Binding::ReactiveLet,
];

const PASSINGS: [Passing; 3] = [
    Passing::Unchanged,
    Passing::LocalLet,
    Passing::SecondFunction,
];

const ARGUMENTS: [Argument; 3] = [Argument::Instance, Argument::LetElement, Argument::Choice];

const STANDS: [Stand; 4] = [
    Stand::Attribute,
    Stand::TopLet,
    Stand::Pitch,
    Stand::InlinedArgument,
];

const DEPTHS: [u32; 3] = [1, 3, 30];

impl Path {
    fn read(self, reference: &str) -> String {
        match self {
            Path::Attribute => format!("{reference}.gain"),
            Path::FirstElement => format!("{reference}[0].gain"),
            Path::ParameterElement => format!("{reference}[i].gain"),
            Path::Child => format!("{reference}.hook.velocity"),
            Path::Wire => format!("{reference}.back.gain"),
        }
    }

    fn takes_array(self) -> bool {
        matches!(self, Path::FirstElement | Path::ParameterElement)
    }

    fn parameters(self) -> &'static str {
        match self {
            Path::ParameterElement => "c, i, n",
            Path::Attribute | Path::FirstElement | Path::Child | Path::Wire => "c, n",
        }
    }

    fn passed(self, reference: &str) -> String {
        match self {
            Path::ParameterElement => format!("{reference}, i, n - 1"),
            Path::Attribute | Path::FirstElement | Path::Child | Path::Wire => {
                format!("{reference}, n - 1")
            }
        }
    }
}

impl Binding {
    fn value(self) -> Option<&'static str> {
        match self {
            Binding::Constant => Some("2"),
            Binding::Default => None,
            Binding::Input => Some("gv"),
            Binding::ReactiveLet => Some("rl"),
        }
    }

    fn reads_cell(self) -> bool {
        matches!(self, Binding::Input | Binding::ReactiveLet)
    }

    fn expected(self) -> i32 {
        match self {
            Binding::Constant => 2,
            Binding::Default | Binding::Input | Binding::ReactiveLet => 1,
        }
    }
}

struct Case {
    path: Path,
    binding: Binding,
    passing: Passing,
    argument: Argument,
    stand: Stand,
    depth: u32,
    reversed: bool,
}

fn clause(binding: Binding, attribute: &str) -> String {
    binding
        .value()
        .map(|value| format!(" | {attribute}={value}"))
        .unwrap_or_default()
}

fn instances(case: &Case) -> Vec<String> {
    let (clip, child, wire) = match case.path {
        Path::Attribute | Path::FirstElement | Path::ParameterElement => {
            (clause(case.binding, "gain"), String::new(), String::new())
        }
        Path::Child => (
            String::new(),
            clause(case.binding, "velocity"),
            String::new(),
        ),
        Path::Wire => (String::new(), String::new(), clause(case.binding, "gain")),
    };
    vec![
        format!("Clip verse{clip}:\n    Note as hook{child}\n    Plays as back{wire}: other\n"),
        "Clip other:\n    Note\n".to_string(),
    ]
}

fn functions(case: &Case, recursive: bool) -> Vec<String> {
    let parameters = case.path.parameters();
    let (prefix, reference) = match case.passing {
        Passing::LocalLet => ("    let d = c\n", "d"),
        Passing::Unchanged | Passing::SecondFunction => ("", "c"),
    };
    let base = case.path.read(reference);
    let body = |callee: &str| {
        let value = if recursive {
            format!(
                "if n < 1 then {base} else {callee}({})",
                case.path.passed(reference)
            )
        } else {
            base.clone()
        };
        format!("fn {{}}({parameters}):\n{prefix}    {value}\n")
    };
    let mut items = Vec::new();
    match case.passing {
        Passing::SecondFunction if recursive => {
            items.push(body("h").replacen("{}", "g", 1));
            items.push(body("g").replacen("{}", "h", 1));
        }
        Passing::SecondFunction | Passing::Unchanged | Passing::LocalLet => {
            items.push(body("g").replacen("{}", "g", 1));
        }
    }
    items
}

fn call(case: &Case) -> String {
    let reference = match case.argument {
        Argument::Instance => "verse",
        Argument::LetElement => "refs[0]",
        Argument::Choice => "(if 1 > 0 then verse else other)",
    };
    let reference = if case.path.takes_array() {
        format!("[{reference}]")
    } else {
        reference.to_string()
    };
    match case.path {
        Path::ParameterElement => format!("g({reference}, 0, {})", case.depth),
        Path::Attribute | Path::FirstElement | Path::Child | Path::Wire => {
            format!("g({reference}, {})", case.depth)
        }
    }
}

fn program(case: &Case, recursive: bool) -> String {
    let call = call(case);
    let mut items = vec![
        "Session main\n".to_string(),
        "input gv = 1.0\n".to_string(),
        "let rl = gv * 1\n".to_string(),
        "let refs = [verse, other]\n".to_string(),
    ];
    items.extend(instances(case));
    items.extend(functions(case, recursive));
    let expected = case.binding.expected();
    match case.stand {
        Stand::Attribute => items.push(format!("Clip k | gain={call}:\n    Note\n")),
        Stand::TopLet => {
            items.push(format!("let t = {call}\n"));
            items.push("Clip k | gain=t:\n    Note\n".to_string());
        }
        Stand::Pitch => items.push(format!(
            "Clip k:\n    Note/([60][i32({call}) - {expected}])\n"
        )),
        Stand::InlinedArgument => {
            items.push("fn pass(x): x\n".to_string());
            items.push(format!("Clip k | gain=pass({call}):\n    Note\n"));
        }
    }
    if case.reversed {
        items.reverse();
    }
    items.concat()
}

type Found = BTreeMap<(Code, String), usize>;

fn in_parallel<T: Copy + Send + Sync>(
    items: &[T],
    rows: impl Fn(T) -> (Vec<String>, usize) + Sync,
) -> (Vec<String>, usize) {
    std::thread::scope(|threads| {
        let workers: Vec<_> = items
            .iter()
            .map(|item| {
                let rows = &rows;
                threads.spawn(move || rows(*item))
            })
            .collect();
        let mut failures = Vec::new();
        let mut programs = 0;
        for worker in workers {
            let (found, counted) = worker
                .join()
                .unwrap_or_else(|_| (vec!["a worker panicked".to_string()], 0));
            failures.extend(found);
            programs += counted;
        }
        (failures, programs)
    })
}

fn found(text: &str) -> Found {
    let mut found = Found::new();
    for diagnostic in support::checked(text) {
        let range = diagnostic.span.range;
        let at = text
            .get(range.start as usize..range.end as usize)
            .unwrap_or("?")
            .to_string();
        *found.entry((diagnostic.code, at)).or_default() += 1;
    }
    found
}

#[test]
fn a_path_through_a_node_reference_parameter_of_a_recursive_function_reads_as_at_top_level() {
    let mut failures = Vec::new();
    let mut programs = 0;
    for path in PATHS {
        for binding in BINDINGS {
            for passing in PASSINGS {
                for argument in ARGUMENTS {
                    for stand in STANDS {
                        for depth in DEPTHS {
                            for reversed in [false, true] {
                                let case = Case {
                                    path,
                                    binding,
                                    passing,
                                    argument,
                                    stand,
                                    depth,
                                    reversed,
                                };
                                let text = program(&case, true);
                                let twin = program(&case, false);
                                let mut expected = found(&twin);
                                if binding.reads_cell() {
                                    expected.retain(|(code, _), _| *code != Code::IndexOutOfRange);
                                    *expected
                                        .entry((Code::ReactiveRecursion, call(&case)))
                                        .or_default() += 1;
                                }
                                let found = found(&text);
                                programs += 1;
                                if found != expected {
                                    failures.push(format!(
                                        "{path:?} {binding:?} {passing:?} {argument:?} {stand:?} {depth} {reversed}\n{text}\n  found {found:?}\n  twin {expected:?}\n"
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {programs} programs fail:\n{}",
        failures.len(),
        failures.iter().take(8).cloned().collect::<String>()
    );
    assert_eq!(programs, 4_320);
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Chooser {
    IfOnParameter,
    IfOnDepth,
    ParameterArray,
    LiteralOfParameters,
    LiteralOfInstances,
    TopLevelLet,
    FromEnd,
    NestedIf,
    IndexedChoice,
    SliceFrom,
    SliceTo,
    SliceFromEnd,
    TopLevelLetSlice,
    LocalMatch,
    MatchHelper,
    IfHelper,
    RecursiveHelper,
    MatchOfArrays,
    IfInMatch,
    JoinedLiterals,
    IfOverArrays,
    NestedArray,
}

const CHOOSERS: [Chooser; 22] = [
    Chooser::IfOnParameter,
    Chooser::IfOnDepth,
    Chooser::ParameterArray,
    Chooser::LiteralOfParameters,
    Chooser::LiteralOfInstances,
    Chooser::TopLevelLet,
    Chooser::FromEnd,
    Chooser::NestedIf,
    Chooser::IndexedChoice,
    Chooser::SliceFrom,
    Chooser::SliceTo,
    Chooser::SliceFromEnd,
    Chooser::TopLevelLetSlice,
    Chooser::LocalMatch,
    Chooser::MatchHelper,
    Chooser::IfHelper,
    Chooser::RecursiveHelper,
    Chooser::MatchOfArrays,
    Chooser::IfInMatch,
    Chooser::JoinedLiterals,
    Chooser::IfOverArrays,
    Chooser::NestedArray,
];

impl Chooser {
    fn expression(self) -> &'static str {
        match self {
            Chooser::IfOnParameter => "(if i > 0 then d else c)",
            Chooser::IfOnDepth => "(if n < 0 then c else d)",
            Chooser::ParameterArray => "xs[i]",
            Chooser::LiteralOfParameters => "[c, d][i]",
            Chooser::LiteralOfInstances => "[verse, other][i]",
            Chooser::TopLevelLet => "rs[i]",
            Chooser::FromEnd => "xs[^i]",
            Chooser::NestedIf => "(if i > 0 then (if n < 0 then c else d) else c)",
            Chooser::IndexedChoice => "[(if i > 0 then d else c), c][0]",
            Chooser::SliceFrom => "xs[i..][0]",
            Chooser::SliceTo => "xs[..i + 1][1]",
            Chooser::SliceFromEnd => "xs[^i..][0]",
            Chooser::TopLevelLetSlice => "rs[i..][0]",
            Chooser::LocalMatch | Chooser::IfInMatch => "m",
            Chooser::MatchHelper => "selm(c, d, i)",
            Chooser::IfHelper => "sel(c, d, i)",
            Chooser::RecursiveHelper => "pickr(c, d, i, 2)",
            Chooser::MatchOfArrays => "m[1]",
            Chooser::JoinedLiterals => "([c] + [d])[i]",
            Chooser::IfOverArrays => "(if i > 0 then rs else [other, verse])[0]",
            Chooser::NestedArray => "[[c, d], [d, c]][i][0]",
        }
    }

    fn indices(self) -> &'static [usize] {
        match self {
            Chooser::FromEnd | Chooser::SliceFromEnd => &[2, 1],
            Chooser::SliceTo | Chooser::IfOnDepth => &[1],
            Chooser::IfOnParameter
            | Chooser::ParameterArray
            | Chooser::LiteralOfParameters
            | Chooser::LiteralOfInstances
            | Chooser::TopLevelLet
            | Chooser::NestedIf
            | Chooser::IndexedChoice
            | Chooser::SliceFrom
            | Chooser::TopLevelLetSlice
            | Chooser::LocalMatch
            | Chooser::MatchHelper
            | Chooser::IfHelper
            | Chooser::RecursiveHelper
            | Chooser::MatchOfArrays
            | Chooser::IfInMatch
            | Chooser::JoinedLiterals
            | Chooser::IfOverArrays
            | Chooser::NestedArray => &[0, 1],
        }
    }

    fn depths(self) -> &'static [i32] {
        match self {
            Chooser::IfOnDepth | Chooser::NestedIf | Chooser::IfInMatch => &[-1, 1, 3, 30],
            Chooser::IfOnParameter
            | Chooser::ParameterArray
            | Chooser::LiteralOfParameters
            | Chooser::LiteralOfInstances
            | Chooser::TopLevelLet
            | Chooser::FromEnd
            | Chooser::IndexedChoice
            | Chooser::SliceFrom
            | Chooser::SliceTo
            | Chooser::SliceFromEnd
            | Chooser::TopLevelLetSlice
            | Chooser::LocalMatch
            | Chooser::MatchHelper
            | Chooser::IfHelper
            | Chooser::RecursiveHelper
            | Chooser::MatchOfArrays
            | Chooser::JoinedLiterals
            | Chooser::IfOverArrays
            | Chooser::NestedArray => &[1, 3, 30],
        }
    }

    fn chooses_other(self, index: usize, depth: i32) -> bool {
        let negative = depth < 0;
        match self {
            Chooser::IfOnDepth => !negative,
            Chooser::NestedIf | Chooser::IfInMatch => index > 0 && !negative,
            Chooser::FromEnd | Chooser::SliceFromEnd => index == 1,
            Chooser::SliceTo => true,
            Chooser::IfOverArrays => index == 0,
            Chooser::IfOnParameter
            | Chooser::ParameterArray
            | Chooser::LiteralOfParameters
            | Chooser::LiteralOfInstances
            | Chooser::TopLevelLet
            | Chooser::IndexedChoice
            | Chooser::SliceFrom
            | Chooser::TopLevelLetSlice
            | Chooser::LocalMatch
            | Chooser::MatchHelper
            | Chooser::IfHelper
            | Chooser::RecursiveHelper
            | Chooser::MatchOfArrays
            | Chooser::JoinedLiterals
            | Chooser::NestedArray => index > 0,
        }
    }

    fn locals(self) -> &'static str {
        match self {
            Chooser::LocalMatch => "    let m = match i:\n        0: c\n        else: d\n",
            Chooser::MatchOfArrays => {
                "    let m = match i:\n        0: [d, c]\n        else: [c, d]\n"
            }
            Chooser::IfInMatch => {
                "    let m = match i:\n        0: c\n        else: (if n < 0 then c else d)\n"
            }
            Chooser::IfOnParameter
            | Chooser::IfOnDepth
            | Chooser::ParameterArray
            | Chooser::LiteralOfParameters
            | Chooser::LiteralOfInstances
            | Chooser::TopLevelLet
            | Chooser::FromEnd
            | Chooser::NestedIf
            | Chooser::IndexedChoice
            | Chooser::SliceFrom
            | Chooser::SliceTo
            | Chooser::SliceFromEnd
            | Chooser::TopLevelLetSlice
            | Chooser::MatchHelper
            | Chooser::IfHelper
            | Chooser::RecursiveHelper
            | Chooser::JoinedLiterals
            | Chooser::IfOverArrays
            | Chooser::NestedArray => "",
        }
    }
}

const HELPERS: [&str; 6] = [
    "fn f(c): c.gain\n",
    "fn g(c, n): if n < 1 then c.gain else g(c, n - 1)\n",
    "fn gb(c, n): if n < 1 then c.back.gain else gb(c, n - 1)\n",
    "fn sel(c, d, p): if p > 0 then d else c\n",
    "fn selm(c, d, p):\n    match p:\n        0: c\n        else: d\n",
    "fn pickr(c, d, p, m): if m < 1 then (if p > 0 then d else c) else pickr(c, d, p, m - 1)\n",
];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Consumer {
    Path,
    Child,
    Wire,
    LocalLet,
    InlinedCall,
    RecursiveArgument,
    RecursiveWireArgument,
}

const CONSUMERS: [Consumer; 7] = [
    Consumer::Path,
    Consumer::Child,
    Consumer::Wire,
    Consumer::LocalLet,
    Consumer::InlinedCall,
    Consumer::RecursiveArgument,
    Consumer::RecursiveWireArgument,
];

impl Consumer {
    fn read(self, chosen: &str) -> String {
        match self {
            Consumer::Path => format!("({chosen}).gain"),
            Consumer::Child => format!("({chosen}).hook.velocity"),
            Consumer::Wire => format!("({chosen}).back.gain"),
            Consumer::LocalLet => "e.gain".to_string(),
            Consumer::InlinedCall => format!("f({chosen})"),
            Consumer::RecursiveArgument => format!("g({chosen}, 2)"),
            Consumer::RecursiveWireArgument => format!("gb({chosen}, 2)"),
        }
    }

    fn local(self, chosen: &str) -> String {
        match self {
            Consumer::LocalLet => format!("    let e = ({chosen})\n"),
            Consumer::Path
            | Consumer::Child
            | Consumer::Wire
            | Consumer::InlinedCall
            | Consumer::RecursiveArgument
            | Consumer::RecursiveWireArgument => String::new(),
        }
    }
}

const CHOSEN_BINDINGS: [Binding; 2] = [Binding::Constant, Binding::Default];

fn chosen_clause(binding: Binding, attribute: &str) -> String {
    match binding {
        Binding::Constant => format!(" | {attribute}=3"),
        Binding::Default | Binding::Input | Binding::ReactiveLet => String::new(),
    }
}

fn chosen_value(binding: Binding) -> i32 {
    match binding {
        Binding::Constant => 3,
        Binding::Default | Binding::Input | Binding::ReactiveLet => 1,
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Levels {
    Base(i32),
    Alternating(usize, i32),
}

const ALTERNATING_DEPTHS: [i32; 3] = [1, 3, 30];

impl Levels {
    fn depth(self) -> i32 {
        match self {
            Levels::Base(depth) | Levels::Alternating(_, depth) => depth,
        }
    }

    fn reads(self, index: usize) -> Vec<(usize, i32)> {
        match self {
            Levels::Base(depth) => vec![(index, depth)],
            Levels::Alternating(sum, depth) => (0..depth)
                .map(|level| {
                    let chosen = if level % 2 == 0 { index } else { sum - index };
                    (chosen, depth - level)
                })
                .collect(),
        }
    }

    fn functions(
        self,
        (params, locals, read): (&str, &str, &str),
        args: impl Fn(&str) -> String,
        recursive: bool,
    ) -> String {
        match self {
            Levels::Base(_) if recursive => format!(
                "fn h({params}):\n{locals}    if n < 1 then {read} else h({})\n",
                args("i")
            ),
            Levels::Base(_) => format!("fn h({params}):\n{locals}    {read}\n"),
            Levels::Alternating(sum, _) if recursive => format!(
                "fn h({params}):\n{locals}    if n < 1 then 0 else {read} + h({})\n",
                args(&format!("{sum} - i"))
            ),
            Levels::Alternating(sum, depth) => {
                let depth = usize::try_from(depth).unwrap_or_default();
                let next = args(&format!("{sum} - i"));
                let name = |level: usize| match level {
                    0 => "h".to_string(),
                    _ => format!("h{level}"),
                };
                (0..=depth)
                    .map(|level| {
                        let rest = if level < depth {
                            format!("{}({next})", name(level + 1))
                        } else {
                            "0".to_string()
                        };
                        format!(
                            "fn {}({params}):\n{locals}    if n < 1 then 0 else {read} + {rest}\n",
                            name(level)
                        )
                    })
                    .collect()
            }
        }
    }

    fn of(indices: &[usize], depths: &[i32]) -> Vec<(usize, Levels)> {
        let mut values = Vec::new();
        for index in indices {
            for depth in depths {
                values.push((*index, Levels::Base(*depth)));
            }
        }
        if let [first, second] = indices {
            for index in indices {
                for depth in ALTERNATING_DEPTHS {
                    values.push((*index, Levels::Alternating(first + second, depth)));
                }
            }
        }
        values
    }
}

fn chosen_function(
    chooser: Chooser,
    (local, read): (&str, &str),
    levels: Levels,
    recursive: bool,
) -> String {
    levels.functions(
        (
            "c, d, xs, i, n",
            &format!("{}{local}", chooser.locals()),
            read,
        ),
        |next| format!("c, d, xs, {next}, n - 1"),
        recursive,
    )
}

fn chosen_program(
    (chooser, consumer, binding): (Chooser, Consumer, Binding),
    (scale, index): (i32, usize),
    (levels, reversed, recursive): (Levels, bool, bool),
) -> String {
    let chosen = chooser.expression();
    let local = consumer.local(chosen);
    let read = consumer.read(chosen);
    let mut items = vec![
        "Session main\n".to_string(),
        "Clip verse | gain=5:\n    Note as hook | velocity=5\n    Plays as back | gain=5: tail\n"
            .to_string(),
        format!(
            "Clip other{}:\n    Note as hook{}\n    Plays as back{}: tail\n",
            chosen_clause(binding, "gain"),
            chosen_clause(binding, "velocity"),
            chosen_clause(binding, "gain"),
        ),
        "Clip tail:\n    Note\n".to_string(),
        "let rs = [verse, other]\n".to_string(),
    ];
    items.extend(HELPERS.iter().map(|helper| helper.to_string()));
    items.push(chosen_function(chooser, (&local, &read), levels, recursive));
    let call = format!(
        "h(verse, other, [verse, other], {index}, {})",
        levels.depth()
    );
    let read = match scale {
        1 => call,
        _ => format!("{call} * {scale}"),
    };
    let offset: i32 = levels
        .reads(index)
        .into_iter()
        .map(|(chosen, depth)| {
            if chooser.chooses_other(chosen, depth) {
                chosen_value(binding)
            } else {
                5
            }
        })
        .sum();
    items.push(format!(
        "Clip k:\n    Note/([60][i32({read}) - {}])\n",
        offset * scale
    ));
    if reversed {
        items.reverse();
    }
    items.concat()
}

fn frame_values(chooser: Chooser) -> Vec<(usize, Levels)> {
    Levels::of(chooser.indices(), chooser.depths())
}

fn codes_at(text: &str, levels: Levels) -> Found {
    let mut drawn = found(text);
    if let Levels::Alternating(..) = levels {
        drawn.values_mut().for_each(|count| *count = 1);
    }
    drawn
}

fn twin_failure(text: &str, twin: &str, levels: Levels) -> Option<String> {
    let expected = codes_at(twin, levels);
    let drawn = codes_at(text, levels);
    if expected
        .keys()
        .any(|(code, _)| *code == Code::IndexOutOfRange)
    {
        return Some(format!(
            "{twin}\n  the twin reads a wrong value: {expected:?}\n"
        ));
    }
    (drawn != expected).then(|| format!("{text}\n  found {drawn:?}\n  twin {expected:?}\n"))
}

#[test]
fn a_reference_a_recursive_body_chooses_by_a_frame_value_reads_the_chosen_reference_at_every_consumer()
 {
    let (failures, programs) = in_parallel(&CHOOSERS, |chooser| {
        let mut failures = Vec::new();
        let mut programs = 0;
        for consumer in CONSUMERS {
            for binding in CHOSEN_BINDINGS {
                for scale in [1, 2] {
                    for (index, levels) in frame_values(chooser) {
                        for reversed in [false, true] {
                            let case = (chooser, consumer, binding);
                            let frame = (scale, index);
                            let text = chosen_program(case, frame, (levels, reversed, true));
                            let twin = chosen_program(case, frame, (levels, reversed, false));
                            programs += 1;
                            if let Some(failure) = twin_failure(&text, &twin, levels) {
                                failures.push(format!(
                                    "{chooser:?} {consumer:?} {binding:?} {scale} {index} {levels:?} {reversed}\n{failure}"
                                ));
                            }
                        }
                    }
                }
            }
        }
        (failures, programs)
    });
    assert!(
        failures.is_empty(),
        "{} of {programs} programs fail:\n{}",
        failures.len(),
        failures.iter().take(8).cloned().collect::<String>()
    );
    assert_eq!(programs, 14_056);
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum ArrayAttribute {
    VariableSet,
    VariableDefault,
    VariableWithDefaultSet,
    VariableWithDefault,
    FixedSet,
    FixedDefault,
    RowsSet,
    RowsDefault,
}

const ARRAY_ATTRIBUTES: [ArrayAttribute; 8] = [
    ArrayAttribute::VariableSet,
    ArrayAttribute::VariableDefault,
    ArrayAttribute::VariableWithDefaultSet,
    ArrayAttribute::VariableWithDefault,
    ArrayAttribute::FixedSet,
    ArrayAttribute::FixedDefault,
    ArrayAttribute::RowsSet,
    ArrayAttribute::RowsDefault,
];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum ArrayRead {
    Length,
    First,
    Last,
    SliceThenFirst,
    JoinThenFirst,
}

const ARRAY_READS: [ArrayRead; 5] = [
    ArrayRead::Length,
    ArrayRead::First,
    ArrayRead::Last,
    ArrayRead::SliceThenFirst,
    ArrayRead::JoinThenFirst,
];

impl ArrayAttribute {
    fn name(self) -> &'static str {
        match self {
            ArrayAttribute::VariableSet | ArrayAttribute::VariableDefault => "sequence",
            ArrayAttribute::VariableWithDefaultSet | ArrayAttribute::VariableWithDefault => "seq",
            ArrayAttribute::FixedSet | ArrayAttribute::FixedDefault => "trio",
            ArrayAttribute::RowsSet | ArrayAttribute::RowsDefault => "rows",
        }
    }

    fn chosen(self) -> Option<&'static str> {
        match self {
            ArrayAttribute::VariableSet | ArrayAttribute::VariableWithDefaultSet => Some("[1, 2]"),
            ArrayAttribute::FixedSet => Some("[1, 2, 3]"),
            ArrayAttribute::RowsSet => Some("[[1, 2]]"),
            ArrayAttribute::VariableDefault
            | ArrayAttribute::VariableWithDefault
            | ArrayAttribute::FixedDefault
            | ArrayAttribute::RowsDefault => None,
        }
    }

    fn other(self) -> &'static str {
        match self {
            ArrayAttribute::VariableSet
            | ArrayAttribute::VariableDefault
            | ArrayAttribute::FixedSet
            | ArrayAttribute::FixedDefault => "[4, 5, 6]",
            ArrayAttribute::VariableWithDefaultSet | ArrayAttribute::VariableWithDefault => {
                "[4, 5, 6, 7]"
            }
            ArrayAttribute::RowsSet | ArrayAttribute::RowsDefault => "[[4, 5], [6, 7], [8, 9]]",
        }
    }

    fn rows(self) -> Vec<Vec<i32>> {
        match self {
            ArrayAttribute::VariableSet | ArrayAttribute::VariableWithDefaultSet => {
                vec![vec![1], vec![2]]
            }
            ArrayAttribute::VariableDefault => Vec::new(),
            ArrayAttribute::VariableWithDefault => vec![vec![7], vec![8]],
            ArrayAttribute::FixedSet => vec![vec![1], vec![2], vec![3]],
            ArrayAttribute::FixedDefault => vec![vec![0], vec![0], vec![0]],
            ArrayAttribute::RowsSet => vec![vec![1, 2]],
            ArrayAttribute::RowsDefault => vec![vec![1, 2], vec![3, 4]],
        }
    }

    fn nested(self) -> bool {
        matches!(self, ArrayAttribute::RowsSet | ArrayAttribute::RowsDefault)
    }

    fn read(self, read: ArrayRead, value: &str) -> String {
        let leaf = if self.nested() { "[1]" } else { "" };
        let joined = if self.nested() { "[[9, 9]]" } else { "[9]" };
        match read {
            ArrayRead::Length => format!("len({value})"),
            ArrayRead::First => format!("{value}[0]{leaf}"),
            ArrayRead::Last => format!("{value}[^1]{leaf}"),
            ArrayRead::SliceThenFirst => format!("{value}[0..][0]{leaf}"),
            ArrayRead::JoinThenFirst => format!("({value} + {joined})[0]{leaf}"),
        }
    }

    fn verse_rows(self) -> Vec<Vec<i32>> {
        match self {
            ArrayAttribute::VariableSet
            | ArrayAttribute::VariableDefault
            | ArrayAttribute::FixedSet
            | ArrayAttribute::FixedDefault => vec![vec![4], vec![5], vec![6]],
            ArrayAttribute::VariableWithDefaultSet | ArrayAttribute::VariableWithDefault => {
                vec![vec![4], vec![5], vec![6], vec![7]]
            }
            ArrayAttribute::RowsSet | ArrayAttribute::RowsDefault => {
                vec![vec![4, 5], vec![6, 7], vec![8, 9]]
            }
        }
    }

    fn expected(self, read: ArrayRead, other: bool) -> Option<i32> {
        let rows = if other {
            self.rows()
        } else {
            self.verse_rows()
        };
        let leaf = |row: &Vec<i32>| row.get(usize::from(self.nested())).copied();
        match read {
            ArrayRead::Length => i32::try_from(rows.len()).ok(),
            ArrayRead::First | ArrayRead::SliceThenFirst | ArrayRead::JoinThenFirst => {
                rows.first().and_then(leaf)
            }
            ArrayRead::Last => rows.last().and_then(leaf),
        }
    }
}

const ARRAY_CONSUMERS: [Consumer; 2] = [Consumer::Path, Consumer::LocalLet];

fn array_program(
    chooser: Chooser,
    consumer: Consumer,
    (attribute, read): (ArrayAttribute, ArrayRead),
    (index, levels, reversed, recursive): (usize, Levels, bool, bool),
    expected: i32,
) -> String {
    let chosen = chooser.expression();
    let name = attribute.name();
    let value = match consumer {
        Consumer::LocalLet => format!("e.{name}"),
        Consumer::Path
        | Consumer::Child
        | Consumer::Wire
        | Consumer::InlinedCall
        | Consumer::RecursiveArgument
        | Consumer::RecursiveWireArgument => format!("({chosen}).{name}"),
    };
    let local = consumer.local(chosen);
    let read = attribute.read(read, &value);
    let chosen_clause = attribute
        .chosen()
        .map(|literal| format!(" | {name}={literal}"))
        .unwrap_or_default();
    let mut items = [
        format!("Clip verse | {name}={}:\n    Note\n", attribute.other()),
        format!("Clip other{chosen_clause}:\n    Note\n"),
        "let rs = [verse, other]\n".to_string(),
        "fn sel(c, d, p): if p > 0 then d else c\n".to_string(),
        "fn selm(c, d, p):\n    match p:\n        0: c\n        else: d\n".to_string(),
        "fn pickr(c, d, p, m): if m < 1 then (if p > 0 then d else c) else pickr(c, d, p, m - 1)\n"
            .to_string(),
        chosen_function(chooser, (&local, &read), levels, recursive),
        format!(
            "Song s:\n    Clip:\n        Note/([60][h(verse, other, [verse, other], {index}, {}) - {expected}])\n",
            levels.depth()
        ),
    ];
    if reversed {
        items.reverse();
    }
    items.concat()
}

fn found_with(compiler: &Compiler, text: &str) -> Found {
    let mut found = Found::new();
    for (code, at) in support::error_texts(compiler, text) {
        *found.entry((code, at)).or_default() += 1;
    }
    found
}

#[test]
fn an_array_attribute_read_through_a_reference_a_recursive_body_chooses_has_the_attributes_type_and_layout()
 {
    let (failures, programs) = in_parallel(&CHOOSERS, |chooser| {
        let compiler = Compiler::new(&native_vocabulary()).expect("the native vocabulary compiles");
        let mut failures = Vec::new();
        let mut programs = 0;
        for consumer in ARRAY_CONSUMERS {
            for attribute in ARRAY_ATTRIBUTES {
                for read in ARRAY_READS {
                    for (index, levels) in frame_values(chooser) {
                        let Some(expected) = levels
                            .reads(index)
                            .into_iter()
                            .map(|(chosen, depth)| {
                                attribute.expected(read, chooser.chooses_other(chosen, depth))
                            })
                            .sum::<Option<i32>>()
                        else {
                            continue;
                        };
                        for reversed in [false, true] {
                            let program = |recursive| {
                                array_program(
                                    chooser,
                                    consumer,
                                    (attribute, read),
                                    (index, levels, reversed, recursive),
                                    expected,
                                )
                            };
                            let (text, twin) = (program(true), program(false));
                            let drawn = found_with(&compiler, &text);
                            let twin_drawn = found_with(&compiler, &twin);
                            programs += 1;
                            if twin_drawn
                                .keys()
                                .any(|(code, _)| *code == Code::IndexOutOfRange)
                                || drawn != twin_drawn
                            {
                                failures.push(format!(
                                    "{chooser:?} {consumer:?} {attribute:?} {read:?} {index} {levels:?} {reversed}\n{text}\n  found {drawn:?}\n  twin {twin_drawn:?}\n"
                                ));
                            }
                        }
                    }
                }
            }
        }
        (failures, programs)
    });
    assert!(
        failures.is_empty(),
        "{} of {programs} programs fail:\n{}",
        failures.len(),
        failures.iter().take(8).cloned().collect::<String>()
    );
    assert_eq!(programs, 37_504);
}

#[test]
fn a_reference_argument_an_error_leaves_unknown_draws_what_its_twin_draws() {
    let header = "Session main\nClip verse | gain=2:\n    Note\nClip other | gain=3:\n    Note\n";
    let mut failures = Vec::new();
    for argument in ["[verse, other][5]", "[verse, nothing][1]", "xs[i]"] {
        for reversed in [false, true] {
            let program = |recursive: bool| {
                let callee = if recursive {
                    "fn g(c, n): if n < 1 then c.gain else g(c, n - 1)\n"
                } else {
                    "fn g(c, n): c.gain\n"
                };
                let caller = if recursive {
                    format!("fn h(xs, i, n): if n < 1 then g({argument}, 2) else h(xs, i, n - 1)\n")
                } else {
                    format!("fn h(xs, i, n): g({argument}, 2)\n")
                };
                let mut items = [
                    header.to_string(),
                    callee.to_string(),
                    caller,
                    "Clip k | gain=h([verse, other], 5, 3):\n    Note\n".to_string(),
                ];
                if reversed {
                    items.reverse();
                }
                items.concat()
            };
            let (text, twin) = (program(true), program(false));
            let (drawn, expected) = (found(&text), found(&twin));
            if drawn != expected || expected.is_empty() {
                failures.push(format!("{text}\n  found {drawn:?}\n  twin {expected:?}\n"));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.concat());
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Start {
    Element,
    FromEnd,
    LiteralOfOne,
    LiteralOfTwo,
    TopLevelLet,
    SliceThenFirst,
    NestedLiteral,
    SaturatingIndex,
    RecursiveHelper,
    RecursiveArrayHelper,
}

const STARTS: [Start; 10] = [
    Start::Element,
    Start::FromEnd,
    Start::LiteralOfOne,
    Start::LiteralOfTwo,
    Start::TopLevelLet,
    Start::SliceThenFirst,
    Start::NestedLiteral,
    Start::SaturatingIndex,
    Start::RecursiveHelper,
    Start::RecursiveArrayHelper,
];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Fault {
    Outside,
    Saturating,
    Depth,
    Inside(usize),
}

const FAULTS: [Fault; 5] = [
    Fault::Outside,
    Fault::Saturating,
    Fault::Depth,
    Fault::Inside(0),
    Fault::Inside(1),
];

const GAINS: [i32; 2] = [2, 3];

impl Fault {
    fn code(self) -> Option<Code> {
        match self {
            Fault::Outside => Some(Code::IndexOutOfRange),
            Fault::Saturating => Some(Code::ConstantOverflow),
            Fault::Depth => Some(Code::RecursionDepth),
            Fault::Inside(_) => None,
        }
    }
}

impl Start {
    fn expression(self, candidates: usize, fault: Fault) -> String {
        let two = candidates == 2;
        let other = if two { "d" } else { "c" };
        let depth = if fault == Fault::Depth { 5000 } else { 2 };
        match self {
            Start::Element => "xs[i]".to_string(),
            Start::FromEnd => "xs[^i]".to_string(),
            Start::LiteralOfOne if two => "[c, d][i]".to_string(),
            Start::LiteralOfOne => "[c][i]".to_string(),
            Start::LiteralOfTwo if two => "[c, d, c][i]".to_string(),
            Start::LiteralOfTwo => "[c, c][i]".to_string(),
            Start::TopLevelLet => "rs[i]".to_string(),
            Start::SliceThenFirst => "xs[i..][0]".to_string(),
            Start::NestedLiteral if two => "[[c, d]][0][i]".to_string(),
            Start::NestedLiteral => "[[c]][0][i]".to_string(),
            Start::SaturatingIndex => "xs[i32(i) * 2147483647]".to_string(),
            Start::RecursiveHelper => format!("pickr(c, {other}, i, {depth})"),
            Start::RecursiveArrayHelper => format!("pk(c, {other}, {depth})[i]"),
        }
    }

    fn frame_value(self, candidates: usize, fault: Fault) -> Option<i32> {
        let indexed = !matches!(self, Start::RecursiveHelper);
        let chosen = |candidate: usize| {
            let candidate = i32::try_from(candidate).ok()?;
            let count = i32::try_from(candidates).ok()?;
            match self {
                Start::FromEnd => Some(count - candidate),
                Start::SaturatingIndex => (candidate == 0).then_some(0),
                Start::Element
                | Start::LiteralOfOne
                | Start::LiteralOfTwo
                | Start::TopLevelLet
                | Start::SliceThenFirst
                | Start::NestedLiteral
                | Start::RecursiveHelper
                | Start::RecursiveArrayHelper => Some(candidate),
            }
        };
        match fault {
            Fault::Inside(candidate) if candidate < candidates => chosen(candidate),
            Fault::Inside(_) => None,
            Fault::Outside if self == Start::SaturatingIndex => Some(1),
            Fault::Outside => indexed.then_some(3),
            Fault::Saturating => (self == Start::SaturatingIndex).then_some(2),
            Fault::Depth => {
                matches!(self, Start::RecursiveHelper | Start::RecursiveArrayHelper).then_some(0)
            }
        }
    }
}

const START_CONSUMERS: [Consumer; 4] = [
    Consumer::Path,
    Consumer::LocalLet,
    Consumer::InlinedCall,
    Consumer::RecursiveArgument,
];

const START_HELPERS: [&str; 3] = [
    "fn f(c): c.gain\n",
    "fn g(c, n): if n < 1 then c.gain else g(c, n - 1)\n",
    "fn pickr(c, d, p, m): if m < 1 then (if p > 0 then d else c) else pickr(c, d, p, m - 1)\n",
];

impl Start {
    fn levels(self, candidates: usize, fault: Fault) -> Vec<(usize, Levels, i32)> {
        let value = |fault| {
            self.frame_value(candidates, fault)
                .and_then(|value| usize::try_from(value).ok())
        };
        let Some(index) = value(fault) else {
            return Vec::new();
        };
        let mut values = vec![(
            index,
            Levels::Base(2),
            match fault {
                Fault::Inside(candidate) => GAINS.get(candidate).copied().unwrap_or_default(),
                Fault::Outside | Fault::Saturating | Fault::Depth => 2,
            },
        )];
        let partner = match fault {
            Fault::Inside(candidate) => value(Fault::Inside(1 - candidate)),
            Fault::Outside | Fault::Saturating | Fault::Depth => {
                [Fault::Inside(0), Fault::Inside(1)]
                    .into_iter()
                    .filter_map(value)
                    .find(|partner| *partner != index)
            }
        };
        let Some(partner) = partner else {
            return values;
        };
        for depth in ALTERNATING_DEPTHS {
            let levels = Levels::Alternating(index + partner, depth);
            let offset = match fault {
                Fault::Inside(candidate) => levels
                    .reads(index)
                    .into_iter()
                    .map(|(chosen, _)| {
                        let candidate = if chosen == index {
                            candidate
                        } else {
                            1 - candidate
                        };
                        GAINS.get(candidate).copied().unwrap_or_default()
                    })
                    .sum(),
                Fault::Outside | Fault::Saturating | Fault::Depth => 2,
            };
            values.push((index, levels, offset));
        }
        values
    }
}

fn start_program(
    (start, fault, consumer): (Start, Fault, Consumer),
    (candidates, index, levels, expected): (usize, usize, Levels, i32),
    (reversed, recursive): (bool, bool),
) -> String {
    let chosen = start.expression(candidates, fault);
    let local = consumer.local(&chosen);
    let read = consumer.read(&chosen);
    let array = if candidates == 2 {
        "[verse, other]"
    } else {
        "[verse]"
    };
    let pair = if candidates == 2 { "[c, d]" } else { "[c, c]" };
    let mut items = vec![
        "Session main\n".to_string(),
        "Clip verse | gain=2:\n    Note\n".to_string(),
        "Clip other | gain=3:\n    Note\n".to_string(),
        format!("let rs = {array}\n"),
        format!("fn pk(c, d, m): if m < 1 then {pair} else pk(c, d, m - 1)\n"),
    ];
    items.extend(START_HELPERS.iter().map(|helper| helper.to_string()));
    items.push(chosen_function(
        Chooser::ParameterArray,
        (&local, &read),
        levels,
        recursive,
    ));
    items.push(format!(
        "Clip k:\n    Note/([60][i32(h(verse, other, {array}, {index}, {})) - {expected}])\n",
        levels.depth()
    ));
    if reversed {
        items.reverse();
    }
    items.concat()
}

#[test]
fn a_path_through_a_frame_chosen_start_evaluates_the_start_in_the_frame_however_many_instances_it_names()
 {
    let mut failures = Vec::new();
    let mut programs = 0;
    for start in STARTS {
        for fault in FAULTS {
            for candidates in [1, 2] {
                for (index, levels, offset) in start.levels(candidates, fault) {
                    for consumer in START_CONSUMERS {
                        for reversed in [false, true] {
                            let program = |recursive| {
                                start_program(
                                    (start, fault, consumer),
                                    (candidates, index, levels, offset),
                                    (reversed, recursive),
                                )
                            };
                            let (text, twin) = (program(true), program(false));
                            let (drawn, expected) =
                                (codes_at(&text, levels), codes_at(&twin, levels));
                            programs += 1;
                            let codes: Vec<Code> = expected.keys().map(|(code, _)| *code).collect();
                            let twin_holds_the_fault = match fault.code() {
                                Some(code) => {
                                    !codes.is_empty() && codes.iter().all(|found| *found == code)
                                }
                                None => codes.is_empty(),
                            };
                            if !twin_holds_the_fault || drawn != expected {
                                failures.push(format!(
                                "{start:?} {fault:?} {candidates} {consumer:?} {levels:?} {reversed}\n{text}\n  found {drawn:?}\n  twin {expected:?}\n"
                            ));
                            }
                        }
                    }
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {programs} programs fail:\n{}",
        failures.len(),
        failures.iter().take(8).cloned().collect::<String>()
    );
    assert_eq!(programs, 1_384);
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum InnerCall {
    OnCycle,
    Partner,
    Settled,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Wrapping {
    Bare,
    Element,
    Choice,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum OuterCallee {
    Itself,
    Separate,
    Partner,
    Inlined,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum CallUse {
    Path,
    Length,
    TopLet,
}

impl InnerCall {
    fn text(self) -> &'static str {
        match self {
            InnerCall::OnCycle => "a(c, m - 1)",
            InnerCall::Partner => "b(c, m - 1)",
            InnerCall::Settled => "s(c, 1)",
        }
    }
}

impl Wrapping {
    fn wrap(self, inner: &str) -> String {
        match self {
            Wrapping::Bare => inner.to_string(),
            Wrapping::Element => format!("[{inner}][0]"),
            Wrapping::Choice => format!("(if m > 5 then c else {inner})"),
        }
    }
}

impl OuterCallee {
    fn call(self, argument: &str) -> String {
        match self {
            OuterCallee::Itself => format!("a({argument}, m - 1)"),
            OuterCallee::Separate => format!("idr({argument}, 1)"),
            OuterCallee::Partner => format!("b({argument}, m - 1)"),
            OuterCallee::Inlined => format!("f({argument})"),
        }
    }
}

impl CallUse {
    fn items(self, depth: u32) -> Vec<String> {
        match self {
            CallUse::Path => vec![format!(
                "Clip k:\n    Note/([60][i32(a(verse, {depth}).gain) - 2])\n"
            )],
            CallUse::Length => vec![format!(
                "Clip k:\n    Note/([60][len([a(verse, {depth})]) - 1])\n"
            )],
            CallUse::TopLet => vec![
                format!("let t = a(verse, {depth})\n"),
                "Clip k:\n    Note/([60][i32(t.gain) - 2])\n".to_string(),
            ],
        }
    }
}

fn keyed_program(
    (inner, wrapping, outer, used): (InnerCall, Wrapping, OuterCallee, CallUse),
    (depth, reversed, recursive): (u32, bool, bool),
) -> String {
    let body = if recursive {
        format!(
            "if m < 1 then c else {}",
            outer.call(&wrapping.wrap(inner.text()))
        )
    } else {
        "c".to_string()
    };
    let mut items = vec![
        "Session main\n".to_string(),
        "Clip verse | gain=2:\n    Note\n".to_string(),
        format!("fn a(c, m): {body}\n"),
        "fn b(c, m): if m < 1 then c else a(c, m - 1)\n".to_string(),
        "fn idr(x, k): if k < 1 then x else idr(x, k - 1)\n".to_string(),
        "fn s(c, m): if m < 1 then c else s(c, m - 1)\n".to_string(),
        "fn f(x): x\n".to_string(),
    ];
    items.extend(used.items(depth));
    if reversed {
        items.reverse();
    }
    items.concat()
}

#[test]
fn keying_a_recursive_calls_reference_argument_that_is_a_call_on_the_callers_cycle_draws_what_its_twin_draws()
 {
    let mut failures = Vec::new();
    let mut programs = 0;
    for inner in [InnerCall::OnCycle, InnerCall::Partner, InnerCall::Settled] {
        for wrapping in [Wrapping::Bare, Wrapping::Element, Wrapping::Choice] {
            for outer in [
                OuterCallee::Itself,
                OuterCallee::Separate,
                OuterCallee::Partner,
                OuterCallee::Inlined,
            ] {
                for used in [CallUse::Path, CallUse::Length, CallUse::TopLet] {
                    for depth in [0, 2, 7] {
                        for reversed in [false, true] {
                            let case = (inner, wrapping, outer, used);
                            let text = keyed_program(case, (depth, reversed, true));
                            let twin = keyed_program(case, (depth, reversed, false));
                            let (drawn, expected) = (found(&text), found(&twin));
                            programs += 1;
                            if !expected.is_empty() || drawn != expected {
                                failures.push(format!(
                                    "{inner:?} {wrapping:?} {outer:?} {used:?} {depth} {reversed}\n{text}\n  found {drawn:?}\n  twin {expected:?}\n"
                                ));
                            }
                        }
                    }
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {programs} programs fail:\n{}",
        failures.len(),
        failures.iter().take(8).cloned().collect::<String>()
    );
    assert_eq!(programs, 648);
}

const TRIO: [(&str, i32); 3] = [("verse", 2), ("other", 3), ("tail", 4)];

fn gain_of(instance: &str) -> i32 {
    TRIO.iter()
        .find(|(name, _)| *name == instance)
        .map(|(_, gain)| *gain)
        .unwrap_or_default()
}

fn trio_items() -> Vec<String> {
    let mut items = vec!["Session main\n".to_string()];
    items.extend(
        TRIO.iter()
            .map(|(name, gain)| format!("Clip {name} | gain={gain}:\n    Note\n")),
    );
    items
}

fn model_failure(text: &str, twin: Option<&str>, expected: &Found) -> Option<String> {
    let drawn = found(text);
    let twin_drawn = twin.map(found);
    if twin_drawn.as_ref().is_some_and(|twin| twin != expected) || drawn != *expected {
        return Some(format!(
            "{text}\n  found {drawn:?}\n  twin {twin_drawn:?}\n  expected {expected:?}\n"
        ));
    }
    None
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum PathHelper {
    Element,
    Slice,
    Literal,
    ArrayResult,
    Choice,
}

const PATH_HELPERS: [PathHelper; 5] = [
    PathHelper::Element,
    PathHelper::Slice,
    PathHelper::Literal,
    PathHelper::ArrayResult,
    PathHelper::Choice,
];

impl PathHelper {
    fn declaration(self, recursive: bool) -> String {
        let body = match self {
            PathHelper::Element => "xs[p]",
            PathHelper::Slice => "xs[p..][0]",
            PathHelper::Literal => "[xs[0], xs[1]][p]",
            PathHelper::Choice => "(if p > 0 then xs[1] else xs[0])",
            PathHelper::ArrayResult if recursive => {
                return "fn pk(xs, m): if m < 1 then xs else pk(xs, m - 1)\n".to_string();
            }
            PathHelper::ArrayResult => return "fn pk(xs, m): xs\n".to_string(),
        };
        if recursive {
            format!("fn pickr(xs, p, m): if m < 1 then {body} else pickr(xs, p, m - 1)\n")
        } else {
            format!("fn pickr(xs, p, m): {body}\n")
        }
    }

    fn call(self, array: &str, index: &str) -> String {
        match self {
            PathHelper::ArrayResult => format!("pk({array}, 2)[{index}]"),
            PathHelper::Element | PathHelper::Slice | PathHelper::Literal | PathHelper::Choice => {
                format!("pickr({array}, {index}, 2)")
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum CallStart {
    Alone,
    LetIf,
    LetMatch,
    LetArray,
    SelfRead,
    SelfReadWrapped,
    SelfReadInPartner,
}

const CALL_STARTS: [CallStart; 7] = [
    CallStart::Alone,
    CallStart::LetIf,
    CallStart::LetMatch,
    CallStart::LetArray,
    CallStart::SelfRead,
    CallStart::SelfReadWrapped,
    CallStart::SelfReadInPartner,
];

const SITE_ARRAYS: [[&str; 2]; 3] = [["verse", "other"], ["tail", "verse"], ["other", "tail"]];

fn site_sets() -> Vec<Vec<[&'static str; 2]>> {
    let mut sets = vec![vec![SITE_ARRAYS[0]]];
    for first in SITE_ARRAYS {
        for second in SITE_ARRAYS {
            if first != second {
                sets.push(vec![first, second]);
            }
        }
    }
    sets
}

fn beside(array: [&str; 2]) -> &'static str {
    TRIO.iter()
        .map(|(name, _)| *name)
        .find(|name| !array.contains(name))
        .unwrap_or("verse")
}

impl CallStart {
    fn self_reading(self) -> bool {
        matches!(
            self,
            CallStart::SelfRead | CallStart::SelfReadWrapped | CallStart::SelfReadInPartner
        )
    }

    fn expression(self, helper: PathHelper) -> String {
        match self {
            CallStart::Alone => helper.call("xs", "i"),
            CallStart::LetIf => "(if i > 0 then rp else c)".to_string(),
            CallStart::LetMatch => "m".to_string(),
            CallStart::LetArray => "[rp, c][i]".to_string(),
            CallStart::SelfRead | CallStart::SelfReadInPartner => "sf(xs, i, n - 1)".to_string(),
            CallStart::SelfReadWrapped => "w(xs, i, n - 1)".to_string(),
        }
    }

    fn locals(self) -> &'static str {
        match self {
            CallStart::LetMatch => "    let m = match i:\n        0: rp\n        else: c\n",
            CallStart::Alone
            | CallStart::LetIf
            | CallStart::LetArray
            | CallStart::SelfRead
            | CallStart::SelfReadWrapped
            | CallStart::SelfReadInPartner => "",
        }
    }

    fn chosen(self, array: [&'static str; 2], index: usize) -> &'static str {
        let element = |position: usize| array.get(position).copied().unwrap_or("verse");
        match self {
            CallStart::Alone => element(index),
            CallStart::LetIf => {
                if index > 0 {
                    "other"
                } else {
                    beside(array)
                }
            }
            CallStart::LetMatch | CallStart::LetArray => {
                if index == 0 {
                    "other"
                } else {
                    beside(array)
                }
            }
            CallStart::SelfRead | CallStart::SelfReadWrapped | CallStart::SelfReadInPartner => {
                let mut chosen = element(index);
                for _ in 0..3 {
                    chosen = if gain_of(chosen) > 2 {
                        element(1)
                    } else {
                        element(0)
                    };
                }
                chosen
            }
        }
    }
}

fn call_start_program(
    (helper, start, consumer): (PathHelper, CallStart, Consumer),
    (sites, index, wrong): (&[[&'static str; 2]], usize, bool),
    (levels, reversed, recursive): (Levels, bool, bool),
) -> (String, Found) {
    let chosen = start.expression(helper);
    let local = consumer.local(&chosen);
    let read = consumer.read(&chosen);
    let mut items = trio_items();
    items.push(helper.declaration(recursive));
    items.push(format!("let rp = {}\n", helper.call("[verse, other]", "1")));
    items.push("fn f(c): c.gain\n".to_string());
    items.push("fn g(c, n): if n < 1 then c.gain else g(c, n - 1)\n".to_string());
    if start.self_reading() {
        let choice = format!("if {read} > 2 then xs[1] else xs[0]");
        if start == CallStart::SelfReadInPartner {
            items.push("fn sf(xs, i, n): if n < 1 then xs[i] else sp(xs, i, n)\n".to_string());
            items.push(format!("fn sp(xs, i, n):\n{local}    {choice}\n"));
        } else {
            let value = if recursive {
                format!("if n < 1 then xs[i] else ({choice})")
            } else {
                "xs[i]".to_string()
            };
            items.push(format!("fn sf(xs, i, n):\n{local}    {value}\n"));
        }
        items.push("fn w(xs, i, n): sf(xs, i, n)\n".to_string());
    } else {
        items.push(levels.functions(
            ("c, xs, i, n", &format!("{}{local}", start.locals()), &read),
            |next| format!("c, xs, {next}, n - 1"),
            recursive,
        ));
    }
    let mut expected = Found::new();
    for (position, array) in sites.iter().enumerate() {
        let gain = levels
            .reads(index)
            .into_iter()
            .map(|(chosen, _)| gain_of(start.chosen(*array, chosen)))
            .sum::<i32>()
            + i32::from(wrong);
        let elements = array.join(", ");
        let depth = levels.depth();
        let call = if start.self_reading() {
            format!("sf([{elements}], {index}, {depth}).gain")
        } else {
            format!("h({}, [{elements}], {index}, {depth})", beside(*array))
        };
        let offset = format!("i32({call}) - {gain}");
        if wrong {
            *expected
                .entry((Code::IndexOutOfRange, offset.clone()))
                .or_default() += 1;
        }
        items.push(format!("Clip k{position}:\n    Note/([60][{offset}])\n"));
    }
    if reversed {
        items.reverse();
    }
    (items.concat(), expected)
}

#[test]
fn a_path_start_chosen_by_a_recursive_call_names_the_instances_that_calls_own_evaluation_returns() {
    let sets = site_sets();
    let (failures, programs) = in_parallel(&CALL_STARTS, |start| {
        let mut failures = Vec::new();
        let mut programs = 0;
        let (helpers, frames): (&[PathHelper], Vec<(usize, Levels)>) = if start.self_reading() {
            (
                &[PathHelper::Element],
                vec![(0, Levels::Base(3)), (1, Levels::Base(3))],
            )
        } else {
            (&PATH_HELPERS, Levels::of(&[0, 1], &[3]))
        };
        for helper in helpers.iter().copied() {
            for consumer in START_CONSUMERS {
                for sites in &sets {
                    for (index, levels) in frames.iter().copied() {
                        for wrong in [false, true] {
                            for reversed in [false, true] {
                                let program = |recursive| {
                                    call_start_program(
                                        (helper, start, consumer),
                                        (sites, index, wrong),
                                        (levels, reversed, recursive),
                                    )
                                };
                                let (text, expected) = program(true);
                                let twin = (!start.self_reading()).then(|| program(false).0);
                                programs += 1;
                                if let Some(failure) =
                                    model_failure(&text, twin.as_deref(), &expected)
                                {
                                    failures.push(format!(
                                        "{start:?} {helper:?} {consumer:?} {sites:?} {index} {levels:?} {wrong} {reversed}\n{failure}"
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }
        (failures, programs)
    });
    assert!(
        failures.is_empty(),
        "{} of {programs} programs fail:\n{}",
        failures.len(),
        failures.iter().take(8).cloned().collect::<String>()
    );
    assert_eq!(programs, 18_592);
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum OpenReader {
    Partner,
    Separate,
}

fn open_reader_program(
    reader: OpenReader,
    (verse, other): (i32, i32),
    (depth, offset, reversed): (i32, i32, bool),
) -> String {
    let (functions, read) = match reader {
        OpenReader::Partner => (
            [
                "fn f(xs, n): if n < 1 then xs[0] else g(xs, n)\n".to_string(),
                "fn g(xs, n): if f(xs, n - 1).gain > 2 then xs[1] else xs[0]\n".to_string(),
            ],
            format!("f([verse, other], {depth}).gain"),
        ),
        OpenReader::Separate => (
            [
                "fn f(xs, n): if n < 1 then xs[0] else (if f(xs, n - 1).gain > 2 then xs[1] else xs[0])\n"
                    .to_string(),
                format!("fn h(xs, n): if n < 1 then f(xs, {depth}).gain else h(xs, n - 1)\n"),
            ],
            "h([verse, other], 2)".to_string(),
        ),
    };
    let mut items = vec![
        "Session main\n".to_string(),
        format!("Clip verse | gain={verse}:\n    Note\n"),
        format!("Clip other | gain={other}:\n    Note\n"),
    ];
    items.extend(functions);
    items.push(format!(
        "Clip k:\n    Note/([60][i32({read}) - {offset}])\n"
    ));
    if reversed {
        items.reverse();
    }
    items.concat()
}

#[test]
fn a_probe_reading_another_familys_open_result_through_a_path_reads_the_chosen_instance() {
    let mut failures = Vec::new();
    let mut programs = 0;
    for reader in [OpenReader::Partner, OpenReader::Separate] {
        for gains in [(2, 2), (2, 3), (3, 2), (3, 3)] {
            for depth in [0, 1, 3] {
                let gain = (0..depth).fold(
                    gains.0,
                    |chosen, _| {
                        if chosen > 2 { gains.1 } else { gains.0 }
                    },
                );
                for wrong in [false, true] {
                    for reversed in [false, true] {
                        let offset = gain + i32::from(wrong);
                        let text = open_reader_program(reader, gains, (depth, offset, reversed));
                        let drawn: Vec<Code> =
                            found(&text).into_keys().map(|(code, _)| code).collect();
                        let expected = if wrong {
                            vec![Code::IndexOutOfRange]
                        } else {
                            Vec::new()
                        };
                        programs += 1;
                        if drawn != expected {
                            failures.push(format!(
                                "{reader:?} {gains:?} {depth} {wrong} {reversed}\n{text}\n  found {drawn:?}\n  expected {expected:?}\n"
                            ));
                        }
                    }
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {programs} programs fail:\n{}",
        failures.len(),
        failures.iter().take(8).cloned().collect::<String>()
    );
    assert_eq!(programs, 96);
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum OpenChooser {
    IfTrue,
    IfFalse,
    MatchOnLiteral,
    NestedIf,
    IfOnParameter,
    ElseArmIf,
    MatchOnParameter,
}

const OPEN_CHOOSERS: [OpenChooser; 7] = [
    OpenChooser::IfTrue,
    OpenChooser::IfFalse,
    OpenChooser::MatchOnLiteral,
    OpenChooser::NestedIf,
    OpenChooser::IfOnParameter,
    OpenChooser::ElseArmIf,
    OpenChooser::MatchOnParameter,
];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum OpenArm {
    Instances,
    Parameter,
    ParameterChosen,
    TopLet,
}

const OPEN_ARMS: [OpenArm; 4] = [
    OpenArm::Instances,
    OpenArm::Parameter,
    OpenArm::ParameterChosen,
    OpenArm::TopLet,
];

impl OpenArm {
    fn text(self) -> &'static str {
        match self {
            OpenArm::Instances => "pickr(verse, other, 1, 2)",
            OpenArm::Parameter => "pickr(c, other, 1, 2)",
            OpenArm::ParameterChosen => "pickr(c, other, i, 2)",
            OpenArm::TopLet => "rp",
        }
    }

    fn chosen(self, index: usize) -> &'static str {
        match self {
            OpenArm::ParameterChosen if index == 0 => "tail",
            OpenArm::Instances
            | OpenArm::Parameter
            | OpenArm::ParameterChosen
            | OpenArm::TopLet => "other",
        }
    }
}

impl OpenChooser {
    fn expression(self, open: &str, known: &str) -> String {
        match self {
            OpenChooser::IfTrue => format!("(if true then {open} else {known})"),
            OpenChooser::IfFalse => format!("(if false then {open} else {known})"),
            OpenChooser::MatchOnLiteral | OpenChooser::MatchOnParameter => "m".to_string(),
            OpenChooser::NestedIf => {
                format!("(if true then (if false then {known} else {open}) else {known})")
            }
            OpenChooser::IfOnParameter => format!("(if i > 0 then {open} else {known})"),
            OpenChooser::ElseArmIf => {
                format!("(if i < 1 then {known} else (if true then {open} else {known}))")
            }
        }
    }

    fn locals(self, open: &str, known: &str) -> String {
        let scrutinee = match self {
            OpenChooser::MatchOnLiteral => "1",
            OpenChooser::MatchOnParameter => "i",
            OpenChooser::IfTrue
            | OpenChooser::IfFalse
            | OpenChooser::NestedIf
            | OpenChooser::IfOnParameter
            | OpenChooser::ElseArmIf => return String::new(),
        };
        format!("    let m = match {scrutinee}:\n        0: {known}\n        else: {open}\n")
    }

    fn takes_open(self, index: usize) -> bool {
        match self {
            OpenChooser::IfTrue | OpenChooser::MatchOnLiteral | OpenChooser::NestedIf => true,
            OpenChooser::IfFalse => false,
            OpenChooser::IfOnParameter | OpenChooser::ElseArmIf | OpenChooser::MatchOnParameter => {
                index > 0
            }
        }
    }
}

fn open_choice_program(
    (chooser, open, known, consumer): (OpenChooser, OpenArm, &str, Consumer),
    (index, levels): (usize, Levels),
    (reversed, recursive): (bool, bool),
) -> String {
    let chosen = chooser.expression(open.text(), known);
    let local = consumer.local(&chosen);
    let read = consumer.read(&chosen);
    let helper = if recursive {
        "fn pickr(c, d, p, m): if m < 1 then (if p > 0 then d else c) else pickr(c, d, p, m - 1)\n"
    } else {
        "fn pickr(c, d, p, m): if p > 0 then d else c\n"
    };
    let instance = |index| {
        if chooser.takes_open(index) {
            open.chosen(index)
        } else if known == "c" {
            "tail"
        } else {
            known
        }
    };
    let offset: i32 = levels
        .reads(index)
        .into_iter()
        .map(|(chosen, _)| gain_of(instance(chosen)))
        .sum();
    let mut items = trio_items();
    items.extend([
        helper.to_string(),
        "let rp = pickr(verse, other, 1, 2)\n".to_string(),
        "fn f(c): c.gain\n".to_string(),
        "fn g(c, n): if n < 1 then c.gain else g(c, n - 1)\n".to_string(),
        levels.functions(
            (
                "c, i, n",
                &format!("{}{local}", chooser.locals(open.text(), known)),
                &read,
            ),
            |next| format!("c, {next}, n - 1"),
            recursive,
        ),
        format!(
            "Clip k:\n    Note/([60][i32(h(tail, {index}, {})) - {offset}])\n",
            levels.depth()
        ),
    ]);
    if reversed {
        items.reverse();
    }
    items.concat()
}

#[test]
fn a_family_probes_path_through_an_open_recursive_call_settles_no_family_from_the_starts_kind() {
    let mut failures = Vec::new();
    let mut programs = 0;
    for chooser in OPEN_CHOOSERS {
        for open in OPEN_ARMS {
            for known in ["verse", "c"] {
                for consumer in START_CONSUMERS {
                    for frame in Levels::of(&[0, 1], &[3]) {
                        for reversed in [false, true] {
                            let case = (chooser, open, known, consumer);
                            let text = open_choice_program(case, frame, (reversed, true));
                            let twin = open_choice_program(case, frame, (reversed, false));
                            programs += 1;
                            if let Some(failure) = model_failure(&text, Some(&twin), &Found::new())
                            {
                                failures.push(format!(
                                    "{chooser:?} {open:?} {known} {consumer:?} {frame:?} {reversed}\n{failure}"
                                ));
                            }
                        }
                    }
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {programs} programs fail:\n{}",
        failures.len(),
        failures.iter().take(8).cloned().collect::<String>()
    );
    assert_eq!(programs, 3_584);
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum HeldReference {
    RecursiveCall,
    RecursiveElement,
    ArrayOfCall,
    InlinedCall,
    Element,
    Instance,
}

const HELD_REFERENCES: [HeldReference; 6] = [
    HeldReference::RecursiveCall,
    HeldReference::RecursiveElement,
    HeldReference::ArrayOfCall,
    HeldReference::InlinedCall,
    HeldReference::Element,
    HeldReference::Instance,
];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum HeldRead {
    Attribute,
    Child,
    InlinedCall,
    LocalLet,
    RecursiveArgument,
}

const HELD_READS: [HeldRead; 5] = [
    HeldRead::Attribute,
    HeldRead::Child,
    HeldRead::InlinedCall,
    HeldRead::LocalLet,
    HeldRead::RecursiveArgument,
];

impl HeldReference {
    fn value(self) -> &'static str {
        match self {
            HeldReference::RecursiveCall => "pickr(verse, other, 1, 2)",
            HeldReference::RecursiveElement => "pk([verse, other], 1, 2)",
            HeldReference::ArrayOfCall => "[pickr(verse, other, 1, 2), verse]",
            HeldReference::InlinedCall => "sel(verse, other, 1)",
            HeldReference::Element => "[verse, other][1]",
            HeldReference::Instance => "other",
        }
    }

    fn reference(self) -> &'static str {
        match self {
            HeldReference::ArrayOfCall => "rp[i]",
            HeldReference::RecursiveCall
            | HeldReference::RecursiveElement
            | HeldReference::InlinedCall
            | HeldReference::Element
            | HeldReference::Instance => "rp",
        }
    }

    fn top_read(self) -> &'static str {
        match self {
            HeldReference::ArrayOfCall => "rp[0].gain",
            HeldReference::RecursiveCall
            | HeldReference::RecursiveElement
            | HeldReference::InlinedCall
            | HeldReference::Element
            | HeldReference::Instance => "rp.gain",
        }
    }

    fn chosen_gain(self, index: usize) -> i32 {
        match self {
            HeldReference::ArrayOfCall if index > 0 => 2,
            HeldReference::RecursiveCall
            | HeldReference::RecursiveElement
            | HeldReference::ArrayOfCall
            | HeldReference::InlinedCall
            | HeldReference::Element
            | HeldReference::Instance => 3,
        }
    }
}

impl HeldRead {
    fn local(self, reference: &str) -> String {
        match self {
            HeldRead::LocalLet => format!("    let e = {reference}\n"),
            HeldRead::Attribute
            | HeldRead::Child
            | HeldRead::InlinedCall
            | HeldRead::RecursiveArgument => String::new(),
        }
    }

    fn read(self, reference: &str) -> String {
        match self {
            HeldRead::Attribute => format!("{reference}.gain"),
            HeldRead::Child => format!("{reference}.hook.velocity"),
            HeldRead::InlinedCall => format!("f({reference})"),
            HeldRead::LocalLet => "e.gain".to_string(),
            HeldRead::RecursiveArgument => format!("g({reference}, 2)"),
        }
    }
}

fn held_program(
    (held, consumer): (HeldReference, HeldRead),
    ((index, levels), top_level, reversed, recursive): ((usize, Levels), bool, bool, bool),
) -> String {
    let reference = held.reference();
    let read = consumer.read(reference);
    let offset: i32 = levels
        .reads(index)
        .into_iter()
        .map(|(chosen, _)| held.chosen_gain(chosen))
        .sum();
    let mut items = vec![
        "Session main\n".to_string(),
        "Clip verse | gain=2:\n    Note as hook | velocity=2\n".to_string(),
        "Clip other | gain=3:\n    Note as hook | velocity=3\n".to_string(),
        "fn pickr(c, d, p, m): if m < 1 then (if p > 0 then d else c) else pickr(c, d, p, m - 1)\n"
            .to_string(),
        "fn pk(xs, p, m): if m < 1 then xs[p] else pk(xs, p, m - 1)\n".to_string(),
        "fn sel(c, d, p): if p > 0 then d else c\n".to_string(),
        "fn g(c, n): if n < 1 then c.gain else g(c, n - 1)\n".to_string(),
        "fn f(c): c.gain\n".to_string(),
        format!("let rp = {}\n", held.value()),
        levels.functions(
            ("c, i, n", &consumer.local(reference), &read),
            |next| format!("c, {next}, n - 1"),
            recursive,
        ),
        format!(
            "Clip k:\n    Note/([60][i32(h(verse, {index}, {})) - {offset}])\n",
            levels.depth()
        ),
    ];
    if top_level {
        items.push(format!("Clip t | gain={}:\n    Note\n", held.top_read()));
    }
    if reversed {
        items.reverse();
    }
    items.concat()
}

#[test]
fn a_top_level_let_holding_a_recursive_calls_reference_reads_through_a_path_in_a_recursive_body_as_at_top_level()
 {
    let mut failures = Vec::new();
    let mut programs = 0;
    for held in HELD_REFERENCES {
        for consumer in HELD_READS {
            for frame in Levels::of(&[0, 1], &[3]) {
                for top_level in [false, true] {
                    for reversed in [false, true] {
                        let program = |recursive| {
                            held_program((held, consumer), (frame, top_level, reversed, recursive))
                        };
                        let (text, twin) = (program(true), program(false));
                        programs += 1;
                        if let Some(failure) = model_failure(&text, Some(&twin), &Found::new()) {
                            failures.push(format!(
                                "{held:?} {consumer:?} {frame:?} {top_level} {reversed}\n{failure}"
                            ));
                        }
                    }
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {programs} programs fail:\n{}",
        failures.len(),
        failures.iter().take(8).cloned().collect::<String>()
    );
    assert_eq!(programs, 960);
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum CandidateFault {
    LacksAttribute,
    ReadsCell,
    ClosesCycle,
    EveryCandidateClosesCycle,
}

const CANDIDATE_FAULTS: [CandidateFault; 4] = [
    CandidateFault::LacksAttribute,
    CandidateFault::ReadsCell,
    CandidateFault::ClosesCycle,
    CandidateFault::EveryCandidateClosesCycle,
];

const FAULT_CHOOSERS: [&str; 4] = [
    "xs[i]",
    "(if i > 0 then d else c)",
    "pickr(c, d, i, 2)",
    "[c, d][i]",
];

fn candidate_fault_program(
    (fault, chooser, consumer): (CandidateFault, &str, Consumer),
    (index, levels, reversed, recursive): (usize, Levels, bool, bool),
    second: Option<bool>,
) -> String {
    let local = consumer.local(chooser);
    let read = consumer.read(chooser);
    let depth = levels.depth();
    let (other, faulty) = match fault {
        CandidateFault::LacksAttribute => ("Clip other | gain=3:\n    Note\n", "verse.hook"),
        CandidateFault::ReadsCell => ("Clip other | gain=gv:\n    Note\n", "other"),
        CandidateFault::ClosesCycle | CandidateFault::EveryCandidateClosesCycle => {
            ("Clip other | gain=k.gain:\n    Note\n", "other")
        }
    };
    let verse = if fault == CandidateFault::EveryCandidateClosesCycle {
        "Clip verse | gain=k.gain:\n    Note as hook\n"
    } else {
        "Clip verse | gain=2:\n    Note as hook\n"
    };
    let mut items = vec![
        "Session main\n".to_string(),
        "input gv = 1.0\n".to_string(),
        verse.to_string(),
        other.to_string(),
        "fn f(c): c.gain\n".to_string(),
        "fn g(c, n): if n < 1 then c.gain else g(c, n - 1)\n".to_string(),
        "fn pickr(c, d, p, m): if m < 1 then (if p > 0 then d else c) else pickr(c, d, p, m - 1)\n"
            .to_string(),
        levels.functions(
            ("c, d, xs, i, n", &local, &read),
            |next| format!("c, d, xs, {next}, n - 1"),
            recursive,
        ),
        format!(
            "Clip k | gain=h(verse, {faulty}, [verse, {faulty}], {index}, {depth}):\n    Note\n"
        ),
    ];
    if let Some(wrong) = second {
        let offset = 2 + i32::from(wrong);
        items.push("Clip tail | gain=4:\n    Note\n".to_string());
        items.push(format!(
            "Clip z:\n    Note/([60][i32(h(other, tail, [other, tail], 0, {depth})) - {offset}])\n"
        ));
    }
    if reversed {
        items.reverse();
    }
    items.concat()
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum StepPath {
    Gain,
    Child,
    Wire,
}

const STEP_PATHS: [StepPath; 3] = [StepPath::Gain, StepPath::Child, StepPath::Wire];

const HOLDERS: [(&str, [i32; 3]); 3] = [
    ("verse", [2, 12, 22]),
    ("other", [3, 13, 23]),
    ("third", [5, 15, 25]),
];

const LACKING_CLIPS: [&str; 3] = ["bare0", "bare1", "bare2"];

impl StepPath {
    fn first(self) -> &'static str {
        match self {
            StepPath::Gain => "gain",
            StepPath::Child => "hook",
            StepPath::Wire => "back",
        }
    }

    fn rest(self) -> &'static str {
        match self {
            StepPath::Gain => "",
            StepPath::Child => ".velocity",
            StepPath::Wire => ".gain",
        }
    }

    fn slot(self) -> usize {
        match self {
            StepPath::Gain => 0,
            StepPath::Child => 1,
            StepPath::Wire => 2,
        }
    }

    fn candidate(self, position: usize, holds: bool) -> String {
        let (holder, _) = HOLDERS[position];
        match (holds, self) {
            (true, _) => holder.to_string(),
            (false, StepPath::Gain) => format!("{holder}.tone{position}"),
            (false, StepPath::Child | StepPath::Wire) => LACKING_CLIPS[position].to_string(),
        }
    }

    fn lacking(self, position: usize) -> String {
        let (kind, name) = match self {
            StepPath::Gain => ("Note", format!("tone{position}")),
            StepPath::Child | StepPath::Wire => ("Clip", LACKING_CLIPS[position].to_string()),
        };
        format!(
            "the `{kind}` named `{name}` has no child, wire or attribute `{}`",
            self.first()
        )
    }

    fn value(self, position: usize) -> i32 {
        HOLDERS[position].1[self.slot()]
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum StepChooser {
    Element,
    Literal,
    IfOnParameter,
    RecursiveHelper,
    IfOnDepth,
}

const STEP_CHOOSERS: [StepChooser; 5] = [
    StepChooser::Element,
    StepChooser::Literal,
    StepChooser::IfOnParameter,
    StepChooser::RecursiveHelper,
    StepChooser::IfOnDepth,
];

impl StepChooser {
    fn expression(self) -> &'static str {
        match self {
            StepChooser::Element => "xs[i]",
            StepChooser::Literal => "[c, d][i]",
            StepChooser::IfOnParameter => "(if i > 0 then d else c)",
            StepChooser::RecursiveHelper => "pickr(c, d, i, 2)",
            StepChooser::IfOnDepth => "(if n > 50 then c else d)",
        }
    }

    fn chosen(self, index: usize, depth: i32) -> usize {
        match self {
            StepChooser::Element | StepChooser::Literal => index,
            StepChooser::IfOnParameter | StepChooser::RecursiveHelper => usize::from(index > 0),
            StepChooser::IfOnDepth => usize::from(depth <= 50),
        }
    }

    fn counts(self) -> &'static [usize] {
        match self {
            StepChooser::Element => &[2, 3],
            StepChooser::Literal
            | StepChooser::IfOnParameter
            | StepChooser::RecursiveHelper
            | StepChooser::IfOnDepth => &[2],
        }
    }

    fn depths(self) -> &'static [i32] {
        match self {
            StepChooser::IfOnDepth => &[1, 3, 30, 60],
            StepChooser::Element
            | StepChooser::Literal
            | StepChooser::IfOnParameter
            | StepChooser::RecursiveHelper => &[1, 3, 30],
        }
    }

    fn reaches_outside(self) -> bool {
        matches!(self, StepChooser::Element | StepChooser::Literal)
    }

    fn length(self, count: usize) -> usize {
        match self {
            StepChooser::Element => count,
            StepChooser::Literal
            | StepChooser::IfOnParameter
            | StepChooser::RecursiveHelper
            | StepChooser::IfOnDepth => 2,
        }
    }

    fn indices(self, count: usize) -> Vec<usize> {
        match self {
            StepChooser::IfOnDepth => vec![0],
            StepChooser::Element
            | StepChooser::Literal
            | StepChooser::IfOnParameter
            | StepChooser::RecursiveHelper => (0..count).collect(),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum StepConsumer {
    Path,
    LetStart,
    LetStep,
    InlinedStart,
    InlinedStep,
    RecursiveArgument,
    ArrayOfStep,
    Match,
}

const STEP_CONSUMERS: [StepConsumer; 8] = [
    StepConsumer::Path,
    StepConsumer::LetStart,
    StepConsumer::LetStep,
    StepConsumer::InlinedStart,
    StepConsumer::InlinedStep,
    StepConsumer::RecursiveArgument,
    StepConsumer::ArrayOfStep,
    StepConsumer::Match,
];

impl StepConsumer {
    fn local(self, path: StepPath, start: &str) -> String {
        match self {
            StepConsumer::LetStart => format!("    let e = {start}\n"),
            StepConsumer::LetStep => format!("    let e = ({start}).{}\n", path.first()),
            StepConsumer::Match => {
                format!("    let e = match 1:\n        0: c\n        else: {start}\n")
            }
            StepConsumer::Path
            | StepConsumer::InlinedStart
            | StepConsumer::InlinedStep
            | StepConsumer::RecursiveArgument
            | StepConsumer::ArrayOfStep => String::new(),
        }
    }

    fn read(self, path: StepPath, start: &str) -> String {
        let (first, rest) = (path.first(), path.rest());
        match self {
            StepConsumer::Path => format!("({start}).{first}{rest}"),
            StepConsumer::LetStart | StepConsumer::Match => format!("e.{first}{rest}"),
            StepConsumer::LetStep => format!("e{rest}"),
            StepConsumer::InlinedStart => format!("fs({start})"),
            StepConsumer::InlinedStep => format!("ft(({start}).{first})"),
            StepConsumer::RecursiveArgument => format!("gs({start}, 2)"),
            StepConsumer::ArrayOfStep => format!("[({start}).{first}][0]{rest}"),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum StepArm {
    Taken,
    IfFalse,
    IfDeep,
}

const STEP_ARMS: [StepArm; 3] = [StepArm::Taken, StepArm::IfFalse, StepArm::IfDeep];

impl StepArm {
    fn wrap(self, read: &str) -> String {
        match self {
            StepArm::Taken => format!("i32({read})"),
            StepArm::IfFalse => format!("(if false then i32({read}) else 0)"),
            StepArm::IfDeep => format!("(if n > 100 then i32({read}) else 0)"),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum FrameValue {
    Fixed(usize),
    Alternating(usize, usize),
    Outside,
}

const OUTSIDE: usize = 5;

impl FrameValue {
    fn at(self, level: i32) -> usize {
        match self {
            FrameValue::Fixed(index) => index,
            FrameValue::Outside => OUTSIDE,
            FrameValue::Alternating(first, _) if level % 2 == 0 => first,
            FrameValue::Alternating(_, second) => second,
        }
    }

    fn index(self) -> usize {
        self.at(0)
    }

    fn next(self) -> String {
        match self {
            FrameValue::Fixed(_) | FrameValue::Outside => "i".to_string(),
            FrameValue::Alternating(first, second) => format!("{} - i", first + second),
        }
    }

    fn of(count: usize, indices: &[usize], outside: bool) -> Vec<FrameValue> {
        let mut values: Vec<FrameValue> = indices
            .iter()
            .map(|index| FrameValue::Fixed(*index))
            .collect();
        values.extend(
            indices
                .iter()
                .map(|index| FrameValue::Alternating(*index, (index + 1) % count.max(2))),
        );
        if outside {
            values.push(FrameValue::Outside);
        }
        values
    }
}

struct StepCase {
    path: StepPath,
    chooser: StepChooser,
    holds: Vec<bool>,
    consumer: StepConsumer,
    arm: StepArm,
    frame: FrameValue,
    depth: i32,
    reversed: bool,
}

impl StepCase {
    fn levels(&self) -> Vec<(i32, bool)> {
        match self.frame {
            FrameValue::Fixed(_) | FrameValue::Outside => (0..=self.depth)
                .map(|level| (level, level == self.depth))
                .collect(),
            FrameValue::Alternating(..) => (0..=self.depth)
                .map(|level| (level, level < self.depth))
                .collect(),
        }
    }

    fn unrolled(&self) -> bool {
        matches!(self.frame, FrameValue::Alternating(..)) || self.depth > 50
    }

    fn expected(&self) -> (BTreeSet<(Code, String, String)>, i32) {
        let mut drawn = BTreeSet::new();
        let mut value = 0;
        let mut faulty = false;
        for (level, reads) in self.levels() {
            let chosen = self
                .chooser
                .chosen(self.frame.at(level), self.depth - level);
            let taken = reads && self.arm == StepArm::Taken;
            let Some(holds) = self.holds.get(chosen) else {
                if taken {
                    faulty = true;
                    drawn.insert((
                        Code::IndexOutOfRange,
                        "i".to_string(),
                        format!(
                            "the index {OUTSIDE} is outside the array of length {}",
                            self.chooser.length(self.holds.len())
                        ),
                    ));
                }
                continue;
            };
            let lacks = !holds;
            let checked = taken || self.consumer != StepConsumer::RecursiveArgument;
            if lacks && checked {
                drawn.insert((
                    Code::UnknownAttribute,
                    self.path.first().to_string(),
                    self.path.lacking(chosen),
                ));
            }
            if taken {
                faulty |= lacks;
                value += self.path.value(chosen);
            }
        }
        (drawn, if faulty { 0 } else { value })
    }

    fn body(&self, callee: Option<&str>) -> String {
        let start = self.chooser.expression();
        let arm = self.arm.wrap(&self.consumer.read(self.path, start));
        let call = |next: &str| match callee {
            Some(callee) => format!("{callee}(c, d, xs, {next}, n - 1)"),
            None => "0".to_string(),
        };
        let value = match self.frame {
            FrameValue::Fixed(_) | FrameValue::Outside => {
                format!("if n < 1 then {arm} else {}", call("i"))
            }
            FrameValue::Alternating(..) => {
                format!("if n < 1 then 0 else {arm} + {}", call(&self.frame.next()))
            }
        };
        format!("{}    {value}\n", self.consumer.local(self.path, start))
    }

    fn functions(&self, recursive: bool) -> Vec<String> {
        if recursive {
            return vec![format!("fn h(c, d, xs, i, n):\n{}", self.body(Some("h")))];
        }
        if !self.unrolled() {
            let start = self.chooser.expression();
            let arm = self.arm.wrap(&self.consumer.read(self.path, start));
            return vec![format!(
                "fn h(c, d, xs, i, n):\n{}    {arm}\n",
                self.consumer.local(self.path, start)
            )];
        }
        let depth = usize::try_from(self.depth).unwrap_or_default();
        (0..=depth)
            .map(|level| {
                let next = (level < depth).then(|| format!("h{}", level + 1));
                format!(
                    "fn h{level}(c, d, xs, i, n):\n{}",
                    self.body(next.as_deref())
                )
            })
            .collect()
    }

    fn program(&self, recursive: bool) -> String {
        let (first, rest) = (self.path.first(), self.path.rest());
        let candidates: Vec<String> = self
            .holds
            .iter()
            .enumerate()
            .map(|(position, holds)| self.path.candidate(position, *holds))
            .collect();
        let (_, offset) = self.expected();
        let callee = if recursive || !self.unrolled() {
            "h"
        } else {
            "h0"
        };
        let mut items = vec!["Session main\n".to_string()];
        for (position, (name, [gain, velocity, wire])) in HOLDERS.iter().enumerate() {
            items.push(format!(
                "Clip {name} | gain={gain}:\n    Note as hook | velocity={velocity}\n    Note as tone{position}\n    Plays as back | gain={wire}: tail\n"
            ));
        }
        for (position, name) in LACKING_CLIPS.iter().enumerate() {
            items.push(format!(
                "Clip {name} | gain={}:\n    Note\n",
                4 + 2 * position
            ));
        }
        items.extend([
            "Clip tail:\n    Note\n".to_string(),
            "fn pickr(c, d, p, m): if m < 1 then (if p > 0 then d else c) else pickr(c, d, p, m - 1)\n".to_string(),
            format!("fn fs(c): c.{first}{rest}\n"),
            format!("fn ft(c): c{rest}\n"),
            format!("fn gs(c, n): if n < 1 then c.{first}{rest} else gs(c, n - 1)\n"),
        ]);
        items.extend(self.functions(recursive));
        items.push(format!(
            "Clip k:\n    Note/([60][{callee}({}, {}, [{}], {}, {}) - {offset}])\n",
            candidates[0],
            candidates[1],
            candidates.join(", "),
            self.frame.index(),
            self.depth
        ));
        if self.reversed {
            items.reverse();
        }
        items.concat()
    }
}

fn found_summaries(text: &str) -> BTreeSet<(Code, String, String)> {
    support::checked(text)
        .into_iter()
        .map(|diagnostic| {
            let range = diagnostic.span.range;
            let at = text
                .get(range.start as usize..range.end as usize)
                .unwrap_or("?")
                .to_string();
            (diagnostic.code, at, diagnostic.summary)
        })
        .collect()
}

fn lacking_step_rows(path: StepPath, chooser: StepChooser) -> (Vec<String>, usize) {
    let mut failures = Vec::new();
    let mut programs = 0;
    for count in chooser.counts().iter().copied() {
        for mask in 0..(1u32 << count) {
            let holds: Vec<bool> = (0..count)
                .map(|position| mask & (1 << position) != 0)
                .collect();
            for consumer in STEP_CONSUMERS {
                for arm in STEP_ARMS {
                    for frame in
                        FrameValue::of(count, &chooser.indices(count), chooser.reaches_outside())
                    {
                        let twin_ranges = frame == FrameValue::Outside
                            && arm != StepArm::Taken
                            && matches!(
                                consumer,
                                StepConsumer::LetStep | StepConsumer::RecursiveArgument
                            );
                        for depth in chooser.depths().iter().copied() {
                            for reversed in [false, true] {
                                let case = StepCase {
                                    path,
                                    chooser,
                                    holds: holds.clone(),
                                    consumer,
                                    arm,
                                    frame,
                                    depth,
                                    reversed,
                                };
                                let (expected, _) = case.expected();
                                let text = case.program(true);
                                let twin = case.program(false);
                                let drawn = found_summaries(&text);
                                let twin_drawn = found_summaries(&twin);
                                programs += 1;
                                if drawn != expected || !twin_ranges && twin_drawn != expected {
                                    failures.push(format!(
                                        "{path:?} {chooser:?} {holds:?} {consumer:?} {arm:?} {frame:?} {depth} {reversed}\n{text}\n  found {drawn:?}\n  twin {twin_drawn:?}\n  expected {expected:?}\n"
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    (failures, programs)
}

fn lacking_step_failures() -> (Vec<String>, usize) {
    let pairs: Vec<(StepPath, StepChooser)> = STEP_PATHS
        .iter()
        .flat_map(|path| STEP_CHOOSERS.iter().map(move |chooser| (*path, *chooser)))
        .collect();
    in_parallel(&pairs, |(path, chooser)| lacking_step_rows(path, chooser))
}

#[test]
fn a_fault_of_a_candidate_binding_draws_only_when_the_evaluation_chooses_that_candidate() {
    let mut failures = Vec::new();
    let mut programs = 0;
    for fault in CANDIDATE_FAULTS {
        for chooser in FAULT_CHOOSERS {
            for consumer in START_CONSUMERS {
                for (index, levels) in Levels::of(&[0, 1], &[1, 3, 30]) {
                    let seconds: &[Option<bool>] = match levels {
                        Levels::Base(_) if fault == CandidateFault::ClosesCycle && index == 0 => {
                            &[None, Some(false), Some(true)]
                        }
                        Levels::Base(_) | Levels::Alternating(..) => &[None],
                    };
                    for (reversed, second) in [false, true]
                        .into_iter()
                        .flat_map(|reversed| seconds.iter().map(move |second| (reversed, *second)))
                    {
                        let program = |recursive| {
                            candidate_fault_program(
                                (fault, chooser, consumer),
                                (index, levels, reversed, recursive),
                                second,
                            )
                        };
                        let chosen: Vec<usize> = levels
                            .reads(index)
                            .into_iter()
                            .map(|(chosen, _)| chosen)
                            .collect();
                        let reading = chosen.contains(&1);
                        let unread = match levels {
                            Levels::Base(_) => index,
                            Levels::Alternating(sum, depth) if depth % 2 == 1 => sum - index,
                            Levels::Alternating(..) => index,
                        };
                        let checked = reading || unread == 1;
                        if fault == CandidateFault::ClosesCycle
                            && consumer == Consumer::RecursiveArgument
                            && !chosen.contains(&unread)
                        {
                            continue;
                        }
                        let (text, twin) = (program(true), program(false));
                        let mut expected = codes_at(&twin, levels);
                        if fault == CandidateFault::ReadsCell
                            && reading
                            && consumer != Consumer::RecursiveArgument
                        {
                            let call = format!(
                                "h(verse, other, [verse, other], {index}, {})",
                                levels.depth()
                            );
                            *expected.entry((Code::ReactiveRecursion, call)).or_default() += 1;
                        }
                        let drawn = codes_at(&text, levels);
                        programs += 1;
                        let faulty_twin = !found(&twin).is_empty();
                        let twin_faults = second == Some(true)
                            || fault == CandidateFault::EveryCandidateClosesCycle
                            || match fault {
                                CandidateFault::LacksAttribute
                                    if consumer == Consumer::RecursiveArgument =>
                                {
                                    reading
                                }
                                CandidateFault::LacksAttribute => checked,
                                CandidateFault::ClosesCycle => checked,
                                CandidateFault::ReadsCell => {
                                    reading && consumer == Consumer::RecursiveArgument
                                }
                                CandidateFault::EveryCandidateClosesCycle => true,
                            };
                        if drawn != expected || faulty_twin != twin_faults {
                            failures.push(format!(
                                    "{fault:?} {chooser} {consumer:?} {index} {levels:?} {reversed} {second:?}\n{text}\n  found {drawn:?}\n  expected {expected:?}\n  twin faults {faulty_twin}\n"
                                ));
                        }
                    }
                }
            }
        }
    }
    let (lacking, lacking_programs) = lacking_step_failures();
    failures.extend(lacking);
    programs += lacking_programs;
    let (crossing, crossing_programs) = in_parallel(&CALLEE_REACHES, callee_template_rows);
    failures.extend(crossing);
    programs += crossing_programs;
    assert!(
        failures.is_empty(),
        "{} of {programs} programs fail:\n{}",
        failures.len(),
        failures.iter().take(8).cloned().collect::<String>()
    );
    assert_eq!(programs, 63_416);
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum CalleeReach {
    Direct,
    Through,
    Twice,
    AtBase,
    Own(OwnStep),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum OwnStep {
    Appended,
    Prepended,
    Sliced,
}

const CALLEE_REACHES: [CalleeReach; 7] = [
    CalleeReach::Direct,
    CalleeReach::Through,
    CalleeReach::Twice,
    CalleeReach::AtBase,
    CalleeReach::Own(OwnStep::Appended),
    CalleeReach::Own(OwnStep::Prepended),
    CalleeReach::Own(OwnStep::Sliced),
];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum CalleeRead {
    Taken,
    Untaken,
    Child,
}

const CALLEE_READS: [CalleeRead; 3] = [CalleeRead::Taken, CalleeRead::Untaken, CalleeRead::Child];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum CallerHolds {
    NoSlot,
    Lacking,
    Cycle,
    Integer,
}

const CALLER_HOLDS: [CallerHolds; 4] = [
    CallerHolds::NoSlot,
    CallerHolds::Lacking,
    CallerHolds::Cycle,
    CallerHolds::Integer,
];

impl CalleeReach {
    fn holds(self) -> &'static [CallerHolds] {
        match self {
            CalleeReach::Own(_) => &CALLER_HOLDS[..3],
            CalleeReach::Direct
            | CalleeReach::Through
            | CalleeReach::Twice
            | CalleeReach::AtBase => &CALLER_HOLDS,
        }
    }

    fn indices(self, depth: usize) -> [usize; 2] {
        match self {
            CalleeReach::Own(OwnStep::Sliced) => [depth + 1, depth + 2],
            CalleeReach::Own(OwnStep::Appended | OwnStep::Prepended) => [1, 2],
            CalleeReach::Direct
            | CalleeReach::Through
            | CalleeReach::Twice
            | CalleeReach::AtBase => [0, 1],
        }
    }
}

impl OwnStep {
    fn step(self, depth: usize, [first, second]: [usize; 2]) -> (&'static str, String, String) {
        match self {
            OwnStep::Appended => ("xs + [other]", "i".to_string(), "3 - i".to_string()),
            OwnStep::Prepended => (
                "[other] + xs",
                "i + 1".to_string(),
                format!("(if i > {depth} - n and i < {depth} - n + 2 then 0 else {depth} - n + 2)"),
            ),
            OwnStep::Sliced => (
                "xs[1..]",
                "i - 1".to_string(),
                format!("{} - i - 2 * ({depth} - n)", first + second - 1),
            ),
        }
    }
}

impl CalleeRead {
    fn text(self) -> &'static str {
        match self {
            CalleeRead::Taken => "i32(xs[i].gain)",
            CalleeRead::Untaken => "(if false then i32(xs[i].gain) else 0) + 1",
            CalleeRead::Child => "i32(xs[i].hook.velocity)",
        }
    }

    fn value(self, verse: bool) -> usize {
        match (self, verse) {
            (CalleeRead::Untaken, _) => 1,
            (CalleeRead::Taken, true) => 2,
            (CalleeRead::Taken, false) => 3,
            (CalleeRead::Child, true) => 12,
            (CalleeRead::Child, false) => 13,
        }
    }
}

impl CallerHolds {
    fn slot(self) -> usize {
        match self {
            CallerHolds::Integer => 5,
            CallerHolds::NoSlot | CallerHolds::Lacking | CallerHolds::Cycle => 2,
        }
    }

    fn element(self) -> &'static str {
        match self {
            CallerHolds::NoSlot => "other",
            CallerHolds::Lacking | CallerHolds::Integer => "verse.tone",
            CallerHolds::Cycle => "cyc",
        }
    }
}

fn recursive_function(
    (name, params): (&str, &str),
    (condition, base): (&str, &str),
    step: impl Fn(Option<&str>) -> String,
    levels: Option<usize>,
) -> String {
    let Some(levels) = levels else {
        return format!(
            "fn {name}({params}): if {condition} then {base} else {}\n",
            step(Some(name))
        );
    };
    let named = |level: usize| match level {
        0 => name.to_string(),
        _ => format!("{name}{level}"),
    };
    (0..=levels)
        .map(|level| {
            let next = (level < levels).then(|| named(level + 1));
            format!(
                "fn {}({params}): if {condition} then {base} else {}\n",
                named(level),
                step(next.as_deref())
            )
        })
        .collect()
}

struct CalleeCase {
    reach: CalleeReach,
    read: CalleeRead,
    holds: CallerHolds,
    depth: usize,
    alternating: bool,
    index: usize,
    reversed: bool,
}

impl CalleeCase {
    fn total(&self) -> usize {
        let [first, _] = self.reach.indices(self.depth);
        let choice = |level: usize| {
            let flipped = self.alternating && level % 2 == 1;
            self.read.value((self.index == first) != flipped)
        };
        match self.reach {
            CalleeReach::Own(_) | CalleeReach::AtBase => choice(self.depth),
            CalleeReach::Twice => (0..self.depth).map(|level| 2 * choice(level)).sum(),
            CalleeReach::Direct | CalleeReach::Through => (0..self.depth).map(choice).sum(),
        }
    }

    fn own_function(&self, own: OwnStep, unrolled: bool) -> (String, String) {
        let depth = self.depth;
        let (argument, constant, alternating) = own.step(depth, self.reach.indices(depth));
        let next = if self.alternating {
            alternating
        } else {
            constant
        };
        let function = recursive_function(
            ("g", "xs, i, n"),
            ("n < 1", self.read.text()),
            |callee| {
                callee.map_or("0".to_string(), |g| {
                    format!("{g}({argument}, {next}, n - 1)")
                })
            },
            unrolled.then_some(depth),
        );
        let element = self.holds.element();
        let start = match own {
            OwnStep::Sliced => {
                let mut elements = vec![element; depth + 1];
                elements.extend(["verse", "other"]);
                elements.join(", ")
            }
            OwnStep::Appended | OwnStep::Prepended => format!("{element}, verse, other"),
        };
        (function, format!("g([{start}], {}, {depth})", self.index))
    }

    fn caller_functions(&self, unrolled: bool) -> (String, String) {
        let depth = self.depth;
        let index = self.index;
        let levels = |count: usize| unrolled.then_some(count);
        let mut functions = recursive_function(
            ("g", "xs, i, n"),
            ("n < 1", self.read.text()),
            |callee| callee.map_or("0".to_string(), |g| format!("{g}(xs, i, n - 1)")),
            levels(1),
        );
        if self.reach == CalleeReach::Through {
            functions.push_str(&recursive_function(
                ("p", "zs, k, n"),
                ("n < 1", "g(zs, k, 1)"),
                |callee| callee.map_or("0".to_string(), |p| format!("{p}(zs, k, n - 1)")),
                levels(1),
            ));
        }
        let next_index = if self.alternating { "1 - j" } else { "j" };
        let (params, condition, array, chosen, next, call) = match self.holds {
            CallerHolds::NoSlot => (
                "m",
                "m < 1",
                "[verse, other, verse.tone]",
                if self.alternating {
                    format!("(m + {}) % 2", index + depth)
                } else {
                    index.to_string()
                },
                "m - 1".to_string(),
                format!("h({depth})"),
            ),
            CallerHolds::Lacking | CallerHolds::Cycle | CallerHolds::Integer => (
                "ys, k, j, n",
                "n < 1",
                "ys",
                "j".to_string(),
                format!("ys, k, {next_index}, n - 1"),
                format!(
                    "h([verse, other, {}], {}, {index}, {depth})",
                    self.holds.element(),
                    self.holds.slot()
                ),
            ),
        };
        let callee = match self.reach {
            CalleeReach::Through => format!("p({array}, {chosen}, 1)"),
            CalleeReach::Twice => {
                format!("g({array}, {chosen}, 1) + g({array}, {chosen}, 1)")
            }
            CalleeReach::Direct | CalleeReach::AtBase | CalleeReach::Own(_) => {
                format!("g({array}, {chosen}, 1)")
            }
        };
        let at_base = self.reach == CalleeReach::AtBase;
        let base = if at_base { callee.as_str() } else { "0" };
        functions.push_str(&recursive_function(
            ("h", params),
            (condition, base),
            |caller| {
                let recursion = caller.map_or("0".to_string(), |h| format!("{h}({next})"));
                if at_base {
                    recursion
                } else {
                    format!("{callee} + {recursion}")
                }
            },
            levels(depth),
        ));
        (functions, call)
    }

    fn program(&self, unrolled: bool) -> String {
        let (functions, call) = match self.reach {
            CalleeReach::Own(own) => self.own_function(own, unrolled),
            CalleeReach::Direct
            | CalleeReach::Through
            | CalleeReach::Twice
            | CalleeReach::AtBase => self.caller_functions(unrolled),
        };
        let mut items = vec![
            "Session main\n".to_string(),
            "Clip verse | gain=2:\n    Note as hook | velocity=12\n    Note as tone\n".to_string(),
            "Clip other | gain=3:\n    Note as hook | velocity=13\n".to_string(),
            functions,
            format!(
                "Clip k | gain={call}:\n    Note/([60][i32(k.gain) - {}])\n",
                self.total()
            ),
        ];
        if self.holds == CallerHolds::Cycle {
            items.push("Clip cyc | gain=k.gain:\n    Note as hook | velocity=k.gain\n".to_string());
        }
        if self.reversed {
            items.reverse();
        }
        items.concat()
    }
}

fn callee_template_rows(reach: CalleeReach) -> (Vec<String>, usize) {
    let mut failures = Vec::new();
    let mut programs = 0;
    for read in CALLEE_READS {
        for holds in reach.holds().iter().copied() {
            for depth in [1, 3, 30] {
                for alternating in [false, true] {
                    for index in reach.indices(depth) {
                        for reversed in [false, true] {
                            let case = CalleeCase {
                                reach,
                                read,
                                holds,
                                depth,
                                alternating,
                                index,
                                reversed,
                            };
                            let text = case.program(false);
                            let twin = case.program(true);
                            let drawn = found(&text);
                            let expected = found(&twin);
                            programs += 1;
                            if !expected.is_empty() || drawn != expected {
                                failures.push(format!(
                                    "{reach:?} {read:?} {holds:?} {depth} {alternating} {index} {reversed}\n{text}\n  found {drawn:?}\n  twin {expected:?}\n"
                                ));
                            }
                        }
                    }
                }
            }
        }
    }
    (failures, programs)
}

fn two_calls_program(index: usize, levels: Levels, reversed: bool, recursive: bool) -> String {
    let depth = levels.depth();
    let mut items = [
        "Session main\n".to_string(),
        "Clip verse | gain=k.gain:\n    Note as hook\n".to_string(),
        "Clip other | gain=k.gain:\n    Note\n".to_string(),
        levels.functions(
            ("c, d, xs, i, n", "", "(xs[i]).gain"),
            |next| format!("c, d, xs, {next}, n - 1"),
            recursive,
        ),
        format!(
            "Clip k | gain=h(verse, other, [verse, other], {index}, {depth}) + h(verse, other, [verse, other], {}, {depth}):\n    Note\n",
            1 - index
        ),
    ];
    if reversed {
        items.reverse();
    }
    items.concat()
}

#[test]
fn two_outermost_calls_choosing_two_cycles_through_one_declaration_draw_one_declaration_cycle() {
    let mut failures = Vec::new();
    let mut programs = 0;
    for (index, levels) in Levels::of(&[0, 1], &[1, 3, 30]) {
        for reversed in [false, true] {
            let text = two_calls_program(index, levels, reversed, true);
            let twin = two_calls_program(index, levels, reversed, false);
            let drawn = found(&text);
            let expected = found(&twin);
            let cycles: usize = drawn
                .iter()
                .filter(|((code, _), _)| *code == Code::DeclarationCycle)
                .map(|(_, count)| *count)
                .sum();
            programs += 1;
            if drawn != expected || cycles != 1 {
                failures.push(format!(
                    "{index} {levels:?} {reversed}\n{text}\n  found {drawn:?}\n  twin {expected:?}\n"
                ));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {programs} programs fail:\n{}",
        failures.len(),
        failures.iter().take(8).cloned().collect::<String>()
    );
    assert_eq!(programs, 24);
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum UntakenStart {
    Recursive,
    Direct,
    Inlined,
    Slice,
}

const UNTAKEN_STARTS: [UntakenStart; 4] = [
    UntakenStart::Recursive,
    UntakenStart::Direct,
    UntakenStart::Inlined,
    UntakenStart::Slice,
];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum StartFault {
    None,
    Outside,
    Fit,
    Overflow,
    Depth,
}

const START_FAULTS: [StartFault; 5] = [
    StartFault::None,
    StartFault::Outside,
    StartFault::Fit,
    StartFault::Overflow,
    StartFault::Depth,
];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Guard {
    AtEnd,
    BeforeEnd,
    IfFalse,
    IfDeep,
    Taken,
}

const GUARDS: [Guard; 5] = [
    Guard::AtEnd,
    Guard::BeforeEnd,
    Guard::IfFalse,
    Guard::IfDeep,
    Guard::Taken,
];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum StartReader {
    Path,
    LocalLet,
    RecursiveArgument,
}

const START_READERS: [StartReader; 3] = [
    StartReader::Path,
    StartReader::LocalLet,
    StartReader::RecursiveArgument,
];

impl StartFault {
    fn index(self) -> &'static str {
        match self {
            StartFault::None | StartFault::Depth => "i % len(xs)",
            StartFault::Outside => "i",
            StartFault::Fit => "u8(i + 300) - 300",
            StartFault::Overflow => "i + 2 ** 126 * 4",
        }
    }

    fn index_at(self, level: usize) -> String {
        match self {
            StartFault::None | StartFault::Depth => format!("{level} % {level}"),
            StartFault::Outside => level.to_string(),
            StartFault::Fit => format!("u8({level} + 300) - 300"),
            StartFault::Overflow => format!("{level} + 2 ** 126 * 4"),
        }
    }

    fn at_every_level(self) -> bool {
        match self {
            StartFault::Fit | StartFault::Overflow | StartFault::Depth => true,
            StartFault::None | StartFault::Outside => false,
        }
    }

    fn drawn(self, start: UntakenStart) -> Option<(Code, &'static str)> {
        match (self, start) {
            (StartFault::None, _) => None,
            (StartFault::Outside, UntakenStart::Recursive) => Some((Code::IndexOutOfRange, "p")),
            (StartFault::Outside, UntakenStart::Direct) => Some((Code::IndexOutOfRange, "i")),
            (StartFault::Outside, UntakenStart::Inlined) => Some((Code::IndexOutOfRange, "q")),
            (StartFault::Outside, UntakenStart::Slice) => Some((Code::IndexOutOfRange, "0")),
            (StartFault::Fit, _) => Some((Code::DoesNotFit, "300")),
            (StartFault::Overflow, _) => Some((Code::IntegerOverflow, "2 ** 126 * 4")),
            (StartFault::Depth, _) => Some((Code::RecursionDepth, "pk(xs, p, m - 1)")),
        }
    }
}

impl UntakenStart {
    fn later_reads(self) -> &'static [bool] {
        match self {
            UntakenStart::Recursive => &[false, true],
            UntakenStart::Direct | UntakenStart::Inlined | UntakenStart::Slice => &[false],
        }
    }

    fn faults(self) -> &'static [StartFault] {
        match self {
            UntakenStart::Recursive => &START_FAULTS,
            UntakenStart::Direct | UntakenStart::Inlined | UntakenStart::Slice => {
                &START_FAULTS[..4]
            }
        }
    }

    fn text(self, fault: StartFault) -> String {
        let index = fault.index();
        match self {
            UntakenStart::Recursive => {
                let depth = if fault == StartFault::Depth {
                    200_000
                } else {
                    1
                };
                format!("pk(xs, {index}, {depth})")
            }
            UntakenStart::Direct => format!("xs[{index}]"),
            UntakenStart::Inlined => format!("pick(xs, {index})"),
            UntakenStart::Slice => format!("xs[{index}..][0]"),
        }
    }
}

impl Guard {
    fn reads_taken(self, level: usize, depth: usize) -> bool {
        match self {
            Guard::AtEnd | Guard::BeforeEnd => level < depth,
            Guard::IfFalse | Guard::IfDeep => false,
            Guard::Taken => true,
        }
    }

    fn body(self, read: &str) -> String {
        let next = "h(xs, i + 1)";
        match self {
            Guard::AtEnd => format!("if i >= len(xs) then 0 else {read} + {next}"),
            Guard::BeforeEnd => format!("if i < len(xs) then {read} + {next} else 0"),
            Guard::IfFalse => {
                format!("if i >= len(xs) then 0 else (if false then {read} else 0) + {next}")
            }
            Guard::IfDeep => {
                format!("if i >= len(xs) then 0 else (if i > 100 then {read} else 0) + {next}")
            }
            Guard::Taken => format!("if i >= len(xs) then {read} else {read} + {next}"),
        }
    }
}

struct UntakenCase {
    start: UntakenStart,
    fault: StartFault,
    guard: Guard,
    reader: StartReader,
    depth: usize,
    later: bool,
    reversed: bool,
}

impl UntakenCase {
    fn expected(&self) -> BTreeSet<(Code, String)> {
        let (mut expected, _) = self.drawn_by_h();
        if self.later {
            expected.extend(
                self.fault
                    .drawn(self.start)
                    .map(|(code, at)| (code, at.to_string())),
            );
        }
        expected
    }

    fn drawn_by_h(&self) -> (BTreeSet<(Code, String)>, usize) {
        let taken: Vec<usize> = (0..=self.depth)
            .filter(|level| self.guard.reads_taken(*level, self.depth))
            .collect();
        let fires = taken
            .iter()
            .any(|level| self.fault.at_every_level() || *level == self.depth);
        let total = taken
            .iter()
            .map(|level| {
                if (level % self.depth).is_multiple_of(2) {
                    2
                } else {
                    3
                }
            })
            .sum();
        match self.fault.drawn(self.start) {
            Some((code, at)) if fires => ([(code, at.to_string())].into(), total),
            Some(_) | None => (BTreeSet::new(), total),
        }
    }

    fn program(&self, unrolled: bool) -> String {
        let start = self.start.text(self.fault);
        let (local, read) = match self.reader {
            StartReader::Path => (String::new(), format!("i32(({start}).gain)")),
            StartReader::LocalLet => (format!("    let e = {start}\n"), "i32(e.gain)".to_string()),
            StartReader::RecursiveArgument => (String::new(), format!("i32(gs({start}, 2))")),
        };
        let body = self.guard.body(&read);
        let function = if unrolled {
            (0..=self.depth)
                .map(|level| {
                    let name = |level: usize| match level {
                        0 => "h".to_string(),
                        _ => format!("h{level}"),
                    };
                    let next = if level < self.depth {
                        format!("{}(xs, i + 1)", name(level + 1))
                    } else {
                        "0".to_string()
                    };
                    format!(
                        "fn {}(xs, i):\n{local}    {}\n",
                        name(level),
                        body.replace("h(xs, i + 1)", &next)
                    )
                })
                .collect()
        } else {
            format!("fn h(xs, i):\n{local}    {body}\n")
        };
        let references = (0..self.depth)
            .map(|level| if level % 2 == 0 { "verse" } else { "other" })
            .collect::<Vec<_>>()
            .join(", ");
        let (faults, total) = self.drawn_by_h();
        let reader = if faults.is_empty() {
            format!("Clip k:\n    Note/([60][h([{references}], 0) - {total}])\n")
        } else {
            format!("Clip k | gain=h([{references}], 0):\n    Note\n")
        };
        let depth = if self.fault == StartFault::Depth {
            200_000
        } else {
            1
        };
        let later = format!(
            "pk([{references}], {}, {depth})",
            self.fault.index_at(self.depth)
        );
        let later = match self.fault {
            StartFault::None => format!("Clip z:\n    Note/([60][i32(({later}).gain) - 2])\n"),
            StartFault::Outside | StartFault::Fit | StartFault::Overflow | StartFault::Depth => {
                format!("Clip z | gain=({later}).gain:\n    Note\n")
            }
        };
        let mut items = vec![
            "Session main\n".to_string(),
            "Clip verse | gain=2:\n    Note\n".to_string(),
            "Clip other | gain=3:\n    Note\n".to_string(),
            "fn pk(xs, p, m): if m < 1 then xs[p] else pk(xs, p, m - 1)\n".to_string(),
            "fn pick(ys, q): ys[q]\n".to_string(),
            "fn gs(c, n): if n < 1 then c.gain else gs(c, n - 1)\n".to_string(),
            function,
            reader,
        ];
        if self.later {
            items.push(later);
        }
        if self.reversed {
            items.reverse();
        }
        items.concat()
    }
}

fn drawn_keys(text: &str) -> BTreeSet<(Code, String)> {
    found(text).into_keys().collect()
}

fn untaken_start_rows(start: UntakenStart) -> (Vec<String>, usize) {
    let mut failures = Vec::new();
    let mut programs = 0;
    for fault in start.faults().iter().copied() {
        for guard in GUARDS {
            for reader in START_READERS {
                for (depth, later) in [1, 3, 30]
                    .into_iter()
                    .flat_map(|depth| start.later_reads().iter().map(move |later| (depth, *later)))
                {
                    for reversed in [false, true] {
                        let case = UntakenCase {
                            start,
                            fault,
                            guard,
                            reader,
                            depth,
                            later,
                            reversed,
                        };
                        let expected = case.expected();
                        let text = case.program(false);
                        let drawn = drawn_keys(&text);
                        let twin = (guard == Guard::Taken).then(|| drawn_keys(&case.program(true)));
                        programs += 1;
                        if drawn != expected || twin.as_ref().is_some_and(|twin| *twin != expected)
                        {
                            failures.push(format!(
                                "{start:?} {fault:?} {guard:?} {reader:?} {depth} {later} {reversed}\n{text}\n  found {drawn:?}\n  twin {twin:?}\n  expected {expected:?}\n"
                            ));
                        }
                    }
                }
            }
        }
    }
    (failures, programs)
}

fn quiet_callee_read_rows() -> (Vec<String>, usize) {
    let mut failures = Vec::new();
    let mut programs = 0;
    for fault in [
        CandidateFault::LacksAttribute,
        CandidateFault::ClosesCycle,
        CandidateFault::ReadsCell,
    ] {
        for condition in ["false", "n > 100"] {
            for depth in [1, 3] {
                for reversed in [false, true] {
                    let (other, candidate) = match fault {
                        CandidateFault::LacksAttribute => {
                            ("Clip other | gain=3:\n    Note\n", "verse.tone")
                        }
                        CandidateFault::ClosesCycle | CandidateFault::EveryCandidateClosesCycle => {
                            ("Clip other | gain=k.gain:\n    Note\n", "other")
                        }
                        CandidateFault::ReadsCell => ("Clip other | gain=gv:\n    Note\n", "other"),
                    };
                    let mut items = [
                        "Session main\n".to_string(),
                        "input gv = 1.0\n".to_string(),
                        "Clip verse | gain=2:\n    Note as tone\n".to_string(),
                        other.to_string(),
                        "fn sel(xs, i, n): if n < 1 then (if i32(xs[i].gain) > 0 then xs[0] else xs[0]) else sel(xs, i, n - 1)\n".to_string(),
                        format!("fn h(xs, i, n): if n < 1 then 0 else (if {condition} then i32(sel(xs, i, 1).gain) else 0) + 1 + h(xs, i, n - 1)\n"),
                        format!("Clip k | gain=h([verse, {candidate}], 1, {depth}):\n    Note/([60][i32(k.gain) - {depth}])\n"),
                    ];
                    if reversed {
                        items.reverse();
                    }
                    let text = items.concat();
                    let drawn = found(&text);
                    programs += 1;
                    if !drawn.is_empty() {
                        failures.push(format!(
                            "{fault:?} {condition} {depth} {reversed}\n{text}\n  found {drawn:?}\n"
                        ));
                    }
                }
            }
        }
    }
    (failures, programs)
}

#[test]
fn evaluating_the_start_of_an_untaken_arm_draws_no_fit_range_overflow_or_depth_diagnostic_in_any_frame()
 {
    let (mut failures, mut programs) = in_parallel(&UNTAKEN_STARTS, untaken_start_rows);
    let (callee, callee_programs) = quiet_callee_read_rows();
    failures.extend(callee);
    programs += callee_programs;
    assert!(
        failures.is_empty(),
        "{} of {programs} programs fail:\n{}",
        failures.len(),
        failures.iter().take(8).cloned().collect::<String>()
    );
    assert_eq!(programs, 2_004);
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum OpenCallArm {
    IfFalse,
    IfDeep,
    AboveOne,
}

const OPEN_CALL_ARMS: [OpenCallArm; 3] = [
    OpenCallArm::IfFalse,
    OpenCallArm::IfDeep,
    OpenCallArm::AboveOne,
];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum OpenCallTerm {
    TakenRead,
    Recursion,
    UntakenRead,
}

impl OpenCallArm {
    fn condition(self) -> &'static str {
        match self {
            OpenCallArm::IfFalse => "false",
            OpenCallArm::IfDeep => "n > 100",
            OpenCallArm::AboveOne => "n > 1",
        }
    }

    fn takes(self, n: i64) -> bool {
        match self {
            OpenCallArm::IfFalse => false,
            OpenCallArm::IfDeep => n > 100,
            OpenCallArm::AboveOne => n > 1,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum OpenCallUse {
    H(i64),
    G(i64),
}

struct OpenCallModel<'a> {
    offset: i64,
    arm: OpenCallArm,
    terms: &'a [OpenCallTerm],
    taken_read: bool,
    memo: BTreeMap<(bool, i64), i64>,
    open: BTreeSet<(bool, i64)>,
}

impl OpenCallModel<'_> {
    fn g(&mut self, m: i64) -> Option<i64> {
        if let Some(value) = self.memo.get(&(false, m)) {
            return Some(*value);
        }
        if !self.open.insert((false, m)) {
            return None;
        }
        let other = m >= 1 && self.h(m - 1)? > 2;
        self.open.remove(&(false, m));
        let gain = if other { 3 } else { 2 };
        self.memo.insert((false, m), gain);
        Some(gain)
    }

    fn h(&mut self, n: i64) -> Option<i64> {
        if let Some(value) = self.memo.get(&(true, n)) {
            return Some(*value);
        }
        if n > 200 || !self.open.insert((true, n)) {
            return None;
        }
        let mut value = 0;
        if n < 1 {
            value = 1;
        } else {
            for term in self.terms {
                value += match term {
                    OpenCallTerm::TakenRead if self.taken_read => self.g(n)?,
                    OpenCallTerm::TakenRead => 0,
                    OpenCallTerm::Recursion => self.h(n - 1)?,
                    OpenCallTerm::UntakenRead if self.arm.takes(n) => self.g(n + self.offset)?,
                    OpenCallTerm::UntakenRead => 0,
                };
            }
        }
        self.open.remove(&(true, n));
        self.memo.insert((true, n), value);
        Some(value)
    }
}

fn open_call_rows(offset: i64) -> (Vec<String>, usize) {
    let orders = [
        [
            OpenCallTerm::TakenRead,
            OpenCallTerm::Recursion,
            OpenCallTerm::UntakenRead,
        ],
        [
            OpenCallTerm::TakenRead,
            OpenCallTerm::UntakenRead,
            OpenCallTerm::Recursion,
        ],
        [
            OpenCallTerm::Recursion,
            OpenCallTerm::TakenRead,
            OpenCallTerm::UntakenRead,
        ],
        [
            OpenCallTerm::Recursion,
            OpenCallTerm::UntakenRead,
            OpenCallTerm::TakenRead,
        ],
        [
            OpenCallTerm::UntakenRead,
            OpenCallTerm::TakenRead,
            OpenCallTerm::Recursion,
        ],
        [
            OpenCallTerm::UntakenRead,
            OpenCallTerm::Recursion,
            OpenCallTerm::TakenRead,
        ],
    ];
    let mut failures = Vec::new();
    let mut programs = 0;
    for arm in OPEN_CALL_ARMS {
        for terms in &orders {
            for taken_read in [true, false] {
                for depth in [1, 2, 3, 6] {
                    for uses in [
                        vec![OpenCallUse::H(depth)],
                        vec![OpenCallUse::H(depth), OpenCallUse::G(depth + offset)],
                        vec![OpenCallUse::H(depth), OpenCallUse::G(depth + offset + 1)],
                        vec![OpenCallUse::G(depth + offset)],
                    ] {
                        let mut model = OpenCallModel {
                            offset,
                            arm,
                            terms,
                            taken_read,
                            memo: BTreeMap::new(),
                            open: BTreeSet::new(),
                        };
                        let values: Option<Vec<i64>> = uses
                            .iter()
                            .map(|used| match used {
                                OpenCallUse::H(n) => model.h(*n),
                                OpenCallUse::G(m) => model.g(*m),
                            })
                            .collect();
                        let Some(values) = values else {
                            continue;
                        };
                        let body: Vec<String> = terms
                            .iter()
                            .filter_map(|term| match term {
                                OpenCallTerm::TakenRead => {
                                    taken_read.then(|| "i32(g(xs, n).gain)".to_string())
                                }
                                OpenCallTerm::Recursion => Some("h(xs, n - 1)".to_string()),
                                OpenCallTerm::UntakenRead => Some(format!(
                                    "(if {} then i32(g(xs, n + {offset}).gain) else 0)",
                                    arm.condition()
                                )),
                            })
                            .collect();
                        for reversed in [false, true] {
                            for wrong in [false, true] {
                                let mut expected = Found::new();
                                let notes: String = uses
                                    .iter()
                                    .zip(&values)
                                    .enumerate()
                                    .map(|(position, (used, value))| {
                                        let read = match used {
                                            OpenCallUse::H(n) => format!("h([verse, other], {n})"),
                                            OpenCallUse::G(m) => {
                                                format!("i32(g([verse, other], {m}).gain)")
                                            }
                                        };
                                        let past = wrong && position + 1 == uses.len();
                                        let offset = value + i64::from(past);
                                        if past {
                                            expected.insert(
                                                (
                                                    Code::IndexOutOfRange,
                                                    format!("{read} - {offset}"),
                                                ),
                                                1,
                                            );
                                        }
                                        format!("    Note/([60][{read} - {offset}])\n")
                                    })
                                    .collect();
                                let mut items = [
                                    "Session main\n".to_string(),
                                    "Clip verse | gain=2:\n    Note\n".to_string(),
                                    "Clip other | gain=3:\n    Note\n".to_string(),
                                    "fn g(xs, m): if m < 1 then xs[0] else (if h(xs, m - 1) > 2 then xs[1] else xs[0])\n".to_string(),
                                    format!("fn h(xs, n): if n < 1 then 1 else {}\n", body.join(" + ")),
                                    format!("Clip k:\n{notes}"),
                                ];
                                if reversed {
                                    items.reverse();
                                }
                                let text = items.concat();
                                let drawn = found(&text);
                                programs += 1;
                                if drawn != expected {
                                    failures.push(format!(
                                        "{offset} {arm:?} {terms:?} {taken_read} {depth} {uses:?} {reversed} {wrong}\n{text}\n  found {drawn:?}\n  expected {expected:?}\n"
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    (failures, programs)
}

fn open_function_rows() -> (Vec<String>, usize) {
    let mut failures = Vec::new();
    let mut programs = 0;
    for lacking in [false, true] {
        for condition in ["false", "n > 100"] {
            for depth in [1, 3] {
                for reversed in [false, true] {
                    let second = if lacking { "verse.tone" } else { "other" };
                    let expected: Found = if lacking {
                        [((Code::UnknownAttribute, "gain".to_string()), 1)].into()
                    } else {
                        Found::new()
                    };
                    let mut items = [
                        "Session main\n".to_string(),
                        "Clip verse | gain=2:\n    Note as tone\n".to_string(),
                        "Clip other | gain=3:\n    Note\n".to_string(),
                        "fn g(xs, m): if h(xs, m - 5) > 0 then xs[1] else xs[0]\n".to_string(),
                        format!(
                            "fn h(xs, n): if n < 1 then 1 else h(xs, n - 1) + (if {condition} then i32(g(xs, n).gain) else 0)\n"
                        ),
                        format!("Clip k:\n    Note/([60][h([verse, {second}], {depth}) - 1])\n"),
                    ];
                    if reversed {
                        items.reverse();
                    }
                    let text = items.concat();
                    let drawn = found(&text);
                    programs += 1;
                    if drawn != expected {
                        failures.push(format!(
                            "{lacking} {condition} {depth} {reversed}\n{text}\n  found {drawn:?}\n  expected {expected:?}\n"
                        ));
                    }
                }
            }
        }
    }
    (failures, programs)
}

fn tainted_start_rows() -> (Vec<String>, usize) {
    let mut failures = Vec::new();
    let mut programs = 0;
    for condition in ["false", "n > 100"] {
        for depth in [2, 3, 6] {
            for reversed in [false, true] {
                let mut items = [
                    "Session main\n".to_string(),
                    "Clip verse | gain=2:\n    Note as tone\n".to_string(),
                    format!(
                        "fn g(xs, m): if h(xs, m - 1) > {} then xs[1] else xs[0]\n",
                        depth - 1
                    ),
                    format!(
                        "fn h(xs, n): if n < 1 then 1 else h(xs, n - 1) + 1 + (if {condition} then i32(g(xs, n).gain) else 0) + (if {condition} then i32(g(xs, n + 1).gain) else 0)\n"
                    ),
                    format!(
                        "Clip k:\n    Note/([60][h([verse, verse.tone], {depth}) - {}])\n",
                        depth + 1
                    ),
                ];
                if reversed {
                    items.reverse();
                }
                let text = items.concat();
                let drawn = found(&text);
                let expected: Found = [((Code::UnknownAttribute, "gain".to_string()), 1)].into();
                programs += 1;
                if drawn != expected {
                    failures.push(format!(
                        "{condition} {depth} {reversed}\n{text}\n  found {drawn:?}\n  expected {expected:?}\n"
                    ));
                }
            }
        }
    }
    (failures, programs)
}

#[test]
fn an_untaken_arms_start_that_reaches_an_open_call_leaves_no_value_any_later_evaluation_reads() {
    let (mut failures, mut programs) = in_parallel(&[0, 1, 2], open_call_rows);
    let (open, open_programs) = open_function_rows();
    failures.extend(open);
    programs += open_programs;
    let (tainted, tainted_programs) = tainted_start_rows();
    failures.extend(tainted);
    programs += tainted_programs;
    assert!(
        failures.is_empty(),
        "{} of {programs} programs fail:\n{}",
        failures.len(),
        failures.iter().take(8).cloned().collect::<String>()
    );
    assert_eq!(programs, 5_596);
}
