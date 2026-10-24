use symphonyscript_core::{
    AttrId, BinaryOp, Binding, Builtin, Cell, CellId, Count, Entry, Expr, ExprNode, ExprRef, Input,
    InputSlot, Instance, InstanceId, Path, Program, Scalar, Segment, Span, StrId, Target, Ty,
    UnaryOp, Value, Vocabulary, Wire, WireId, dump,
};

fn span(file: &str, start: u32, end: u32) -> Span {
    Span {
        file: file.into(),
        range: start..end,
    }
}

fn path(segments: &[(&str, Option<u32>)]) -> Path {
    Path {
        segments: segments
            .iter()
            .map(|(name, index)| match index {
                Some(index) => Segment::Anonymous {
                    name: (*name).into(),
                    index: *index,
                },
                None => Segment::Named((*name).into()),
            })
            .collect(),
    }
}

fn leaf(node: ExprNode) -> Expr {
    Expr::leaf(node).expect("a leaf has no operands")
}

fn push(expr: &mut Expr, node: ExprNode) -> ExprRef {
    expr.push(node).expect("operands precede their node")
}

fn first_cell() -> Expr {
    let mut expr = leaf(ExprNode::Input(InputSlot(3)));
    let input = expr.root();
    let converted = push(
        &mut expr,
        ExprNode::Convert {
            from: Ty::F64,
            to: Ty::F32,
            operand: input,
        },
    );
    let factor = push(&mut expr, ExprNode::Const(Value::F32(0.1)));
    push(
        &mut expr,
        ExprNode::Binary {
            op: BinaryOp::Mul,
            ty: Ty::F32,
            lhs: converted,
            rhs: factor,
        },
    );
    expr
}

fn second_cell() -> Expr {
    let mut expr = leaf(ExprNode::Input(InputSlot(0)));
    let scrutinee = expr.root();
    let nan = push(&mut expr, ExprNode::Const(Value::F64(f64::NAN)));
    let other_nan = push(
        &mut expr,
        ExprNode::Const(Value::F64(f64::from_bits(0x7FF8_0000_0000_0001))),
    );
    let infinity = push(&mut expr, ExprNode::Const(Value::F64(f64::INFINITY)));
    let negative = push(&mut expr, ExprNode::Const(Value::F64(f64::NEG_INFINITY)));
    let tiny = push(&mut expr, ExprNode::Const(Value::F64(1e-20)));
    let one = push(&mut expr, ExprNode::Const(Value::F64(1.0)));
    let huge = push(&mut expr, ExprNode::Const(Value::F64(2.5e300)));
    let cell = push(&mut expr, ExprNode::Cell(CellId(0)));
    let lerp = push(
        &mut expr,
        ExprNode::Call {
            builtin: Builtin::Lerp,
            ty: Ty::F64,
            args: vec![one, huge, cell],
        },
    );
    let pick = push(
        &mut expr,
        ExprNode::Pick {
            index_ty: Ty::I64,
            index: scrutinee,
            items: vec![infinity, negative, tiny, lerp],
        },
    );
    push(
        &mut expr,
        ExprNode::Match {
            ty: Ty::I64,
            scrutinee,
            arms: vec![(Value::I64(0), nan), (Value::I64(1), other_nan)],
            otherwise: Some(pick),
        },
    );
    expr
}

fn third_cell(wave: symphonyscript_core::EnumId) -> Expr {
    let mut expr = leaf(ExprNode::Input(InputSlot(1)));
    let input = expr.root();
    let condition = push(
        &mut expr,
        ExprNode::Unary {
            op: UnaryOp::Not,
            ty: Ty::BOOL,
            operand: input,
        },
    );
    let saw = push(&mut expr, ExprNode::Const(Value::Enum(wave, 1)));
    let sine = push(&mut expr, ExprNode::Const(Value::Enum(wave, 0)));
    let then = push(
        &mut expr,
        ExprNode::Binary {
            op: BinaryOp::Is,
            ty: Ty::of(Scalar::Enum(wave)),
            lhs: saw,
            rhs: sine,
        },
    );
    let seed = push(&mut expr, ExprNode::Const(Value::I64(i64::MAX)));
    let section = push(&mut expr, ExprNode::Input(InputSlot(0)));
    let negated = push(
        &mut expr,
        ExprNode::Unary {
            op: UnaryOp::Neg,
            ty: Ty::I64,
            operand: section,
        },
    );
    let rand = push(
        &mut expr,
        ExprNode::Call {
            builtin: Builtin::Rand,
            ty: Ty::I64,
            args: vec![seed, negated],
        },
    );
    let odd_nan = push(
        &mut expr,
        ExprNode::Const(Value::F32(f32::from_bits(0x7FC0_0001))),
    );
    let otherwise = push(
        &mut expr,
        ExprNode::Binary {
            op: BinaryOp::Gt,
            ty: Ty::F32,
            lhs: rand,
            rhs: odd_nan,
        },
    );
    push(
        &mut expr,
        ExprNode::If {
            condition,
            then,
            otherwise,
        },
    );
    expr
}

fn fixture() -> (Program, Vocabulary) {
    let mut v = Vocabulary::builder("dump");
    let time = v.unit_family("time").base("s").unit("ms", 0.001);
    let wave = v.enumeration("Wave", ["Sine", "Saw"]);
    let clip = v.node("Clip");
    let note = v.node("Note");
    let plays = v.wire("Plays");
    v.kind(clip)
        .top_level(Count::Any)
        .children(note, Count::Any)
        .attr::<f32>("gain")
        .default(1.0)
        .attr::<[bool; 2]>("steps")
        .default([true, false])
        .attr::<&'static str>("label")
        .default("");
    v.kind(note)
        .attr::<i32>("pitch")
        .default(60)
        .default_attr()
        .attr_enum("wave", &wave, "Sine");
    v.wire_kind(plays)
        .connects(clip, clip)
        .attr::<f64>("delay")
        .default(0.0)
        .family(&time, "ms");
    let vocab = v.build().expect("the dump vocabulary is valid");
    let wave = vocab.enum_named("Wave").expect("Wave exists");
    let time = vocab.family_at(0).expect("time exists");
    let note_kind = |name: &str| vocab.node_kind_named(name).expect("kind exists");

    let program = Program {
        vocabulary: vocab.stamp().clone(),
        top_level: vec![InstanceId(0), InstanceId(4)],
        instances: vec![
            Instance {
                kind: note_kind("Clip"),
                path: path(&[("intro", None)]),
                span: span("main.sym", 0, 120),
                expanded_from: vec![],
                attrs: vec![
                    (AttrId(0), Binding::Cell(CellId(0))),
                    (
                        AttrId(1),
                        Binding::Array(vec![
                            Binding::Input(InputSlot(1)),
                            Binding::Const(Value::Bool(false)),
                        ]),
                    ),
                    (AttrId(2), Binding::Const(Value::Str(StrId(1)))),
                ],
                gate: Some(Binding::Input(InputSlot(2))),
                sequence: vec![
                    Entry::Child(InstanceId(1)),
                    Entry::Bundle(vec![InstanceId(2), InstanceId(3)]),
                    Entry::Wire(WireId(0)),
                    Entry::Wire(WireId(1)),
                ],
            },
            Instance {
                kind: note_kind("Note"),
                path: path(&[("intro", None), ("hook", None)]),
                span: span("main.sym", 10, 20),
                expanded_from: vec![],
                attrs: vec![
                    (AttrId(0), Binding::Const(Value::I32(-48))),
                    (AttrId(1), Binding::Const(Value::Enum(wave, 1))),
                ],
                gate: None,
                sequence: vec![],
            },
            Instance {
                kind: note_kind("Note"),
                path: path(&[("intro", None), ("Arp", Some(0)), ("Note", Some(0))]),
                span: span("lib/arp.sym", 30, 44),
                expanded_from: vec![span("main.sym", 21, 27), span("lib/riff.sym", 2, 9)],
                attrs: vec![
                    (AttrId(0), Binding::Const(Value::I32(60))),
                    (AttrId(1), Binding::Const(Value::Enum(wave, 0))),
                ],
                gate: None,
                sequence: vec![],
            },
            Instance {
                kind: note_kind("Note"),
                path: path(&[("intro", None), ("Note", Some(0))]),
                span: span("main.sym", 28, 35),
                expanded_from: vec![],
                attrs: vec![
                    (AttrId(0), Binding::Const(Value::I32(i32::MIN))),
                    (AttrId(9), Binding::Const(Value::Enum(wave, 7))),
                ],
                gate: Some(Binding::Const(Value::Bool(true))),
                sequence: vec![],
            },
            Instance {
                kind: note_kind("Clip"),
                path: path(&[("verse", None)]),
                span: span("main.sym", 121, 140),
                expanded_from: vec![],
                attrs: vec![
                    (AttrId(0), Binding::Const(Value::F32(-0.0))),
                    (
                        AttrId(1),
                        Binding::Array(vec![
                            Binding::Const(Value::Bool(true)),
                            Binding::Const(Value::Bool(false)),
                        ]),
                    ),
                ],
                gate: None,
                sequence: vec![],
            },
        ],
        wires: vec![
            Wire {
                kind: vocab.wire_kind_named("Plays").expect("Plays exists"),
                path: path(&[("intro", None), ("Plays", Some(0))]),
                span: span("main.sym", 40, 52),
                expanded_from: vec![],
                source: InstanceId(0),
                target: Target::Fixed(InstanceId(4)),
                attrs: vec![(AttrId(0), Binding::Const(Value::F64(0.2)))],
                gate: None,
            },
            Wire {
                kind: vocab.wire_kind_named("Plays").expect("Plays exists"),
                path: path(&[("intro", None), ("back", None)]),
                span: span("main.sym", 53, 80),
                expanded_from: vec![],
                source: InstanceId(0),
                target: Target::Select {
                    candidates: vec![InstanceId(4), InstanceId(0)],
                    index: Binding::Input(InputSlot(0)),
                },
                attrs: vec![(AttrId(0), Binding::Cell(CellId(1)))],
                gate: Some(Binding::Cell(CellId(2))),
            },
        ],
        inputs: vec![
            Input {
                name: "section".into(),
                ty: Ty::I64,
                family: None,
                slots: InputSlot(0)..InputSlot(1),
                initial: vec![Value::I64(0)],
                span: span("main.sym", 0, 17),
            },
            Input {
                name: "steps".into(),
                ty: Ty::BOOL.array(1).array(2),
                family: None,
                slots: InputSlot(1)..InputSlot(3),
                initial: vec![Value::Bool(true), Value::Bool(false)],
                span: span("main.sym", 18, 50),
            },
            Input {
                name: "delay".into(),
                ty: Ty::F64,
                family: Some(time),
                slots: InputSlot(3)..InputSlot(4),
                initial: vec![Value::F64(0.005)],
                span: span("main.sym", 51, 70),
            },
        ],
        cells: vec![
            Cell {
                ty: Ty::F32,
                expr: first_cell(),
                span: span("main.sym", 71, 90),
            },
            Cell {
                ty: Ty::F64,
                expr: second_cell(),
                span: span("main.sym", 91, 100),
            },
            Cell {
                ty: Ty::BOOL,
                expr: third_cell(wave),
                span: span("main.sym", 101, 110),
            },
        ],
        strings: vec![
            "kit/0.wav".into(),
            "a \"quoted\" \\ line\nnext\ttab\u{1}".into(),
        ],
    };
    (program, vocab)
}

#[test]
fn a_hand_built_program_dumps_to_its_golden_text() {
    let (program, vocab) = fixture();
    assert_eq!(dump(&program, &vocab), include_str!("golden/dump.golden"));
}

#[test]
fn programs_compare_floats_by_bits() {
    let (program, _) = fixture();
    let mut changed = program.clone();
    assert_eq!(program, changed);
    changed.instances[4].attrs[0].1 = Binding::Const(Value::F32(0.0));
    assert_ne!(program, changed);
}

#[test]
fn every_operator_and_built_in_has_a_distinct_name() {
    let binary = [
        BinaryOp::Add,
        BinaryOp::Sub,
        BinaryOp::Mul,
        BinaryOp::Div,
        BinaryOp::IntDiv,
        BinaryOp::Rem,
        BinaryOp::Pow,
        BinaryOp::Is,
        BinaryOp::IsNot,
        BinaryOp::Lt,
        BinaryOp::Le,
        BinaryOp::Gt,
        BinaryOp::Ge,
        BinaryOp::And,
        BinaryOp::Or,
    ];
    let builtins = [
        Builtin::Abs,
        Builtin::Min,
        Builtin::Max,
        Builtin::Clamp,
        Builtin::Floor,
        Builtin::Ceil,
        Builtin::Round,
        Builtin::Sqrt,
        Builtin::Sin,
        Builtin::Cos,
        Builtin::Tan,
        Builtin::Exp,
        Builtin::Log,
        Builtin::Lerp,
        Builtin::Rand,
    ];
    let mut names: Vec<&str> = binary.iter().map(|op| op.name()).collect();
    names.extend([UnaryOp::Neg.name(), UnaryOp::Not.name()]);
    let operator_count = names.len();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), operator_count);
    let mut builtin_names: Vec<&str> = builtins.iter().map(|builtin| builtin.name()).collect();
    builtin_names.sort_unstable();
    builtin_names.dedup();
    assert_eq!(builtin_names.len(), builtins.len());
    let mut expected: Vec<&str> = symphonyscript_core::BUILTINS
        .iter()
        .copied()
        .filter(|name| *name != "pi")
        .collect();
    expected.sort_unstable();
    assert_eq!(builtin_names, expected);
}
