use super::ctx::Ctx;
use super::frame::Frame;
use super::machine::Machine;
use crate::semantics::Internal;
use crate::semantics::analysis::call_key::CallKey;
use crate::semantics::analysis::eval_ty::EvalTy;
use crate::semantics::analysis::fast_hash::FastMap;
use crate::semantics::analysis::kind::Kind;
use crate::semantics::analysis::op::Op;
use crate::semantics::analysis::sig::Sig;
use crate::semantics::analysis::tests::with_engine;
use crate::semantics::analysis::width::Width;
use crate::semantics::ir::{FnId, Loc};

#[test]
fn a_miss_in_the_recursive_evaluators_own_state_is_an_internal_error() {
    with_engine("fn f(n): f(n)\nSession s\n", |engine| {
        let scope = engine.source_global;
        let function = FnId(0);
        let call = engine
            .push_node(
                Op::Recursive {
                    function,
                    args: Vec::new(),
                },
                Loc::default(),
                scope,
            )
            .expect("the call is pushed");
        let key = CallKey {
            function,
            args: Vec::new(),
            shapes: Vec::new(),
            expected: EvalTy::Exact,
            units: Vec::new(),
            nesting: 0,
        };
        let mut need = Vec::new();
        let mut evaluated = |engine: &mut crate::semantics::analysis::Engine<'_>, ctx| {
            engine.recursive(call, (function, Vec::new()), EvalTy::Exact, ctx, &mut need)
        };

        engine.machines.insert(key.clone(), Machine::default());
        assert_eq!(
            evaluated(engine, Ctx::Global),
            Err(Internal::missed("evaluation work"))
        );
        engine.machines.insert(
            key.clone(),
            Machine {
                frames: Vec::new(),
                work: vec![(0, call, EvalTy::Exact)],
                open: FastMap::default(),
                open_keys: Default::default(),
                too_deep: false,
            },
        );
        assert_eq!(
            evaluated(engine, Ctx::Global),
            Err(Internal::missed("evaluation frame"))
        );
        engine.active = None;
        assert_eq!(
            evaluated(engine, Ctx::Frame(0)),
            Err(Internal::missed("evaluation machine"))
        );
        assert_eq!(
            engine.frame_arg(0, 0),
            Err(Internal::missed("evaluation machine"))
        );

        engine.active = Some(Machine {
            frames: vec![Frame {
                key,
                function,
                args: Vec::new(),
                call,
                root: call,
                ty: EvalTy::Exact,
                expected: EvalTy::Exact,
                feeds: Vec::new(),
                memo: FastMap::default(),
                unbounded: 1,
                faults: Default::default(),
                templates: Default::default(),
                checking: false,
                check_open: None,
            }],
            work: Vec::new(),
            open: FastMap::default(),
            open_keys: Default::default(),
            too_deep: false,
        });
        assert_eq!(engine.frame_arg(0, 0), Err(Internal::missed("argument")));
        assert_eq!(
            engine.frame_value(1, call, EvalTy::Exact),
            Err(Internal::missed("evaluation frame"))
        );

        let mut need = Vec::new();
        let unsigned = engine
            .push_node(Op::Bool(true), Loc::default(), scope)
            .expect("the operand is pushed");
        assert_eq!(
            engine.child(unsigned, EvalTy::Bool, Ctx::Frame(0), &mut need),
            Err(Internal::missed("signature"))
        );
        let arg = engine
            .push_node(Op::Arg(0), Loc::default(), scope)
            .expect("the argument is pushed");
        let mut arg_sig = Sig::of(Kind::Int, Width::Flexible, false, None);
        arg_sig.frame = true;
        engine.memo.sigs.insert(arg, arg_sig);
        assert_eq!(
            engine.evaluate(arg, EvalTy::Exact, Ctx::Global, &mut need),
            Err(Internal::missed("evaluation frame"))
        );
    });
}
