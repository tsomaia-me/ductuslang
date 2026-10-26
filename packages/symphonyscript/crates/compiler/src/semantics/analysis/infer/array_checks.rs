use super::position::Position;
use super::role::Role;
use crate::diagnostics::Code;
use crate::semantics::Internal;
use crate::semantics::analysis::engine::Engine;
use crate::semantics::analysis::resolve::LENGTH_LIMIT;
use crate::semantics::analysis::val::Val;

pub(super) enum ArrayPosition {
    Usual,
    Faulty,
    Variable,
}

impl Engine<'_> {
    pub(super) fn array_position(
        &mut self,
        position: &Position,
    ) -> Result<ArrayPosition, Internal> {
        let whole = match &position.val {
            Val::Scalar(node) if self.arrays.contains(node) => Some(*node),
            Val::Scalar(_) | Val::Array(_) | Val::Error => None,
        };
        if position.ty.capacity.is_none() {
            let Some(node) = whole else {
                return Ok(ArrayPosition::Usual);
            };
            if !self.definite_array(node)? {
                return Ok(ArrayPosition::Usual);
            }
            if self.is_variable(node)? {
                let (code, message) = if position.ty.lengths.is_empty() {
                    self.single_value_fault(position, "a variable array")?
                } else {
                    (
                        Code::VariableArray,
                        "this value is a variable array, and a fixed array is expected here".into(),
                    )
                };
                self.error(code, position.loc, position.scope, message)?;
                return Ok(ArrayPosition::Faulty);
            }
            return Ok(ArrayPosition::Usual);
        }
        let inner = match &position.val {
            Val::Array(array) if array.elements.is_empty() => return Ok(ArrayPosition::Variable),
            Val::Array(array) => array.shape.inner().lengths(),
            Val::Scalar(node) if self.arrays.contains(node) => {
                let found = self.known_inner(*node)?;
                match found {
                    Some(inner) => inner,
                    None => return Ok(ArrayPosition::Faulty),
                }
            }
            Val::Scalar(_) => {
                self.error(
                    Code::TypeMismatch,
                    position.loc,
                    position.scope,
                    "this attribute holds a variable array, and the value is a single value",
                )?;
                return Ok(ArrayPosition::Faulty);
            }
            Val::Error => return Ok(ArrayPosition::Faulty),
        };
        if inner != position.ty.lengths {
            self.error(
                Code::TypeMismatch,
                position.loc,
                position.scope,
                "the value's rows differ from the attribute's type",
            )?;
            return Ok(ArrayPosition::Faulty);
        }
        Ok(ArrayPosition::Variable)
    }

    pub(super) fn single_value_fault(
        &self,
        position: &Position,
        value: &str,
    ) -> Result<(Code, String), Internal> {
        let held = self.scalar_name(position.ty.scalar)?;
        Ok(match (position.role, &position.name) {
            (Role::Gate, _) => (
                Code::ConditionType,
                format!("a gate or a guard is one `bool`, and this value is {value}"),
            ),
            (Role::Attribute | Role::Initializer, Some(name)) => (
                Code::TypeMismatch,
                format!("`{name}` holds one `{held}`, and this value is {value}"),
            ),
            (Role::Attribute | Role::Initializer, None) => (
                Code::TypeMismatch,
                format!("this input holds one `{held}`, and this value is {value}"),
            ),
        })
    }

    fn known_inner(
        &mut self,
        node: crate::semantics::analysis::ids::NodeId,
    ) -> Result<Option<Vec<u32>>, Internal> {
        let found = self.known_layout(node)?;
        Ok(match found.shaped {
            Ok(Some(layout)) => Some(layout.inner().to_vec()),
            Ok(None) => None,
            Err(fault) => {
                if let Some(fault) = fault {
                    self.error_at_node(fault.code, fault.node, fault.message)?;
                }
                None
            }
        })
    }

    pub(super) fn capacity_checks(&mut self, positions: &[Position]) -> Result<(), Internal> {
        for position in positions {
            let Some(capacity) = position.ty.capacity else {
                continue;
            };
            let holds = match &position.val {
                Val::Array(array) => Some(array.shape.outer().unwrap_or(0)),
                Val::Scalar(node) => match self.extent(*node)? {
                    Some(held) => Some(held),
                    None => self.fixed_length(*node)?,
                },
                Val::Error => None,
            };
            if let Some(holds) = holds
                && holds > capacity
                && holds < LENGTH_LIMIT
            {
                self.error(
                    Code::CapacityExceeded,
                    position.loc,
                    position.scope,
                    format!(
                        "this value holds up to {holds} elements, more than the attribute's capacity {capacity}"
                    ),
                )?;
            }
        }
        Ok(())
    }

    fn fixed_length(
        &mut self,
        node: crate::semantics::analysis::ids::NodeId,
    ) -> Result<Option<u32>, Internal> {
        let found = self.known_layout(node)?;
        Ok(match found.shaped {
            Ok(Some(layout)) => layout.outer(),
            Ok(None) | Err(_) => None,
        })
    }
}

impl Engine<'_> {
    pub(super) fn state_checks(&mut self) -> Result<(), Internal> {
        use crate::semantics::analysis::bound::Bound;
        use crate::semantics::analysis::bound_val::BoundVal;
        use crate::semantics::analysis::history_cell::HistoryCell;
        use crate::semantics::analysis::op::Op;
        use crate::semantics::ir::StateId;
        for index in 0..self.hir.states.len() {
            let state = StateId(u32::try_from(index).unwrap_or(u32::MAX));
            let Some(BoundVal::Scalar(Bound::Cell(binding))) =
                self.memo.states.get(&state).cloned()
            else {
                continue;
            };
            let init = self.binding(binding)?.init;
            let reads = self.history_reads(init)?;
            let mut deepest = 0u64;
            for (node, _) in &reads {
                if let Op::History { offset, .. } = self.node(*node)?.op {
                    deepest = deepest.max(offset);
                }
            }
            let depth = self.hir.state(state)?.depth;
            for (node, cell) in reads {
                let Op::History {
                    offset, fallback, ..
                } = self.node(node)?.op
                else {
                    continue;
                };
                if let Some((Some(limit), _)) = depth
                    && offset > limit
                {
                    self.error_at_node(
                        Code::HistoryDepth,
                        node,
                        format!(
                            "this history read reaches {offset} steps back, deeper than `[{limit}]`; `[{deepest}]` covers every read of this state"
                        ),
                    )?;
                }
                if let HistoryCell::Let(id) = cell {
                    let global = self.global_of_let(id);
                    let reactive = match self
                        .memo
                        .lets
                        .get(&(global, crate::semantics::analysis::let_ref::LetRef::Top(id)))
                    {
                        Some(BoundVal::Scalar(Bound::Cell(_))) => true,
                        Some(BoundVal::Array(array)) => {
                            array.1.iter().any(|item| matches!(item, Bound::Cell(_)))
                        }
                        Some(BoundVal::Scalar(Bound::Value(_)) | BoundVal::Error) | None => false,
                    };
                    if !reactive {
                        self.error_at_node(
                            Code::HistoryTarget,
                            node,
                            "this `let` is a compile-time value, which has no history; a history read names an input, a state or a reactive `let`",
                        )?;
                        continue;
                    }
                }
                if let Some(fallback) = fallback {
                    self.fallback_check(node, cell, fallback)?;
                }
            }
            if let Some(layout) = self.memo.binding_layouts.get(&binding).cloned()
                && !layout.variable
                && self.extent(init)?.is_some()
            {
                let decl = self.hir.state(state)?;
                let name = decl
                    .name
                    .as_ref()
                    .map_or(String::new(), |name| name.text.clone());
                self.error(
                    Code::VariableArray,
                    self.hir.expr(decl.value)?.loc,
                    self.source_global,
                    format!(
                        "`{name}` holds a fixed array, and its initializer yields a variable array"
                    ),
                )?;
            }
        }
        Ok(())
    }

    fn fallback_check(
        &mut self,
        node: crate::semantics::analysis::ids::NodeId,
        cell: crate::semantics::analysis::history_cell::HistoryCell,
        fallback: crate::semantics::analysis::ids::NodeId,
    ) -> Result<(), Internal> {
        use crate::semantics::analysis::bound::Bound;
        use crate::semantics::analysis::bound_val::BoundVal;
        use crate::semantics::analysis::history_cell::HistoryCell;
        let bound = match cell {
            HistoryCell::Input(input) => self.memo.inputs.get(&input).cloned(),
            HistoryCell::Let(id) => self
                .memo
                .lets
                .get(&(
                    self.global_of_let(id),
                    crate::semantics::analysis::let_ref::LetRef::Top(id),
                ))
                .cloned(),
            HistoryCell::State(state) => self.memo.states.get(&state).cloned(),
        };
        let cell_layout = match &bound {
            Some(BoundVal::Array(array)) => Some(
                crate::semantics::analysis::layout::Layout::fixed(array.0.lengths()),
            ),
            Some(BoundVal::Scalar(Bound::Cell(binding))) => {
                self.memo.binding_layouts.get(binding).cloned()
            }
            Some(BoundVal::Scalar(Bound::Value(_)) | BoundVal::Error) | None => return Ok(()),
        };
        let found = self.known_layout(fallback)?;
        let Ok(fallback_layout) = found.shaped else {
            return Ok(());
        };
        let agrees = match (&cell_layout, &fallback_layout) {
            (None, None) => true,
            (Some(cell), Some(fallback)) if cell.variable => {
                fallback.inner() == cell.inner()
                    || (fallback.outer() == Some(0) && fallback.inner().is_empty())
            }
            (Some(cell), Some(fallback)) => cell == fallback && !fallback.variable,
            (None, Some(_)) | (Some(_), None) => false,
        };
        if !agrees {
            self.error_at_node(
                Code::TypeMismatch,
                node,
                "a history read's fallback has the cell's type, and this one's shape differs",
            )?;
        }
        Ok(())
    }

    pub(super) fn annotated_capacities(&mut self) -> Result<(), Internal> {
        use crate::semantics::analysis::binding_kind::BindingKind;
        use crate::semantics::analysis::ids::BindingId;
        for index in 0..self.bindings.len() {
            let binding = BindingId(u32::try_from(index).unwrap_or(u32::MAX));
            let record = self.binding(binding)?.clone();
            if !matches!(record.kind, BindingKind::Let | BindingKind::State { .. }) {
                continue;
            }
            if self.memo.fixed_annotations.contains(&binding) && self.is_variable(record.init)? {
                let scope = self.node(record.init)?.scope;
                self.error(
                    Code::VariableArray,
                    record.loc,
                    scope,
                    format!(
                        "`{}` is annotated as a fixed array, and its value is a variable array",
                        record.name
                    ),
                )?;
                continue;
            }
            let Some(capacity) = self.memo.binding_capacities.get(&binding).copied() else {
                continue;
            };
            let annotated = match record.kind {
                BindingKind::State { state } => self
                    .hir
                    .state(state)?
                    .ty
                    .as_ref()
                    .is_some_and(|ty| ty.capacity.is_some()),
                BindingKind::Let => true,
                BindingKind::Input { .. } | BindingKind::Param | BindingKind::Attr { .. } => false,
            };
            if !annotated {
                continue;
            }
            let holds = match self.extent(record.init)? {
                Some(held) => Some(held),
                None => self.fixed_length(record.init)?,
            };
            if let Some(holds) = holds
                && holds > capacity
                && holds < LENGTH_LIMIT
            {
                let scope = self.node(record.init)?.scope;
                self.error(
                    Code::CapacityExceeded,
                    record.loc,
                    scope,
                    format!(
                        "`{}` holds up to {holds} elements, more than its annotated capacity {capacity}",
                        record.name
                    ),
                )?;
            }
        }
        Ok(())
    }

    pub(super) fn native_arg_checks(
        &mut self,
        native: symphonyscript_core::NativeId,
        args: &[crate::semantics::analysis::ids::NodeId],
    ) -> Result<(), Internal> {
        use symphonyscript_core::{NativeType, Scalar};
        let params = self
            .vocabulary
            .native(native)
            .ok_or(Internal::missed("native function"))?
            .signature
            .params
            .clone();
        for (arg, param) in args.iter().zip(params) {
            let (ty, open) = match param {
                NativeType::Value(ty) => (ty, false),
                NativeType::Open(ty) => (ty, true),
                NativeType::CapacityOf(..) => continue,
            };
            if (ty.is_scalar() && !open) || ty.scalar == Scalar::Str {
                continue;
            }
            let found = self.known_layout(*arg)?;
            let Ok(Some(layout)) = found.shaped else {
                continue;
            };
            let (rows, length) = if open || ty.capacity.is_some() {
                (ty.lengths.clone(), None)
            } else {
                match ty.lengths.split_last() {
                    Some((length, rows)) => (rows.to_vec(), Some(*length)),
                    None => (Vec::new(), None),
                }
            };
            let loc = self.node(*arg)?.loc;
            let scope = self.node(*arg)?.scope;
            if layout.inner() != rows.as_slice() {
                self.error(
                    Code::TypeMismatch,
                    loc,
                    scope,
                    "the argument's rows differ from the parameter's type",
                )?;
                continue;
            }
            if let Some(length) = length {
                if layout.variable {
                    self.error(
                        Code::VariableArray,
                        loc,
                        scope,
                        format!(
                            "this argument is a variable array, and the parameter takes a fixed array of {length} elements"
                        ),
                    )?;
                } else if let Some(held) = layout.outer()
                    && held != length
                {
                    self.error(
                        Code::TypeMismatch,
                        loc,
                        scope,
                        format!(
                            "this argument holds {held} elements, and the parameter takes {length}"
                        ),
                    )?;
                }
                continue;
            }
            if let Some(capacity) = ty.capacity {
                let holds = match self.extent(*arg)? {
                    Some(held) => Some(held),
                    None => self.fixed_length(*arg)?,
                };
                if let Some(holds) = holds
                    && holds > capacity
                    && holds < LENGTH_LIMIT
                {
                    self.error(
                        Code::CapacityExceeded,
                        loc,
                        scope,
                        format!(
                            "this argument holds up to {holds} elements, more than the parameter's capacity {capacity}"
                        ),
                    )?;
                }
            }
        }
        Ok(())
    }
}
