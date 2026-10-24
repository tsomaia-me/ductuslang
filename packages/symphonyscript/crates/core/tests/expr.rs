use std::hash::{Hash, Hasher};

use symphonyscript_core::{
    BinaryOp, Cell, Expr, ExprError, ExprNode, ExprRef, Program, Span, Ty, Value, Vocabulary, dump,
};

#[derive(Default)]
struct CountingHasher {
    bytes: usize,
}

impl Hasher for CountingHasher {
    fn finish(&self) -> u64 {
        0
    }

    fn write(&mut self, bytes: &[u8]) {
        self.bytes += bytes.len();
    }
}

fn hashed_bytes(expr: &Expr) -> usize {
    let mut hasher = CountingHasher::default();
    expr.hash(&mut hasher);
    hasher.bytes
}

fn doubled(levels: u32, leaf: i32) -> Expr {
    let mut expr = Expr::leaf(ExprNode::Const(Value::I32(leaf))).expect("a leaf has no operands");
    for level in 1..=levels {
        let below = ExprRef(level - 1);
        expr.push(ExprNode::Binary {
            op: BinaryOp::Add,
            ty: Ty::I32,
            lhs: below,
            rhs: below,
        })
        .expect("both operands precede their node");
    }
    expr
}

fn dumped(expr: Expr) -> String {
    let vocab = Vocabulary::builder("shared")
        .build()
        .expect("an empty vocabulary is valid");
    let program = Program {
        vocabulary: vocab.stamp().clone(),
        top_level: vec![],
        instances: vec![],
        wires: vec![],
        inputs: vec![],
        cells: vec![Cell {
            ty: Ty::I32,
            expr,
            span: Span {
                file: "shared.sym".into(),
                range: 0..1,
            },
        }],
        strings: vec![],
    };
    dump(&program, &vocab)
}

#[test]
fn shared_operands_cost_one_visit_per_node() {
    let expr = doubled(40, 1);
    let copy = expr.clone();
    assert_eq!(copy.nodes().len(), 41);
    assert_eq!(copy.root(), ExprRef(40));
    assert_eq!(expr, copy);
    assert_ne!(expr, doubled(40, 2));
    assert_ne!(expr, doubled(39, 1));

    let per_level = hashed_bytes(&doubled(1, 1)) - hashed_bytes(&doubled(0, 1));
    assert_eq!(
        hashed_bytes(&expr),
        hashed_bytes(&doubled(0, 1)) + 40 * per_level
    );

    assert_eq!(format!("{expr:?}").matches("Binary {").count(), 40);

    let text = dumped(copy);
    assert_eq!(text.matches("\n    e").count(), 41);
    assert!(text.contains("\n    e0 const i32 1\n    e1 add i32 e0 e0\n"));
    assert!(text.contains("\n    e40 add i32 e39 e39\n  span"));

    drop(expr);
}

#[test]
fn an_operand_must_come_before_its_node() {
    let unary = |operand: u32| ExprNode::Unary {
        op: symphonyscript_core::UnaryOp::Neg,
        ty: Ty::I32,
        operand: ExprRef(operand),
    };
    assert_eq!(
        Expr::leaf(unary(0)),
        Err(ExprError::ForwardOperand {
            node: ExprRef(0),
            operand: ExprRef(0),
        })
    );

    let pick = |items: [u32; 2]| ExprNode::Pick {
        index_ty: Ty::I32,
        index: ExprRef(0),
        items: items.into_iter().map(ExprRef).collect(),
    };
    for (node, expected) in [
        (
            unary(1),
            Err(ExprError::ForwardOperand {
                node: ExprRef(1),
                operand: ExprRef(1),
            }),
        ),
        (
            unary(7),
            Err(ExprError::ForwardOperand {
                node: ExprRef(1),
                operand: ExprRef(7),
            }),
        ),
        (
            pick([0, 2]),
            Err(ExprError::ForwardOperand {
                node: ExprRef(1),
                operand: ExprRef(2),
            }),
        ),
        (unary(0), Ok(ExprRef(1))),
    ] {
        let mut expr = Expr::leaf(ExprNode::Const(Value::I32(1))).expect("a leaf has no operands");
        let refused = expected.is_err();
        assert_eq!(expr.push(node), expected);
        let kept = if refused { 1 } else { 2 };
        assert_eq!(expr.nodes().len(), kept);
        assert_eq!(expr.root(), ExprRef(kept as u32 - 1));
    }
}
