use crate::semantics::builtin::Builtin;
use symphonyscript_core::{BinaryOp, UnaryOp};

use super::extra::Extra;
use crate::semantics::Internal;
use crate::semantics::analysis::Engine;
use crate::semantics::analysis::ids::NodeId;
use crate::semantics::analysis::kind::Kind;
use crate::semantics::analysis::op::Op;
use crate::semantics::analysis::sig::Sig;
use crate::semantics::analysis::tests::with_engine;
use crate::semantics::analysis::val::Val;
use crate::semantics::analysis::width::Width;
use crate::semantics::ir::{FnId, Loc, Number, Suffix};

#[test]
fn a_missing_operand_signature_is_an_internal_error() {
    with_engine("Session s\n", |engine| {
        let scope = engine.source_global;
        let push = |engine: &mut Engine<'_>, op: Op| -> NodeId {
            engine
                .push_node(op, Loc::default(), scope)
                .expect("the node is pushed")
        };
        let int = Sig::of(Kind::Int, Width::Flexible, false, None);
        let one = push(engine, Op::Number(Number::Int(1), Suffix::None));
        let two = push(engine, Op::Number(Number::Int(2), Suffix::None));
        let three = push(engine, Op::Number(Number::Int(3), Suffix::None));
        let missing = Err(Internal::missed("operand signature"));
        let rows = vec![
            (
                Op::Binary(BinaryOp::Add, one, two),
                vec![one, two],
                vec![int],
                missing,
            ),
            (
                Op::Binary(BinaryOp::Add, one, two),
                vec![one, two],
                vec![int, int],
                Ok(Kind::Int),
            ),
            (
                Op::Unary(UnaryOp::Neg, one),
                vec![one],
                vec![int],
                Ok(Kind::Int),
            ),
            (
                Op::Builtin(Builtin::Abs, vec![one]),
                vec![one],
                vec![],
                missing,
            ),
            (
                Op::Match {
                    scrutinee: one,
                    arms: vec![(two, three)],
                    otherwise: Some(three),
                },
                vec![one, two, three, three],
                vec![int, int, int],
                missing,
            ),
        ];
        for (op, operands, sigs, expected) in rows {
            let node = push(engine, op.clone());
            assert_eq!(
                engine
                    .rule(node, &op, &operands, &sigs, Extra::None)
                    .map(|sig| sig.kind),
                expected,
                "{op:?}"
            );
        }

        let arg = push(engine, Op::Arg(0));
        assert_eq!(
            engine.demand_sig(arg),
            Err(Internal::missed("argument signature"))
        );
        let unsigned = push(engine, Op::Bool(true));
        let call = push(
            engine,
            Op::Recursive {
                function: FnId(0),
                args: vec![Some(Val::Scalar(unsigned))],
            },
        );
        let mut need = Vec::new();
        assert_eq!(
            engine.recursive_sig(call, FnId(0), &[Some(Val::Scalar(unsigned))], &mut need),
            Err(Internal::missed("signature"))
        );
    });
}
