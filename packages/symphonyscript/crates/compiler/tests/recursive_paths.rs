mod support;

use std::collections::BTreeMap;

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
        failures.iter().take(12).cloned().collect::<String>()
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
}

const CHOOSERS: [Chooser; 19] = [
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
            | Chooser::RecursiveHelper => "",
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

fn chosen_function(chooser: Chooser, (local, read): (&str, &str), recursive: bool) -> String {
    let value = if recursive {
        format!("if n < 1 then {read} else h(c, d, xs, i, n - 1)")
    } else {
        read.to_string()
    };
    format!(
        "fn h(c, d, xs, i, n):\n{}{local}    {value}\n",
        chooser.locals()
    )
}

fn chosen_program(
    (chooser, consumer, binding): (Chooser, Consumer, Binding),
    scale: i32,
    (depth, reversed, recursive): (u32, bool, bool),
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
    items.push(chosen_function(chooser, (&local, &read), recursive));
    let call = format!("h(verse, other, [verse, other], 1, {depth})");
    let read = match scale {
        1 => call,
        _ => format!("{call} * {scale}"),
    };
    items.push(format!(
        "Clip k:\n    Note/([60][i32({read}) - {}])\n",
        chosen_value(binding) * scale
    ));
    if reversed {
        items.reverse();
    }
    items.concat()
}

fn twin_failure(text: &str, twin: &str) -> Option<String> {
    let expected = found(twin);
    let drawn = found(text);
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
    let mut failures = Vec::new();
    let mut programs = 0;
    for chooser in CHOOSERS {
        for consumer in CONSUMERS {
            for binding in CHOSEN_BINDINGS {
                for scale in [1, 2] {
                    for depth in DEPTHS {
                        for reversed in [false, true] {
                            let case = (chooser, consumer, binding);
                            let text = chosen_program(case, scale, (depth, reversed, true));
                            let twin = chosen_program(case, scale, (depth, reversed, false));
                            programs += 1;
                            if let Some(failure) = twin_failure(&text, &twin) {
                                failures.push(format!(
                                    "{chooser:?} {consumer:?} {binding:?} {scale} {depth} {reversed}\n{failure}"
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
    assert_eq!(programs, 3_192);
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

    fn expected(self, read: ArrayRead) -> Option<i32> {
        let rows = self.rows();
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
    (depth, reversed, recursive): (u32, bool, bool),
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
        chosen_function(chooser, (&local, &read), recursive),
        format!(
            "Song s:\n    Clip:\n        Note/([60][h(verse, other, [verse, other], 1, {depth}) - {expected}])\n"
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
    let compiler = Compiler::new(&native_vocabulary()).expect("the native vocabulary compiles");
    let mut failures = Vec::new();
    let mut programs = 0;
    for chooser in CHOOSERS {
        for consumer in ARRAY_CONSUMERS {
            for attribute in ARRAY_ATTRIBUTES {
                for read in ARRAY_READS {
                    let Some(expected) = attribute.expected(read) else {
                        continue;
                    };
                    for depth in DEPTHS {
                        for reversed in [false, true] {
                            let program = |recursive| {
                                array_program(
                                    chooser,
                                    consumer,
                                    (attribute, read),
                                    (depth, reversed, recursive),
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
                                    "{chooser:?} {consumer:?} {attribute:?} {read:?} {depth} {reversed}\n{text}\n  found {drawn:?}\n  twin {twin_drawn:?}\n"
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
    assert_eq!(programs, 8_208);
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
