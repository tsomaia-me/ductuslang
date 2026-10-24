use std::collections::BTreeMap;

use symphonyscript_core::Scalar;

use crate::diagnostics::Code;
use crate::semantics::analysis::binding::BindingKind;
use crate::semantics::analysis::cval::CVal;
use crate::semantics::analysis::engine::Engine;
use crate::semantics::analysis::eval_ty::EvalTy;
use crate::semantics::analysis::ids::{BindingId, NodeId};
use crate::semantics::analysis::node::Op;
use crate::semantics::analysis::sig::{is_float, is_integer};
use crate::semantics::ir::{InputId, Loc};

pub(super) fn eval_ty(scalar: Option<Scalar>) -> EvalTy {
    match scalar {
        Some(Scalar::I32) => EvalTy::I32,
        Some(Scalar::I64) => EvalTy::I64,
        Some(Scalar::F32) => EvalTy::F32,
        Some(Scalar::F64) => EvalTy::F64,
        Some(Scalar::Bool) => EvalTy::Bool,
        Some(Scalar::Enum(id)) => EvalTy::Enum(id),
        Some(Scalar::Str) => EvalTy::Str,
        None => EvalTy::Default,
    }
}

impl Engine<'_> {
    pub(super) fn values(&mut self, roots: &[(NodeId, Option<Scalar>)]) {
        for (node, scalar) in roots {
            let ty = eval_ty(*scalar);
            let value = self.demand_cval(*node, ty);
            if !value.is_error() {
                self.fit(*node, value, ty);
            }
        }
    }

    pub(super) fn input_fits(&mut self) {
        for index in 0..self.bindings.len() {
            let binding = BindingId(u32::try_from(index).unwrap_or(u32::MAX));
            let record = self.binding(binding).clone();
            let BindingKind::Input { .. } = record.kind else {
                continue;
            };
            let Some(own) = self.typing.own.get(&binding).copied() else {
                continue;
            };
            let value = self.demand_cval(record.init, eval_ty(Some(own)));
            let expectations = self
                .typing
                .expectations
                .get(&binding)
                .cloned()
                .unwrap_or_default();
            for (scalar, loc) in expectations {
                let fits = match (&value, scalar) {
                    (CVal::Int(value), Scalar::I32) => i32::try_from(*value).is_ok(),
                    (CVal::F64(value), Scalar::F32) => {
                        value.is_nan() || (*value as f32).is_finite()
                    }
                    _ => true,
                };
                if !fits {
                    let scope = self.node(record.init).scope;
                    self.error(
                        Code::DoesNotFit,
                        loc,
                        scope,
                        format!(
                            "the initial value of `{}` does not fit the type this use reads it in",
                            record.name
                        ),
                    );
                }
            }
        }
    }

    pub(super) fn consistency(&mut self) {
        let mut kinds: crate::semantics::analysis::fast_hash::FastMap<NodeId, (bool, bool, bool)> =
            crate::semantics::analysis::fast_hash::FastMap::default();
        for ((node, ty), value) in &self.memo.evals {
            if let Op::Use(target) = self.node(*node).op
                && !value.is_error()
            {
                let entry = kinds.entry(target).or_default();
                match ty {
                    EvalTy::I32 | EvalTy::I64 | EvalTy::Exact => entry.0 = true,
                    EvalTy::F32 | EvalTy::F64 | EvalTy::DefaultF32 | EvalTy::DefaultF64 => {
                        entry.1 = true
                    }
                    EvalTy::Enum(_) => entry.2 = true,
                    _ => {}
                }
            }
        }
        let mut uses: BTreeMap<NodeId, Vec<(NodeId, EvalTy, CVal)>> = BTreeMap::new();
        for ((node, ty), value) in &self.memo.evals {
            if let Op::Use(target) = self.node(*node).op
                && !value.is_error()
                && kinds
                    .get(&target)
                    .is_some_and(|(int, float, enumeration)| (*int && *float) || *enumeration)
            {
                uses.entry(target)
                    .or_default()
                    .push((*node, *ty, value.clone()));
            }
        }
        for (_, mut evaluations) in uses {
            evaluations.sort_by_key(|(node, _, _)| *node);
            let integer = evaluations
                .iter()
                .find(|(_, ty, _)| matches!(ty, EvalTy::I32 | EvalTy::I64 | EvalTy::Exact));
            let float = evaluations.iter().find(|(node, ty, value)| {
                matches!(
                    ty,
                    EvalTy::F32 | EvalTy::F64 | EvalTy::DefaultF32 | EvalTy::DefaultF64
                ) && integer
                    .is_some_and(|(other, _, int)| other != node && !same_number(int, value))
            });
            if let (Some((int_node, _, _)), Some((float_node, _, _))) = (integer, float) {
                let float_loc = self.node(*float_node).loc;
                let (loc, scope) = {
                    let record = self.node(*int_node);
                    (record.loc, record.scope)
                };
                self.error_with_chain(
                    Code::InconsistentUses,
                    loc,
                    vec![float_loc],
                    scope,
                    "this value is an integer here and a different number at its float use",
                );
            }
            let enums: Vec<&(NodeId, EvalTy, CVal)> = evaluations
                .iter()
                .filter(|(_, ty, _)| matches!(ty, EvalTy::Enum(_)))
                .collect();
            if let (Some(first), Some(other)) = (
                enums.first(),
                enums
                    .iter()
                    .find(|(_, ty, _)| Some(ty) != enums.first().map(|(_, first, _)| first)),
            ) {
                let other_loc = self.node(other.0).loc;
                let (loc, scope) = {
                    let record = self.node(first.0);
                    (record.loc, record.scope)
                };
                self.error_with_chain(
                    Code::TypeMismatch,
                    loc,
                    vec![other_loc],
                    scope,
                    "this variant takes its enum from its uses, and they expect different enums",
                );
            }
        }
    }

    pub(super) fn integer_inputs(&mut self) {
        let mut inputs: BTreeMap<InputId, (usize, usize, Loc)> = BTreeMap::new();
        for index in 0..self.bindings.len() {
            let binding = BindingId(u32::try_from(index).unwrap_or(u32::MAX));
            let record = self.binding(binding);
            let BindingKind::Input { input, .. } = record.kind else {
                continue;
            };
            if !self
                .typing
                .own
                .get(&binding)
                .is_some_and(|own| is_integer(*own))
            {
                continue;
            }
            let reads = self.typing.reads.get(&binding).map_or(0, Vec::len);
            let floats = self
                .typing
                .expectations
                .get(&binding)
                .map_or(0, |expected| {
                    expected
                        .iter()
                        .filter(|(scalar, _)| is_float(*scalar))
                        .count()
                });
            let loc = self
                .hir
                .input(input)
                .name
                .as_ref()
                .map_or(record.loc, |name| name.loc);
            let entry = inputs.entry(input).or_insert((0, 0, loc));
            entry.0 += reads;
            entry.1 += floats;
        }
        for (input, (reads, floats, loc)) in inputs {
            if reads > 0 && reads == floats {
                let name = self
                    .hir
                    .input(input)
                    .name
                    .as_ref()
                    .map_or(String::new(), |name| name.text.clone());
                self.report.error(
                    self.hir,
                    Code::IntegerInput,
                    loc,
                    format!(
                        "`{name}` is an integer input whose every use expects a float, so its host cannot write a fraction; a float initializer such as `1.0` makes it a float input"
                    ),
                );
            }
        }
    }
}

fn same_number(int: &CVal, float: &CVal) -> bool {
    match (int, float) {
        (CVal::Int(value), CVal::F32(other)) => (*value as f32).to_bits() == other.to_bits(),
        (CVal::Int(value), CVal::F64(other)) => (*value as f64).to_bits() == other.to_bits(),
        _ => true,
    }
}
