use symphonyscript_core::{Builtin, NativeId, Scalar};

use crate::diagnostics::Code;
use crate::semantics::Internal;
use crate::semantics::analysis::engine::Engine;
use crate::semantics::analysis::evaluation::native_ty;
use crate::semantics::analysis::ids::ScopeId;
use crate::semantics::analysis::kind::Kind;
use crate::semantics::analysis::op::Op;
use crate::semantics::analysis::query::Query;
use crate::semantics::analysis::some::some;
use crate::semantics::analysis::val::Val;
use crate::semantics::ir::{Args, Loc};

impl Engine<'_> {
    pub(in crate::semantics::analysis) fn native_call(
        &mut self,
        native: NativeId,
        args: &Args,
        loc: Loc,
        scope: ScopeId,
        need: &mut Vec<Query>,
    ) -> Result<Option<Val>, Internal> {
        let decl = self
            .vocabulary
            .native(native)
            .ok_or(Internal::missed("native function"))?;
        let Args::Positional(values) = args else {
            self.error(
                Code::NamedBuiltinArguments,
                loc,
                scope,
                format!(
                    "the native function `{}` takes positional arguments only",
                    decl.name
                ),
            )?;
            return Ok(Some(Val::Error));
        };
        let vals: Vec<Option<Val>> = values
            .iter()
            .map(|value| self.val(Query::Expr(*value, scope), need))
            .collect();
        if vals.iter().any(Option::is_none) {
            return Ok(None);
        }
        let arity = decl.signature.params.len();
        if vals.len() != arity {
            self.error(
                Code::ArgumentCount,
                loc,
                scope,
                format!(
                    "`{}` takes {arity} argument{}, not {}",
                    decl.name,
                    if arity == 1 { "" } else { "s" },
                    vals.len()
                ),
            )?;
            return Ok(Some(Val::Error));
        }
        let mut operands = Vec::with_capacity(arity);
        for ((val, param), value) in vals
            .into_iter()
            .flatten()
            .zip(&decl.signature.params)
            .zip(values)
        {
            let arg_loc = self.hir.expr(*value)?.loc;
            let ty = native_ty(param);
            let array_param =
                !ty.is_scalar() || !matches!(param, symphonyscript_core::NativeType::Value(_));
            if matches!(val, Val::Error) {
                return Ok(Some(Val::Error));
            }
            if !array_param {
                self.single_operand(&val, Code::TypeMismatch, SCALAR_PARAM);
                match val {
                    Val::Scalar(node) if !self.definite_array(node)? => operands.push(node),
                    Val::Scalar(_) | Val::Array(_) | Val::Error => {
                        self.error(Code::TypeMismatch, loc, scope, SCALAR_PARAM)?;
                        return Ok(Some(Val::Error));
                    }
                }
                continue;
            }
            let open = matches!(param, symphonyscript_core::NativeType::Open(_));
            let dimensions = ty.lengths.len() + usize::from(open || ty.capacity.is_some());
            if ty.scalar == Scalar::U8
                && dimensions == 1
                && let Val::Scalar(node) = val
                && !self.arrays.contains(&node)
            {
                let sig = some!(self.sig(node, need));
                if sig.is_error() {
                    return Ok(Some(Val::Error));
                }
                if sig.kind == Kind::Str {
                    operands.push(self.push_node(
                        Op::Builtin(Builtin::Bytes, vec![node]),
                        arg_loc,
                        scope,
                    )?);
                    continue;
                }
            }
            if let Val::Scalar(node) = val
                && !self.arrays.contains(&node)
            {
                self.error(Code::TypeMismatch, loc, scope, ARRAY_PARAM)?;
                return Ok(Some(Val::Error));
            }
            let Some(node) = self.boxed(&val, arg_loc, scope)? else {
                return Ok(Some(Val::Error));
            };
            operands.push(node);
        }
        Ok(Some(self.scalar(
            Op::Native {
                native,
                args: operands,
            },
            loc,
            scope,
        )?))
    }
}

const SCALAR_PARAM: &str = "this parameter takes a single value, not an array";

const ARRAY_PARAM: &str = "this parameter takes an array, not a single value";
