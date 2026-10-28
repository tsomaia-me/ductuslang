mod support;

use std::collections::BTreeMap;

use symphonyscript::Code;

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
