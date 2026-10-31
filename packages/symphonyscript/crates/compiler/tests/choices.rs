mod support;

use std::collections::{BTreeSet, HashMap};

use symphonyscript::{Code, Compiler, Source};
use symphonyscript_fixtures::{Rows, covered};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum Element {
    Int,
    Float,
    WidenedInt,
    Bool,
    Wave,
    Text,
    Bars,
    Ms,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum Content {
    Scalar(Element),
    Fixed {
        element: Element,
        length: usize,
        nested: bool,
    },
    Variable(Element),
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum Form {
    Integer,
    Float,
    Bool,
    Variant,
    Text,
    Single,
    Pair,
    Nested,
    VariableInput,
    State,
    TopLet,
    LocalLet,
    Inlined,
    Recursive,
    Choice,
}

const FORMS: [Form; 15] = [
    Form::Integer,
    Form::Float,
    Form::Bool,
    Form::Variant,
    Form::Text,
    Form::Single,
    Form::Pair,
    Form::Nested,
    Form::VariableInput,
    Form::State,
    Form::TopLet,
    Form::LocalLet,
    Form::Inlined,
    Form::Recursive,
    Form::Choice,
];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Relation {
    OneType,
    Widening,
    Family,
    ScalarAndArray,
    Dimension,
    NoWidening,
    Lengths,
}

impl Relation {
    fn agrees(self) -> bool {
        match self {
            Relation::OneType | Relation::Widening | Relation::Lengths => true,
            Relation::Family
            | Relation::ScalarAndArray
            | Relation::Dimension
            | Relation::NoWidening => false,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Chooser {
    True,
    False,
    Flag,
    Enum,
    EnumInput,
    Integer,
    Parameter,
    ParameterMatch,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Position {
    TopLet,
    LocalLet,
    Scalar,
    IndexBase,
    JoinLeft,
    JoinRight,
    SliceBase,
    Len,
    Iterable,
    Attribute,
    Native,
    Argument,
    Result,
    RecursiveArgument,
    RecursiveResult,
    StateInit,
    OuterArm,
}

const POSITIONS: [Position; 17] = [
    Position::TopLet,
    Position::LocalLet,
    Position::Scalar,
    Position::IndexBase,
    Position::JoinLeft,
    Position::JoinRight,
    Position::SliceBase,
    Position::Len,
    Position::Iterable,
    Position::Attribute,
    Position::Native,
    Position::Argument,
    Position::Result,
    Position::RecursiveArgument,
    Position::RecursiveResult,
    Position::StateInit,
    Position::OuterArm,
];

const fn fixed(element: Element, length: usize) -> Content {
    Content::Fixed {
        element,
        length,
        nested: false,
    }
}

const fn nested(element: Element) -> Content {
    Content::Fixed {
        element,
        length: 1,
        nested: true,
    }
}

fn pairs(relation: Relation) -> Vec<(Content, Content)> {
    use Element::{Bars, Bool, Float, Int, Ms, Text, Wave};
    let scalar = Content::Scalar;
    let one_way: Vec<(Content, Content)> = match relation {
        Relation::OneType => {
            return vec![
                (scalar(Int), scalar(Int)),
                (fixed(Int, 1), fixed(Int, 1)),
                (fixed(Int, 2), fixed(Int, 2)),
                (Content::Variable(Int), Content::Variable(Int)),
            ];
        }
        Relation::Widening => vec![
            (scalar(Int), scalar(Float)),
            (fixed(Int, 1), fixed(Float, 1)),
            (Content::Variable(Int), Content::Variable(Float)),
        ],
        Relation::Family => vec![(scalar(Bars), scalar(Ms)), (fixed(Bars, 1), fixed(Ms, 1))],
        Relation::ScalarAndArray => vec![
            (scalar(Int), fixed(Int, 1)),
            (scalar(Int), Content::Variable(Int)),
        ],
        Relation::Dimension => vec![
            (fixed(Int, 1), nested(Int)),
            (fixed(Int, 2), nested(Int)),
            (Content::Variable(Int), nested(Int)),
        ],
        Relation::NoWidening => vec![
            (scalar(Int), scalar(Bool)),
            (scalar(Int), scalar(Wave)),
            (scalar(Int), scalar(Text)),
            (fixed(Int, 1), fixed(Bool, 1)),
            (Content::Variable(Int), Content::Variable(Bool)),
        ],
        Relation::Lengths => vec![
            (fixed(Int, 1), fixed(Int, 2)),
            (fixed(Int, 1), Content::Variable(Int)),
            (fixed(Int, 2), Content::Variable(Int)),
        ],
    };
    one_way
        .iter()
        .flat_map(|(first, second)| [(*first, *second), (*second, *first)])
        .collect()
}

fn element_literal(element: Element) -> &'static str {
    match element {
        Element::Int => "1",
        Element::Float => "1.5",
        Element::WidenedInt => "1.0",
        Element::Bool => "true",
        Element::Wave => "Wave::Saw",
        Element::Text => "\"a\"",
        Element::Bars => "1bars",
        Element::Ms => "5ms",
    }
}

fn element_type(element: Element) -> Option<&'static str> {
    match element {
        Element::Int => Some("i32"),
        Element::Float | Element::WidenedInt => Some("f64"),
        Element::Bool => Some("bool"),
        Element::Wave | Element::Text | Element::Bars | Element::Ms => None,
    }
}

fn element_zero(element: Element) -> &'static str {
    match element {
        Element::Int => "0",
        Element::Float | Element::WidenedInt => "0.5",
        Element::Bool => "false",
        Element::Wave => "Wave::Saw",
        Element::Text => "\"a\"",
        Element::Bars => "1bars",
        Element::Ms => "5ms",
    }
}

impl Content {
    fn literal(self) -> Option<String> {
        match self {
            Content::Scalar(element) => Some(element_literal(element).to_string()),
            Content::Fixed {
                element,
                length,
                nested,
            } => {
                let row = vec![element_literal(element); length].join(", ");
                Some(if nested {
                    format!("[[{row}]]")
                } else {
                    format!("[{row}]")
                })
            }
            Content::Variable(_) => None,
        }
    }

    fn element(self) -> Element {
        match self {
            Content::Scalar(element)
            | Content::Fixed { element, .. }
            | Content::Variable(element) => element,
        }
    }

    fn dimensions(self) -> usize {
        match self {
            Content::Scalar(_) => 0,
            Content::Fixed { nested: true, .. } => 2,
            Content::Fixed { nested: false, .. } | Content::Variable(_) => 1,
        }
    }

    fn with_element(self, element: Element) -> Content {
        match self {
            Content::Scalar(_) => Content::Scalar(element),
            Content::Fixed { length, nested, .. } => Content::Fixed {
                element,
                length,
                nested,
            },
            Content::Variable(_) => Content::Variable(element),
        }
    }
}

struct Arm {
    text: String,
    items: Vec<String>,
    locals: Vec<String>,
}

fn variable_input(name: &str, element: Element) -> Option<String> {
    Some(format!(
        "state {name}([]): {}[..4]\n",
        element_type(element)?
    ))
}

fn arm(form: Form, content: Content, index: usize, hosted: bool) -> Option<Arm> {
    let plain = |text: String| Arm {
        text,
        items: Vec::new(),
        locals: Vec::new(),
    };
    let cell = format!("w{index}");
    let held = |content: Content| -> Option<(String, Vec<String>)> {
        match content {
            Content::Variable(element) => {
                Some((cell.clone(), vec![variable_input(&cell, element)?]))
            }
            Content::Scalar(_) | Content::Fixed { .. } => Some((content.literal()?, Vec::new())),
        }
    };
    let is_array = content.dimensions() > 0;
    match (form, content) {
        (Form::Integer, Content::Scalar(Element::Int | Element::Bars))
        | (Form::Float, Content::Scalar(Element::Float | Element::WidenedInt | Element::Ms))
        | (Form::Bool, Content::Scalar(Element::Bool))
        | (Form::Variant, Content::Scalar(Element::Wave))
        | (Form::Text, Content::Scalar(Element::Text))
        | (
            Form::Single,
            Content::Fixed {
                length: 1,
                nested: false,
                ..
            },
        )
        | (
            Form::Pair,
            Content::Fixed {
                length: 2,
                nested: false,
                ..
            },
        )
        | (Form::Nested, Content::Fixed { nested: true, .. }) => Some(plain(content.literal()?)),
        (Form::VariableInput, Content::Variable(element)) => {
            let name = format!("v{index}");
            Some(Arm {
                items: vec![variable_input(&name, element)?],
                text: name,
                locals: Vec::new(),
            })
        }
        (Form::State, Content::Fixed { .. }) => {
            let name = format!("s{index}");
            Some(Arm {
                items: vec![format!("state {name} = {}\n", content.literal()?)],
                text: name,
                locals: Vec::new(),
            })
        }
        (Form::State, Content::Variable(element)) => {
            let name = format!("s{index}");
            let literal = element_literal(element);
            Some(Arm {
                items: vec![format!(
                    "state {name} = ({name}.past(1, []) + [{literal}])[^4..]\n"
                )],
                text: name,
                locals: Vec::new(),
            })
        }
        (Form::TopLet, _) if is_array => {
            let name = format!("l{index}");
            let (value, mut items) = held(content)?;
            items.push(format!("let {name} = {value}\n"));
            Some(Arm {
                text: name,
                items,
                locals: Vec::new(),
            })
        }
        (Form::LocalLet, _) if is_array && hosted => {
            let name = format!("m{index}");
            let (value, items) = held(content)?;
            Some(Arm {
                text: name.clone(),
                items,
                locals: vec![format!("    let {name} = {value}\n")],
            })
        }
        (Form::Inlined, _) if is_array => {
            let name = format!("i{index}");
            let (value, mut items) = held(content)?;
            items.push(format!("fn {name}(z): {value}\n"));
            Some(Arm {
                text: format!("{name}(0)"),
                items,
                locals: Vec::new(),
            })
        }
        (Form::Recursive, Content::Fixed { .. }) => {
            let name = format!("r{index}");
            Some(Arm {
                text: format!("{name}(1)"),
                items: vec![format!(
                    "fn {name}(n): if n > 0 then {name}(n - 1) else {}\n",
                    content.literal()?
                )],
                locals: Vec::new(),
            })
        }
        (Form::Choice, _) => {
            let gate = format!("n{index}");
            let (value, mut items) = held(content)?;
            items.push(format!("state {gate}(false)\n"));
            Some(Arm {
                text: format!("(if {gate} then {value} else {value})"),
                items,
                locals: Vec::new(),
            })
        }
        (
            Form::Integer
            | Form::Float
            | Form::Bool
            | Form::Variant
            | Form::Text
            | Form::Single
            | Form::Pair
            | Form::Nested
            | Form::VariableInput
            | Form::State
            | Form::TopLet
            | Form::LocalLet
            | Form::Inlined
            | Form::Recursive,
            _,
        ) => None,
    }
}

impl Chooser {
    fn items(self) -> Vec<String> {
        let item = match self {
            Chooser::True | Chooser::False | Chooser::Parameter | Chooser::ParameterMatch => {
                return Vec::new();
            }
            Chooser::Flag => "state flag(false)\n",
            Chooser::Enum => "let w = Wave::Saw\n",
            Chooser::EnumInput => "state w(Wave::Saw)\n",
            Chooser::Integer => "state k(1)\n",
        };
        vec![item.to_string()]
    }

    fn taken(self) -> Option<usize> {
        match self {
            Chooser::True | Chooser::Enum | Chooser::Parameter | Chooser::ParameterMatch => Some(0),
            Chooser::False => Some(1),
            Chooser::Flag | Chooser::EnumInput | Chooser::Integer => None,
        }
    }

    fn is_match(self) -> bool {
        match self {
            Chooser::Enum | Chooser::EnumInput | Chooser::Integer | Chooser::ParameterMatch => true,
            Chooser::True | Chooser::False | Chooser::Flag | Chooser::Parameter => false,
        }
    }

    fn is_parameter(self) -> bool {
        match self {
            Chooser::Parameter | Chooser::ParameterMatch => true,
            Chooser::True
            | Chooser::False
            | Chooser::Flag
            | Chooser::Enum
            | Chooser::EnumInput
            | Chooser::Integer => false,
        }
    }

    fn render(self, first: &str, second: &str, indent: &str) -> String {
        match self {
            Chooser::True => format!("if true then {first} else {second}"),
            Chooser::False => format!("if false then {first} else {second}"),
            Chooser::Flag => format!("if flag then {first} else {second}"),
            Chooser::Enum | Chooser::EnumInput => {
                format!("match w:\n{indent}    Saw: {first}\n{indent}    else: {second}\n")
            }
            Chooser::Integer => {
                format!("match k:\n{indent}    1: {first}\n{indent}    else: {second}\n")
            }
            Chooser::Parameter => format!("if m is 0 then {first} else {second}"),
            Chooser::ParameterMatch => {
                format!("match m:\n{indent}    0: {first}\n{indent}    else: {second}\n")
            }
        }
    }
}

const SLOT: &str = "\u{1}";

fn read(value: &str, dimensions: usize) -> String {
    if dimensions == 0 {
        return format!("Clip c:\n    Note/({value})\n");
    }
    format!(
        "Clip c:\n    Note/(({value}){})\n",
        "[0]".repeat(dimensions)
    )
}

struct Layout {
    items: Vec<String>,
    bound: Option<&'static str>,
    hosted: bool,
    signature: bool,
    read_in_body: bool,
}

impl Position {
    fn signature(self) -> bool {
        match self {
            Position::Attribute | Position::Native => true,
            Position::TopLet
            | Position::LocalLet
            | Position::Scalar
            | Position::IndexBase
            | Position::JoinLeft
            | Position::JoinRight
            | Position::SliceBase
            | Position::Len
            | Position::Iterable
            | Position::Argument
            | Position::Result
            | Position::RecursiveArgument
            | Position::RecursiveResult
            | Position::StateInit
            | Position::OuterArm => false,
        }
    }

    fn read_in_body(self) -> bool {
        match self {
            Position::Scalar
            | Position::IndexBase
            | Position::JoinLeft
            | Position::JoinRight
            | Position::SliceBase
            | Position::Len
            | Position::Native
            | Position::Argument
            | Position::RecursiveArgument => true,
            Position::TopLet
            | Position::LocalLet
            | Position::Iterable
            | Position::Attribute
            | Position::Result
            | Position::RecursiveResult
            | Position::StateInit
            | Position::OuterArm => false,
        }
    }

    fn hosted(self) -> bool {
        match self {
            Position::LocalLet | Position::Result | Position::RecursiveResult => true,
            Position::TopLet
            | Position::Scalar
            | Position::IndexBase
            | Position::JoinLeft
            | Position::JoinRight
            | Position::SliceBase
            | Position::Len
            | Position::Iterable
            | Position::Attribute
            | Position::Native
            | Position::Argument
            | Position::RecursiveArgument
            | Position::StateInit
            | Position::OuterArm => false,
        }
    }

    fn layout(self, dimensions: usize, other: &str) -> Layout {
        let at_least_one = dimensions.max(1);
        let partner = if dimensions == 2 { "[[9]]" } else { "[9]" };
        let element = "[0]".repeat(dimensions.saturating_sub(1));
        let (items, bound, signature) = match self {
            Position::TopLet => (
                vec![format!("let x = {SLOT}\n"), read("x", dimensions)],
                Some(""),
                false,
            ),
            Position::LocalLet => (
                vec![
                    format!("fn h(z):\n\u{2}    let x = {SLOT}\n    x\n"),
                    read("h(0)", dimensions),
                ],
                Some("    "),
                false,
            ),
            Position::Scalar => (vec![format!("Clip c:\n    Note/({SLOT})\n")], None, false),
            Position::IndexBase => (vec![read(SLOT, at_least_one)], None, false),
            Position::JoinLeft => (
                vec![read(&format!("({SLOT}) + {partner}"), at_least_one)],
                None,
                false,
            ),
            Position::JoinRight => (
                vec![read(&format!("{partner} + ({SLOT})"), at_least_one)],
                None,
                false,
            ),
            Position::SliceBase => (
                vec![read(&format!("({SLOT})[0..1]"), at_least_one)],
                None,
                false,
            ),
            Position::Len => (
                vec![format!("Clip c:\n    Note/(({SLOT}).size)\n")],
                None,
                false,
            ),
            Position::Iterable => (
                vec![format!(
                    "Clip c:\n    for e in ({SLOT}):\n        Note/(e{element})\n"
                )],
                None,
                false,
            ),
            Position::Attribute => (
                vec![format!("Clip c | three=({SLOT}):\n    Note\n")],
                None,
                true,
            ),
            Position::Native => (
                vec![format!("Clip c:\n    Note/(cap4({SLOT}))\n")],
                None,
                true,
            ),
            Position::Argument => (
                vec![
                    "fn pass(v): v\n".to_string(),
                    read(&format!("pass({SLOT})"), dimensions),
                ],
                None,
                false,
            ),
            Position::Result => (
                vec![
                    format!("fn g(z):\n\u{2}    {SLOT}\n"),
                    read("g(0)", dimensions),
                ],
                Some("    "),
                false,
            ),
            Position::RecursiveArgument => (
                vec![
                    "fn rr(v, n): if n > 0 then rr(v, n - 1) else v\n".to_string(),
                    read(&format!("rr({SLOT}, 1)"), dimensions),
                ],
                None,
                false,
            ),
            Position::RecursiveResult => (
                vec![
                    format!("fn rr(n):\n\u{2}    if n > 0 then rr(n - 1) else ({SLOT})\n"),
                    read("rr(1)", dimensions),
                ],
                None,
                false,
            ),
            Position::StateInit => (
                vec![format!("state st = {SLOT}\n"), read("st", dimensions)],
                Some(""),
                false,
            ),
            Position::OuterArm => (
                vec![
                    "state gate(false)\n".to_string(),
                    read(&format!("if gate then ({SLOT}) else ({other})"), dimensions),
                ],
                None,
                false,
            ),
        };
        Layout {
            items,
            bound,
            read_in_body: self.read_in_body(),
            hosted: self.hosted(),
            signature,
        }
    }
}

struct Case {
    chooser: Chooser,
    relation: Relation,
    forms: (Form, Form),
    pair: usize,
    contents: (Content, Content),
    position: Position,
    reversed: bool,
}

struct Built {
    text: String,
    start: usize,
    end: usize,
}

const EDGE: char = '\u{3}';

fn assemble(
    layout: &Layout,
    chooser: Chooser,
    declarations: &[String],
    locals: &[String],
    value: &str,
    reversed: bool,
) -> Built {
    let marked = format!("{EDGE}{value}{EDGE}");
    let mut lets = locals.concat();
    let mut extra = Vec::new();
    let in_slot = match (chooser.is_match(), layout.bound) {
        _ if chooser.is_parameter() => {
            let end = if value.ends_with('\n') { "" } else { "\n" };
            let read = if layout.read_in_body {
                body_read(&layout.items)
            } else {
                "ch".to_string()
            };
            extra.push(format!(
                "fn rc(m):\n{lets}    let ch = {marked}{end}    if m > 0 then rc(m - 1) else {read}\n"
            ));
            lets.clear();
            "rc(0)".to_string()
        }
        (true, None) if layout.hosted => {
            lets.push_str(&format!("    let ch = {marked}"));
            "ch".to_string()
        }
        (true, None) => {
            extra.push(format!("let ch = {marked}"));
            "ch".to_string()
        }
        (true, Some(_)) | (false, _) => marked,
    };
    let slot = if in_slot.ends_with(&format!("\n{EDGE}")) {
        format!("{SLOT}\n")
    } else {
        SLOT.to_string()
    };
    let mut items: Vec<String> = declarations.to_vec();
    items.extend(extra);
    for item in &layout.items {
        let item = if chooser.is_parameter() && layout.read_in_body && item.contains(SLOT) {
            format!("{READER}{SLOT}{READ_END}")
        } else {
            item.clone()
        };
        items.push(item.replace('\u{2}', &lets).replacen(&slot, &in_slot, 1));
    }
    if reversed {
        items.reverse();
    }
    let joined = items.concat();
    let start = joined.find(EDGE).unwrap_or(0);
    let end = joined.rfind(EDGE).unwrap_or(0) - EDGE.len_utf8();
    Built {
        text: joined.replace(EDGE, ""),
        start,
        end,
    }
}

const READER: &str = "Clip c:\n    Note/(";

const READ_END: &str = ")\n";

fn body_read(items: &[String]) -> String {
    items
        .iter()
        .find_map(|item| {
            item.strip_prefix(READER)?
                .strip_suffix(READ_END)
                .filter(|read| read.contains(SLOT))
        })
        .expect("a position read in the body reads its slot in a note's pitch")
        .replace(SLOT, "ch")
}

type Spans = BTreeSet<(Code, usize, usize)>;

fn spans(compiler: &Compiler, text: &str) -> Spans {
    support::errors(compiler.check(&[Source {
        path: "main.sym".into(),
        text: text.to_string(),
    }]))
    .into_iter()
    .map(|diagnostic| {
        (
            diagnostic.code,
            usize::try_from(diagnostic.span.range.start).unwrap_or(usize::MAX),
            usize::try_from(diagnostic.span.range.end).unwrap_or(usize::MAX),
        )
    })
    .collect()
}

fn capacity(content: Content) -> usize {
    match content {
        Content::Fixed { length, .. } => length,
        Content::Scalar(_) | Content::Variable(_) => 4,
    }
}

fn unified(relation: Relation, content: Content) -> Content {
    match relation {
        Relation::Widening if content.element() == Element::Int => {
            content.with_element(Element::WidenedInt)
        }
        Relation::Widening => content,
        Relation::OneType
        | Relation::Lengths
        | Relation::Family
        | Relation::ScalarAndArray
        | Relation::Dimension
        | Relation::NoWidening => content,
    }
}

fn value_twin(relation: Relation, contents: (Content, Content)) -> Arm {
    let (first, second) = (unified(relation, contents.0), unified(relation, contents.1));
    let element = first.element();
    let ty = element_type(element).expect("the arms of an agreeing pair hold numbers");
    let scalar = format!("state tw({}): {ty}\n", element_zero(element));
    let (text, declared) = match (first, second) {
        (Content::Scalar(_), _) => ("(tw * 1)".to_string(), scalar),
        (
            Content::Fixed { length, nested, .. },
            Content::Fixed {
                length: other,
                nested: other_nested,
                ..
            },
        ) if length == other && nested == other_nested => {
            let row = vec!["tw * 1"; length].join(", ");
            let text = if nested {
                format!("[[{row}]]")
            } else {
                format!("[{row}]")
            };
            (text, scalar)
        }
        (Content::Fixed { .. } | Content::Variable(_), _) => (
            "(tw + [])".to_string(),
            format!(
                "state tw([]): {ty}[..{}]\n",
                capacity(first).max(capacity(second))
            ),
        ),
    };
    Arm {
        text,
        items: vec![declared],
        locals: Vec::new(),
    }
}

fn taken_twin(relation: Relation, form: Form, content: Content, hosted: bool) -> Arm {
    if relation == Relation::Widening && reads_cell(form, content) {
        return value_twin(relation, (content, content));
    }
    let unified = unified(relation, content);
    let form = if form == Form::Integer && unified == Content::Scalar(Element::WidenedInt) {
        Form::Float
    } else {
        form
    };
    arm(form, unified, 2, hosted).expect("the taken arm's form holds the unified type")
}

fn reads_cell(form: Form, content: Content) -> bool {
    let held_cell = match content {
        Content::Variable(_) => true,
        Content::Scalar(_) | Content::Fixed { .. } => false,
    };
    held_cell
        || match form {
            Form::VariableInput | Form::State | Form::Choice => true,
            Form::Integer
            | Form::Float
            | Form::Bool
            | Form::Variant
            | Form::Text
            | Form::Single
            | Form::Pair
            | Form::Nested
            | Form::TopLet
            | Form::LocalLet
            | Form::Inlined
            | Form::Recursive => false,
        }
}

#[derive(Default)]
struct Outcome {
    programs: usize,
    landing_apart: usize,
    failures: Vec<String>,
}

struct Compilers {
    fixture: Compiler,
    signature: Compiler,
    arms: HashMap<(Form, Content, usize, bool), Option<Arm>>,
    twins: HashMap<String, Spans>,
}

fn arms_hold(rows: &Rows, chooser: Chooser) -> Outcome {
    let mut compilers = Compilers {
        fixture: Compiler::new(&symphonyscript_fixtures::vocabulary())
            .expect("the vocabulary compiles"),
        signature: Compiler::new(&support::signature_vocabulary())
            .expect("the vocabulary compiles"),
        arms: HashMap::new(),
        twins: HashMap::new(),
    };
    let mut outcome = Outcome::default();
    let relations = [
        Relation::OneType,
        Relation::Widening,
        Relation::Family,
        Relation::ScalarAndArray,
        Relation::Dimension,
        Relation::NoWidening,
        Relation::Lengths,
    ];
    for relation in relations {
        for (pair, contents) in pairs(relation).into_iter().enumerate() {
            for first_form in FORMS {
                for second_form in FORMS {
                    for position in POSITIONS {
                        for reversed in [false, true] {
                            let case = Case {
                                chooser,
                                relation,
                                forms: (first_form, second_form),
                                pair,
                                contents,
                                position,
                                reversed,
                            };
                            check_case(rows, &case, &mut compilers, &mut outcome);
                        }
                    }
                }
            }
        }
    }
    outcome
}

fn check_case(rows: &Rows, case: &Case, compilers: &mut Compilers, outcome: &mut Outcome) {
    let hosted = case.position.hosted() || case.chooser.is_parameter();
    let keys = [
        (case.forms.0, case.contents.0, 0, hosted),
        (case.forms.1, case.contents.1, 1, hosted),
    ];
    for key in keys {
        compilers
            .arms
            .entry(key)
            .or_insert_with(|| arm(key.0, key.1, key.2, hosted));
    }
    let (Some(Some(first)), Some(Some(second))) =
        (compilers.arms.get(&keys[0]), compilers.arms.get(&keys[1]))
    else {
        return;
    };
    if case.position.signature() && case.relation == Relation::Family {
        return;
    }
    if case.chooser.is_parameter()
        && (reads_cell(case.forms.0, case.contents.0) || reads_cell(case.forms.1, case.contents.1))
    {
        return;
    }
    let taken = case.chooser.taken();
    let taken_content = match taken {
        Some(1) => case.contents.1,
        Some(_) | None => case.contents.0,
    };
    let literal = if case.chooser.is_parameter() {
        let Some(literal) = unified(case.relation, taken_content).literal() else {
            return;
        };
        Some(literal)
    } else {
        None
    };
    let tag = [
        case.chooser as usize,
        case.relation as usize,
        case.pair,
        case.forms.0 as usize,
        case.forms.1 as usize,
        case.position as usize,
        usize::from(case.reversed),
    ];
    if !rows.take(&tag) {
        return;
    }
    let body_reads_cell = taken.is_none()
        || reads_cell(case.forms.0, case.contents.0)
        || reads_cell(case.forms.1, case.contents.1);
    let rejected_call = case.position == Position::RecursiveResult && body_reads_cell;
    let twin = match (case.relation.agrees() && !rejected_call, taken) {
        (false, _) => None,
        (true, Some(index)) => {
            let form = if index == 0 {
                case.forms.0
            } else {
                case.forms.1
            };
            Some(taken_twin(case.relation, form, taken_content, hosted))
        }
        (true, None) => Some(value_twin(case.relation, case.contents)),
    };
    let other = literal.unwrap_or_else(|| twin.as_ref().unwrap_or(first).text.clone());
    let layout = case.position.layout(taken_content.dimensions(), &other);
    let compiler = if layout.signature {
        &compilers.signature
    } else {
        &compilers.fixture
    };
    let mut declarations = case.chooser.items();
    let mut locals = Vec::new();
    for part in [Some(first), Some(second), twin.as_ref()]
        .into_iter()
        .flatten()
    {
        declarations.extend(part.items.iter().cloned());
        locals.extend(part.locals.iter().cloned());
    }
    let indent = if layout.hosted || case.chooser.is_parameter() {
        "    "
    } else {
        ""
    };
    let choice = case.chooser.render(&first.text, &second.text, indent);
    let build = |value: &str| {
        assemble(
            &layout,
            case.chooser,
            &declarations,
            &locals,
            value,
            case.reversed,
        )
    };
    let built = build(&choice);
    let found = spans(compiler, &built.text);
    outcome.programs += 1;
    let expected: Spans = match &twin {
        None if rejected_call => {
            let call = built.text.find("rr(1)").unwrap_or(0);
            [(Code::ReactiveRecursion, call, call + "rr(1)".len())].into()
        }
        None if case.relation == Relation::Family => {
            [(Code::MixedFamilies, built.start, built.end)].into()
        }
        None => [(Code::TypeMismatch, built.start, built.end)].into(),
        Some(twin) => {
            let value = if case.chooser.is_match() {
                format!("{}\n", twin.text)
            } else {
                twin.text.clone()
            };
            let twin_built = build(&value);
            let twin_found = compilers
                .twins
                .entry(twin_built.text.clone())
                .or_insert_with(|| spans(compiler, &twin_built.text));
            let arm_start = taken
                .filter(|_| case.relation != Relation::Widening)
                .map(|index| {
                    let offset = if index == 0 {
                        choice.find(&first.text)
                    } else {
                        choice.rfind(&second.text)
                    };
                    built.start + offset.unwrap_or(0)
                });
            mapped(twin_found, &twin_built, &built, arm_start)
        }
    };
    if found == expected {
        return;
    }
    if taken.is_some() && case.relation == Relation::Widening {
        let (start, end) = landing(case, taken_content.dimensions(), &built);
        let at_landing: Spans = codes(&expected)
            .into_iter()
            .map(|code| (code, start, end))
            .collect();
        if found == at_landing {
            outcome.landing_apart += 1;
            return;
        }
    }
    {
        outcome.failures.push(format!(
            "{:?} {:?} {:?} {:?}\n{}  found {}\n  expected {}",
            case.relation,
            case.forms,
            case.position,
            case.reversed,
            built.text,
            shown(&found, &built.text),
            shown(&expected, &built.text)
        ));
    }
}

fn landing(case: &Case, dimensions: usize, built: &Built) -> (usize, usize) {
    let indexes = "[0]".len() * dimensions.max(1);
    let named = |name: &str| {
        let text = format!("({name})");
        let start = built
            .text
            .find(&format!("Note/({text}"))
            .map_or(0, |at| at + "Note/(".len());
        (start, start + text.len() + indexes)
    };
    match (case.position, case.chooser.is_match()) {
        (Position::TopLet, _) => named("x"),
        (Position::IndexBase, true) => named("ch"),
        (Position::IndexBase, false) => (built.start - 1, built.end + 1 + indexes),
        (Position::Attribute, true) => {
            let start = built
                .text
                .find("three=(ch)")
                .map_or(0, |at| at + "three=(".len());
            (start, start + "ch".len())
        }
        (
            Position::LocalLet
            | Position::Scalar
            | Position::JoinLeft
            | Position::JoinRight
            | Position::SliceBase
            | Position::Len
            | Position::Iterable
            | Position::Attribute
            | Position::Native
            | Position::Argument
            | Position::Result
            | Position::RecursiveArgument
            | Position::RecursiveResult
            | Position::StateInit
            | Position::OuterArm,
            _,
        ) => (built.start, built.end),
    }
}

fn codes(spans: &Spans) -> Vec<Code> {
    spans.iter().map(|(code, ..)| *code).collect()
}

fn bare(text: &str) -> &str {
    let trimmed = text.trim_end();
    trimmed
        .strip_prefix('(')
        .and_then(|inner| inner.strip_suffix(')'))
        .unwrap_or(trimmed)
}

fn mapped(twin_found: &Spans, twin: &Built, built: &Built, arm_start: Option<usize>) -> Spans {
    let shift = |offset: usize| offset + built.end - twin.end;
    let whole = bare(&twin.text[twin.start..twin.end]);
    twin_found
        .iter()
        .map(|(code, start, end)| {
            let (start, end) = (*start, *end);
            let span = if end <= twin.start {
                (start, end)
            } else if start >= twin.end {
                (shift(start), shift(end))
            } else if start <= twin.start && end >= twin.end {
                (start, shift(end))
            } else if bare(&twin.text[start..end]) == whole {
                (built.start, built.end)
            } else if let Some(arm) = arm_start {
                (arm + start - twin.start, arm + end - twin.start)
            } else {
                (built.start, built.end)
            };
            (*code, span.0, span.1)
        })
        .collect()
}

fn shown(found: &Spans, text: &str) -> String {
    found
        .iter()
        .map(|(code, start, end)| format!("{code:?} {:?}", text.get(*start..*end).unwrap_or("?")))
        .collect::<Vec<_>>()
        .join(", ")
}

fn assert_outcome(outcome: &Outcome, expected: (usize, usize)) {
    assert!(
        outcome.failures.is_empty(),
        "{} of {} programs fail:\n{}",
        outcome.failures.len(),
        outcome.programs,
        outcome
            .failures
            .iter()
            .take(40)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
    assert_eq!((outcome.programs, outcome.landing_apart), expected);
}

fn assert_arms_hold(chooser: Chooser, programs: usize, landing_apart: usize) {
    assert_outcome(&arms_hold(&Rows::Every, chooser), (programs, landing_apart));
}

fn assert_covered_arms_hold(chooser: Chooser, programs: usize, landing_apart: usize) {
    assert_outcome(
        &covered(|rows| arms_hold(rows, chooser)),
        (programs, landing_apart),
    );
}
#[test]
fn the_arms_of_a_compile_time_true_choice_agree_wherever_it_stands() {
    assert_covered_arms_hold(Chooser::True, 344, 0);
}

#[test]
#[ignore = "exhaustive"]
fn exhaustive_the_arms_of_a_compile_time_true_choice_agree_wherever_it_stands() {
    assert_arms_hold(Chooser::True, 32080, 72);
}

#[test]
fn the_arms_of_a_compile_time_false_choice_agree_wherever_it_stands() {
    assert_covered_arms_hold(Chooser::False, 344, 1);
}

#[test]
#[ignore = "exhaustive"]
fn exhaustive_the_arms_of_a_compile_time_false_choice_agree_wherever_it_stands() {
    assert_arms_hold(Chooser::False, 32080, 72);
}

#[test]
fn the_arms_of_a_runtime_choice_agree_wherever_it_stands() {
    assert_covered_arms_hold(Chooser::Flag, 344, 0);
}

#[test]
#[ignore = "exhaustive"]
fn exhaustive_the_arms_of_a_runtime_choice_agree_wherever_it_stands() {
    assert_arms_hold(Chooser::Flag, 32080, 0);
}

#[test]
fn the_arms_of_a_compile_time_enum_match_agree_wherever_it_stands() {
    assert_covered_arms_hold(Chooser::Enum, 344, 0);
}

#[test]
#[ignore = "exhaustive"]
fn exhaustive_the_arms_of_a_compile_time_enum_match_agree_wherever_it_stands() {
    assert_arms_hold(Chooser::Enum, 32080, 48);
}

#[test]
fn the_arms_of_a_match_on_an_enum_input_agree_wherever_it_stands() {
    assert_covered_arms_hold(Chooser::EnumInput, 344, 0);
}

#[test]
#[ignore = "exhaustive"]
fn exhaustive_the_arms_of_a_match_on_an_enum_input_agree_wherever_it_stands() {
    assert_arms_hold(Chooser::EnumInput, 32080, 0);
}

#[test]
fn the_arms_of_a_match_on_an_integer_input_agree_wherever_it_stands() {
    assert_covered_arms_hold(Chooser::Integer, 344, 0);
}

#[test]
#[ignore = "exhaustive"]
fn exhaustive_the_arms_of_a_match_on_an_integer_input_agree_wherever_it_stands() {
    assert_arms_hold(Chooser::Integer, 32080, 0);
}

#[test]
fn the_arms_of_a_choice_on_a_compile_time_recursive_functions_parameter_agree_wherever_it_stands() {
    assert_covered_arms_hold(Chooser::Parameter, 292, 0);
}

#[test]
#[ignore = "exhaustive"]
fn exhaustive_the_arms_of_a_choice_on_a_compile_time_recursive_functions_parameter_agree_wherever_it_stands()
 {
    assert_arms_hold(Chooser::Parameter, 12406, 0);
}

#[test]
fn the_arms_of_a_match_on_a_compile_time_recursive_functions_parameter_agree_wherever_it_stands() {
    assert_covered_arms_hold(Chooser::ParameterMatch, 292, 0);
}

#[test]
#[ignore = "exhaustive"]
fn exhaustive_the_arms_of_a_match_on_a_compile_time_recursive_functions_parameter_agree_wherever_it_stands()
 {
    assert_arms_hold(Chooser::ParameterMatch, 12406, 0);
}
