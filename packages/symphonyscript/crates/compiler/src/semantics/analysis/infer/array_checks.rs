use super::position::Position;
use super::role::Role;
use crate::diagnostics::Code;
use crate::semantics::Internal;
use crate::semantics::analysis::binding_kind::BindingKind;
use crate::semantics::analysis::bound::Bound;
use crate::semantics::analysis::engine::Engine;
use crate::semantics::analysis::ids::NodeId;
use crate::semantics::analysis::kind::Kind;
use crate::semantics::analysis::op::Op;
use crate::semantics::analysis::resolve::LENGTH_LIMIT;
use crate::semantics::analysis::sig::Sig;
use crate::semantics::analysis::val::Val;

pub(super) enum ArrayPosition {
    Usual,
    Faulty,
    Variable,
}

impl Engine<'_> {
    pub(super) fn refused_cell(&mut self, node: NodeId) -> Result<bool, Internal> {
        let Op::Read(binding) = self.node(self.through_uses(node)?)?.op else {
            return Ok(false);
        };
        let record = self.binding(binding)?.clone();
        if matches!(
            record.kind,
            BindingKind::Input { .. } | BindingKind::Attr { .. }
        ) {
            return Ok(false);
        }
        let init = self.demand_sig(record.init)?;
        Ok(init.is_error()
            || record.annotation.is_none() && matches!(init.kind, Kind::Str | Kind::Node))
    }

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
            let variable = if self.definite_array(node)? {
                self.is_variable(node)?
            } else if position.ty.lengths.is_empty() {
                false
            } else {
                let found = self.known_layout(node)?;
                found.certain && matches!(found.shaped, Ok(Some(layout)) if layout.variable)
            };
            if variable {
                if position.ty.lengths.is_empty() {
                    let (code, message) = self.single_value_fault(position, "a variable array")?;
                    self.error(code, position.loc, position.scope, message)?;
                } else {
                    let help = self.fixed_help(&position.ty)?;
                    self.error_helped(
                        Code::VariableArray,
                        position.loc,
                        position.scope,
                        "this value is a variable array, and a fixed array is expected here".into(),
                        help,
                    )?;
                }
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
            Val::Scalar(node) => {
                if !self.demand_sig(*node)?.is_error() {
                    self.error(
                        Code::TypeMismatch,
                        position.loc,
                        position.scope,
                        "this attribute holds a variable array, and the value is a single value",
                    )?;
                }
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

    pub(in crate::semantics::analysis) fn capacity_help(capacity: u32) -> String {
        if capacity == 1 {
            return String::from("slice the value to its newest element with `[^1..]`");
        }
        format!("slice the value to its newest {capacity} elements with `[^{capacity}..]`")
    }

    pub(in crate::semantics::analysis) fn capacity_fix(
        &self,
        value: crate::semantics::analysis::ids::NodeId,
        (capacity, holds): (u32, u32),
        annotated: bool,
    ) -> Result<String, Internal> {
        if !self.is_string_bytes(value)? {
            return Ok(Self::capacity_help(capacity));
        }
        Ok(Self::string_capacity_help(capacity, holds, annotated))
    }

    pub(in crate::semantics::analysis) fn string_capacity_help(
        capacity: u32,
        holds: u32,
        annotated: bool,
    ) -> String {
        let bytes = if capacity == 1 { "byte" } else { "bytes" };
        if annotated {
            format!(
                "write a string of at most {capacity} {bytes}, or annotate a capacity of at least {holds}, as in `u8[..{holds}]`"
            )
        } else {
            format!("write a string of at most {capacity} {bytes}")
        }
    }

    fn is_string_bytes(
        &self,
        node: crate::semantics::analysis::ids::NodeId,
    ) -> Result<bool, Internal> {
        use crate::semantics::analysis::kind::Kind;
        use crate::semantics::analysis::op::Op;
        use symphonyscript_core::Builtin;
        let mut leaves = vec![node];
        while let Some(leaf) = leaves.pop() {
            match &self.node(leaf)?.op {
                Op::Use(inner) | Op::Chosen(inner) => leaves.push(*inner),
                Op::If(_, then, otherwise) => leaves.extend([*then, *otherwise]),
                Op::Match {
                    arms, otherwise, ..
                } => leaves.extend(arms.iter().map(|(_, value)| *value).chain(*otherwise)),
                Op::Builtin(Builtin::Bytes, operands) => {
                    let [operand] = operands.as_slice() else {
                        return Ok(false);
                    };
                    if !self
                        .memo
                        .sigs
                        .get(operand)
                        .is_some_and(|sig| sig.kind == Kind::Str)
                    {
                        return Ok(false);
                    }
                }
                _ => return Ok(false),
            }
        }
        Ok(true)
    }

    fn fixed_help(&self, ty: &symphonyscript_core::Ty) -> Result<String, Internal> {
        let mut form = self.scalar_name(ty.scalar)?;
        for length in &ty.lengths {
            form.push_str(&format!("[{length}]"));
        }
        Ok(format!(
            "pass a fixed `{form}` here, such as an input annotated `{form}`"
        ))
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
                let help = match &position.val {
                    Val::Scalar(node) => self.capacity_fix(*node, (capacity, holds), false)?,
                    Val::Array(_) | Val::Error => Self::capacity_help(capacity),
                };
                self.error_helped(
                    Code::CapacityExceeded,
                    position.loc,
                    position.scope,
                    format!(
                        "this value holds up to {holds} elements, more than the attribute's capacity {capacity}"
                    ),
                    help,
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
    fn bound_drew_error(&self, items: &[Bound]) -> Result<bool, Internal> {
        for item in items {
            let node = match item {
                Bound::Value(node) => *node,
                Bound::Cell(binding) => self.binding(*binding)?.init,
            };
            if self.memo.sigs.get(&node).is_some_and(Sig::is_error) {
                return Ok(true);
            }
        }
        Ok(false)
    }

    pub(super) fn state_checks(&mut self) -> Result<(), Internal> {
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
                    let name = self
                        .hir
                        .state(state)?
                        .name
                        .as_ref()
                        .map_or(String::new(), |name| name.text.clone());
                    self.error_at_node_helped(
                        Code::HistoryDepth,
                        node,
                        Vec::new(),
                        format!(
                            "this history read reaches {offset} {}, deeper than `[{limit}]`; `[{deepest}]` covers every read of this state",
                            if offset == 1 { "step back" } else { "steps back" }
                        ),
                        format!("declare the state as `state[{deepest}] {name}`, which covers every read of it"),
                    )?;
                }
                if let HistoryCell::Let(id) = cell {
                    let global = self.global_of_let(id);
                    let bound = self
                        .memo
                        .lets
                        .get(&(global, crate::semantics::analysis::let_ref::LetRef::Top(id)))
                        .cloned();
                    let items = match &bound {
                        Some(BoundVal::Scalar(item)) => vec![*item],
                        Some(BoundVal::Array(array)) => array.1.clone(),
                        Some(BoundVal::Error) => continue,
                        None => Vec::new(),
                    };
                    if self.bound_drew_error(&items)? {
                        continue;
                    }
                    let reactive = items.iter().any(|item| matches!(item, Bound::Cell(_)));
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
        let mut let_cell = None;
        let cell_layout = match &bound {
            Some(BoundVal::Array(array)) => Some(
                crate::semantics::analysis::layout::Layout::fixed(array.0.lengths()),
            ),
            Some(BoundVal::Scalar(Bound::Cell(binding))) => {
                if matches!(cell, HistoryCell::Let(_)) {
                    let_cell = Some(*binding);
                }
                match self.memo.binding_layouts.get(binding) {
                    Some(layout) => Some(layout.clone()),
                    None if let_cell.is_some() => {
                        let init = self.binding(*binding)?.init;
                        let Ok(layout) = self.known_layout(init)?.shaped else {
                            return Ok(());
                        };
                        layout
                    }
                    None => None,
                }
            }
            Some(BoundVal::Scalar(Bound::Value(_)) | BoundVal::Error) | None => return Ok(()),
        };
        let found = self.known_layout(fallback)?;
        let Ok(fallback_layout) = found.shaped else {
            return Ok(());
        };
        if let (Some(binding), Some(cell), Some(held)) = (let_cell, &cell_layout, &fallback_layout)
            && cell.variable
            && !held.variable
            && held.inner() == cell.inner()
            && let Some(length) = held.outer()
        {
            let init = self.binding(binding)?.init;
            let capacity = match self.memo.binding_capacities.get(&binding) {
                Some(capacity) => Some(*capacity),
                None => self.extent(init)?,
            };
            if let Some(capacity) = capacity
                && length > capacity
            {
                let name = self.binding(binding)?.name.clone();
                self.error_at_node(
                    Code::CapacityExceeded,
                    fallback,
                    format!(
                        "the fallback holds {length} elements, more than the capacity {capacity} of `{name}`"
                    ),
                )?;
                return Ok(());
            }
        }
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
                let help = self.capacity_fix(record.init, (capacity, holds), true)?;
                self.error_helped(
                    Code::CapacityExceeded,
                    record.loc,
                    scope,
                    format!(
                        "`{}` holds up to {holds} elements, more than its annotated capacity {capacity}",
                        record.name
                    ),
                    help,
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
                    let help = self.fixed_help(&ty)?;
                    self.error_helped(
                        Code::VariableArray,
                        loc,
                        scope,
                        format!(
                            "this argument is a variable array, and the parameter takes a fixed array of {length} elements"
                        ),
                        help,
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
                self.memo.native_capacities.push((*arg, capacity));
            }
        }
        Ok(())
    }

    pub(super) fn native_capacity_checks(&mut self) -> Result<(), Internal> {
        for (arg, capacity) in std::mem::take(&mut self.memo.native_capacities) {
            let holds = match self.extent(arg)? {
                Some(held) => Some(held),
                None => self.fixed_length(arg)?,
            };
            if let Some(holds) = holds
                && holds > capacity
                && holds < LENGTH_LIMIT
            {
                let help = self.capacity_fix(arg, (capacity, holds), false)?;
                let node = self.node(arg)?;
                let (loc, scope) = (node.loc, node.scope);
                self.error_helped(
                    Code::CapacityExceeded,
                    loc,
                    scope,
                    format!(
                        "this argument holds up to {holds} elements, more than the parameter's capacity {capacity}"
                    ),
                    help,
                )?;
            }
        }
        Ok(())
    }
}
