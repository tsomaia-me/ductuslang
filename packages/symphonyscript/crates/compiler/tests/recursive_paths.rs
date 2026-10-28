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

fn start_program(
    (start, fault, consumer): (Start, Fault, Consumer),
    (candidates, index): (usize, i32),
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
    let expected = match fault {
        Fault::Inside(candidate) => GAINS.get(candidate).copied().unwrap_or_default(),
        Fault::Outside | Fault::Saturating | Fault::Depth => 2,
    };
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
        recursive,
    ));
    items.push(format!(
        "Clip k:\n    Note/([60][i32(h(verse, other, {array}, {index}, 2)) - {expected}])\n"
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
                let Some(index) = start.frame_value(candidates, fault) else {
                    continue;
                };
                if (start, fault) == (Start::SliceThenFirst, Fault::Outside) {
                    continue;
                }
                for consumer in START_CONSUMERS {
                    for reversed in [false, true] {
                        let program = |recursive| {
                            start_program(
                                (start, fault, consumer),
                                (candidates, index),
                                (reversed, recursive),
                            )
                        };
                        let (text, twin) = (program(true), program(false));
                        let (drawn, expected) = (found(&text), found(&twin));
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
                                "{start:?} {fault:?} {candidates} {consumer:?} {reversed}\n{text}\n  found {drawn:?}\n  twin {expected:?}\n"
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
    assert_eq!(programs, 408);
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
}

const CALL_STARTS: [CallStart; 6] = [
    CallStart::Alone,
    CallStart::LetIf,
    CallStart::LetMatch,
    CallStart::LetArray,
    CallStart::SelfRead,
    CallStart::SelfReadWrapped,
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
        matches!(self, CallStart::SelfRead | CallStart::SelfReadWrapped)
    }

    fn expression(self, helper: PathHelper) -> String {
        match self {
            CallStart::Alone => helper.call("xs", "i"),
            CallStart::LetIf => "(if i > 0 then rp else c)".to_string(),
            CallStart::LetMatch => "m".to_string(),
            CallStart::LetArray => "[rp, c][i]".to_string(),
            CallStart::SelfRead => "sf(xs, i, n - 1)".to_string(),
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
            | CallStart::SelfReadWrapped => "",
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
            CallStart::SelfRead | CallStart::SelfReadWrapped => {
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
    (reversed, recursive): (bool, bool),
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
        let value = if recursive {
            format!("if n < 1 then xs[i] else (if {read} > 2 then xs[1] else xs[0])")
        } else {
            "xs[i]".to_string()
        };
        items.push(format!("fn sf(xs, i, n):\n{local}    {value}\n"));
        items.push("fn w(xs, i, n): sf(xs, i, n)\n".to_string());
    } else {
        let value = if recursive {
            format!("if n < 1 then {read} else h(c, xs, i, n - 1)")
        } else {
            read
        };
        items.push(format!(
            "fn h(c, xs, i, n):\n{}{local}    {value}\n",
            start.locals()
        ));
    }
    let mut expected = Found::new();
    for (position, array) in sites.iter().enumerate() {
        let gain = gain_of(start.chosen(*array, index)) + i32::from(wrong);
        let elements = array.join(", ");
        let call = if start.self_reading() {
            format!("sf([{elements}], {index}, 3).gain")
        } else {
            format!("h({}, [{elements}], {index}, 3)", beside(*array))
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
    let mut failures = Vec::new();
    let mut programs = 0;
    for start in CALL_STARTS {
        let helpers: &[PathHelper] = if start.self_reading() {
            &[PathHelper::Element]
        } else {
            &PATH_HELPERS
        };
        for helper in helpers.iter().copied() {
            for consumer in START_CONSUMERS {
                for sites in &sets {
                    for index in [0, 1] {
                        for wrong in [false, true] {
                            for reversed in [false, true] {
                                let program = |recursive| {
                                    call_start_program(
                                        (helper, start, consumer),
                                        (sites, index, wrong),
                                        (reversed, recursive),
                                    )
                                };
                                let (text, expected) = program(true);
                                let twin = (!start.self_reading()).then(|| program(false).0);
                                programs += 1;
                                if let Some(failure) =
                                    model_failure(&text, twin.as_deref(), &expected)
                                {
                                    failures.push(format!(
                                        "{start:?} {helper:?} {consumer:?} {sites:?} {index} {wrong} {reversed}\n{failure}"
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
    assert_eq!(programs, 4_928);
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
    index: usize,
    (reversed, recursive): (bool, bool),
) -> String {
    let chosen = chooser.expression(open.text(), known);
    let local = consumer.local(&chosen);
    let read = consumer.read(&chosen);
    let value = if recursive {
        format!("if n < 1 then {read} else h(c, i, n - 1)")
    } else {
        read
    };
    let helper = if recursive {
        "fn pickr(c, d, p, m): if m < 1 then (if p > 0 then d else c) else pickr(c, d, p, m - 1)\n"
    } else {
        "fn pickr(c, d, p, m): if p > 0 then d else c\n"
    };
    let instance = if chooser.takes_open(index) {
        open.chosen(index)
    } else if known == "c" {
        "tail"
    } else {
        known
    };
    let mut items = trio_items();
    items.extend([
        helper.to_string(),
        "let rp = pickr(verse, other, 1, 2)\n".to_string(),
        "fn f(c): c.gain\n".to_string(),
        "fn g(c, n): if n < 1 then c.gain else g(c, n - 1)\n".to_string(),
        format!(
            "fn h(c, i, n):\n{}{local}    {value}\n",
            chooser.locals(open.text(), known)
        ),
        format!(
            "Clip k:\n    Note/([60][i32(h(tail, {index}, 3)) - {}])\n",
            gain_of(instance)
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
                    for index in [0, 1] {
                        for reversed in [false, true] {
                            let case = (chooser, open, known, consumer);
                            let text = open_choice_program(case, index, (reversed, true));
                            let twin = open_choice_program(case, index, (reversed, false));
                            programs += 1;
                            if let Some(failure) = model_failure(&text, Some(&twin), &Found::new())
                            {
                                failures.push(format!(
                                    "{chooser:?} {open:?} {known} {consumer:?} {index} {reversed}\n{failure}"
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
    assert_eq!(programs, 896);
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
    (index, top_level, reversed, recursive): (usize, bool, bool, bool),
) -> String {
    let reference = held.reference();
    let read = consumer.read(reference);
    let value = if recursive {
        format!("if n < 1 then {read} else h(c, i, n - 1)")
    } else {
        read
    };
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
        format!("fn h(c, i, n):\n{}    {value}\n", consumer.local(reference)),
        format!(
            "Clip k:\n    Note/([60][i32(h(verse, {index}, 3)) - {}])\n",
            held.chosen_gain(index)
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
            for index in [0, 1] {
                for top_level in [false, true] {
                    for reversed in [false, true] {
                        let program = |recursive| {
                            held_program((held, consumer), (index, top_level, reversed, recursive))
                        };
                        let (text, twin) = (program(true), program(false));
                        programs += 1;
                        if let Some(failure) = model_failure(&text, Some(&twin), &Found::new()) {
                            failures.push(format!(
                                "{held:?} {consumer:?} {index} {top_level} {reversed}\n{failure}"
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
    assert_eq!(programs, 240);
}
