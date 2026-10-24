use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use symphonyscript_core::{
    AttrId, Binding, Cell, Expr, ExprNode, Input, InputSlot, Instance, Path, Program, Span, Ty,
    UnaryOp, Value, Vocabulary, dump,
};

const DEPTH: usize = 100_000;

fn on_small_stack(work: impl FnOnce() + Send + 'static) {
    std::thread::Builder::new()
        .stack_size(1 << 20)
        .spawn(work)
        .expect("the thread starts")
        .join()
        .expect("the work finishes without overflowing");
}

fn hash_of(value: &impl Hash) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

fn deep_ty() -> Ty {
    (0..DEPTH).fold(Ty::I32, |ty, _| ty.array(1))
}

fn deep_expr(leaf: i32) -> Expr {
    let mut expr = Expr::leaf(ExprNode::Const(Value::I32(leaf))).expect("a leaf has no operands");
    for _ in 0..DEPTH {
        let operand = expr.root();
        expr.push(ExprNode::Unary {
            op: UnaryOp::Neg,
            ty: Ty::I32,
            operand,
        })
        .expect("the operand precedes its node");
    }
    expr
}

fn deep_binding(leaf: i32) -> Binding {
    (0..DEPTH).fold(Binding::Const(Value::I32(leaf)), |binding, _| {
        Binding::Array(vec![binding])
    })
}

fn span() -> Span {
    Span {
        file: "deep.sym".into(),
        range: 0..1,
    }
}

#[test]
fn deep_types_compare_hash_clone_and_drop() {
    on_small_stack(|| {
        let ty = deep_ty();
        let copy = ty.clone();
        assert_eq!(ty, copy);
        assert_eq!(hash_of(&ty), hash_of(&copy));
        assert_ne!(ty, deep_ty().array(1));
        assert_eq!(ty.element_count(), 1);
        drop(copy);
        drop(ty);
    });
}

#[test]
fn deep_exprs_compare_hash_clone_and_drop() {
    on_small_stack(|| {
        let expr = deep_expr(1);
        let copy = expr.clone();
        assert_eq!(expr, copy);
        assert_eq!(hash_of(&expr), hash_of(&copy));
        assert_ne!(expr, deep_expr(2));
        drop(copy);
        drop(expr);
    });
}

#[test]
fn deep_bindings_compare_hash_clone_print_and_drop() {
    on_small_stack(|| {
        let binding = deep_binding(1);
        let copy = binding.clone();
        assert_eq!(binding, copy);
        assert_eq!(hash_of(&binding), hash_of(&copy));
        assert_ne!(binding, deep_binding(2));
        let text = format!("{binding:?}");
        assert!(text.starts_with("Array([Array(["));
        assert!(text.ends_with("])])"));
        assert_eq!(
            text.len(),
            DEPTH * "Array([])".len() + "Const(I32(1))".len()
        );
        drop(copy);
        drop(binding);
    });
}

#[test]
fn a_program_of_deep_values_dumps_compares_clones_and_drops() {
    on_small_stack(|| {
        let mut v = Vocabulary::builder("deep");
        let clip = v.node("Clip");
        v.kind(clip).attr::<i32>("steps");
        let vocab = v.build().expect("the vocabulary is valid");
        let program = Program {
            vocabulary: vocab.stamp().clone(),
            top_level: vec![],
            instances: vec![Instance {
                kind: vocab.node_kind_at(0).expect("Clip exists"),
                path: Path::default(),
                span: span(),
                expanded_from: vec![],
                attrs: vec![(AttrId(0), deep_binding(3))],
                gate: Some(deep_binding(4)),
                sequence: vec![],
            }],
            wires: vec![],
            inputs: vec![Input {
                name: "grid".into(),
                ty: deep_ty(),
                family: None,
                slots: InputSlot(0)..InputSlot(1),
                initial: vec![Value::I32(0)],
                span: span(),
            }],
            cells: vec![Cell {
                ty: Ty::I32,
                expr: deep_expr(7),
                span: span(),
            }],
            strings: vec![],
        };
        let copy = program.clone();
        assert_eq!(program, copy);
        let text = dump(&program, &vocab);
        assert!(text.contains(&format!("  ty i32{}\n", "[1]".repeat(DEPTH))));
        assert!(text.contains("  expr\n    e0 const i32 7\n    e1 neg i32 e0\n"));
        let last = format!("    e{DEPTH} neg i32 e{}\n  span", DEPTH - 1);
        assert!(text.contains(&last));
        assert_eq!(text.matches(" neg i32 e").count(), DEPTH);
        let binding = format!(
            "    steps {}const i32 3{}\n",
            "array [".repeat(DEPTH),
            "]".repeat(DEPTH)
        );
        assert!(text.contains(&binding));
        drop(copy);
        drop(program);
    });
}
