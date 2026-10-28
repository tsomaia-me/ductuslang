use symphonyscript_core::{BinaryOp, Builtin, Scalar, UnaryOp};

use crate::diagnostics::Code;
use crate::semantics::Internal;
use crate::semantics::analysis::c_val::CVal;
use crate::semantics::analysis::engine::Engine;
use crate::semantics::analysis::eval_ty::EvalTy;
use crate::semantics::analysis::eval_ty_of::eval_ty_of;
use crate::semantics::analysis::evaluation::own_type;
use crate::semantics::analysis::ids::NodeId;
use crate::semantics::analysis::is_float::is_float;
use crate::semantics::analysis::is_integer::is_integer;
use crate::semantics::analysis::kind::Kind;
use crate::semantics::analysis::op::Op;
use crate::semantics::analysis::outside_i64::outside_i64;
use crate::semantics::analysis::sig::Sig;
use crate::semantics::analysis::widens::widens;
use crate::semantics::analysis::width::Width;
use crate::semantics::ir::Suffix;

impl Engine<'_> {
    pub(super) fn check(
        &mut self,
        root: NodeId,
        typed: (Option<Scalar>, bool),
        condition: bool,
        roots: &mut Vec<(NodeId, Option<Scalar>)>,
    ) -> Result<(), Internal> {
        self.check_from(root, typed, (condition, Selection::Chosen), roots)
    }

    pub(super) fn check_detached(
        &mut self,
        root: NodeId,
        typed: (Option<Scalar>, bool),
        passed_over: bool,
        roots: &mut Vec<(NodeId, Option<Scalar>)>,
    ) -> Result<(), Internal> {
        let selection = if passed_over {
            Selection::PassedOver
        } else {
            Selection::Chosen
        };
        self.check_from(root, typed, (false, selection), roots)
    }

    fn check_from(
        &mut self,
        root: NodeId,
        (expected, flows): (Option<Scalar>, bool),
        (condition, selection): (bool, Selection),
        roots: &mut Vec<(NodeId, Option<Scalar>)>,
    ) -> Result<(), Internal> {
        let mut stack = vec![(root, expected, selection, flows)];
        let mut condition_root = condition;
        while let Some((node, expected, selection, flows)) = stack.pop() {
            #[cfg(test)]
            {
                self.typing.checks += 1;
            }
            let fresh = match selection {
                Selection::Chosen => self.typing.checked.insert((node, expected, flows)),
                Selection::Evaluated => self.typing.evaluated.insert((node, expected, flows)),
                Selection::PassedOver => self.typing.passed_over.insert((node, expected, flows)),
            };
            if !fresh {
                continue;
            }
            let sig = self.known_sig(node)?;
            let placeholder = sig.is_error();
            if placeholder && !self.holds_placeholder(node)? {
                continue;
            }
            let Some((sig, group_of)) = self.grouped_sig(node, sig)? else {
                continue;
            };
            let mut expected = match (sig.width, expected) {
                (Width::Fixed(own), Some(target))
                    if !flows
                        && !sig.reactive
                        && !sig.frame
                        && own != target
                        && widens(own, target) =>
                {
                    Some(own)
                }
                _ => expected,
            };
            let mut flows = flows && expected.is_some() && !placeholder;
            let as_condition = std::mem::take(&mut condition_root);
            if selection != Selection::Evaluated && !placeholder {
                if let Some(target) = expected
                    && !self.compatible(node, &sig, target, as_condition)?
                {
                    if matches!(
                        self.carried_kind(node, sig.kind)?,
                        Kind::Enum(_) | Kind::Bare
                    ) {
                        continue;
                    }
                    expected = None;
                    flows = false;
                }
                if let (Some(Scalar::Enum(id)), Op::Bare(name)) = (expected, &self.node(node)?.op)
                    && self.variants.position(id, name)?.is_none()
                    && !self.in_name_probe(self.node(node)?.scope)?
                {
                    let message = format!(
                        "`{name}` is not a variant of `{}`",
                        self.enum_decl(id)?.name
                    );
                    self.error_at_node(Code::UnknownName, node, message)?;
                }
            }
            let selection = match (sig.reactive || sig.frame || placeholder, selection) {
                (false, Selection::Chosen) => {
                    roots.push((node, expected));
                    Selection::Evaluated
                }
                (true, _) | (false, Selection::Evaluated | Selection::PassedOver) => selection,
            };
            let ty = match expected {
                _ if placeholder => None,
                Some(expected) => Some(expected),
                None => self.synth(node)?,
            };
            if let Some(group) = group_of
                && self.typing.passed_over_groups.insert((group, ty))
                && let Op::Array(elements) = &self.node(group)?.op
            {
                stack.extend(
                    elements
                        .iter()
                        .map(|element| (*element, ty, Selection::PassedOver, flows)),
                );
            }
            let op = self.node(node)?.op.clone();
            let passed_over = Selection::PassedOver;
            match op {
                Op::Read(_) | Op::ToRead { .. } | Op::Concat(_) => {}
                Op::Use(inner) | Op::Chosen(inner) => stack.push((inner, ty, selection, flows)),
                Op::Unary(UnaryOp::Neg, operand) => stack.push((operand, ty, selection, flows)),
                Op::Unary(UnaryOp::Not, operand) => {
                    stack.push((operand, Some(Scalar::Bool), selection, true));
                }
                Op::Binary(op, lhs, rhs) => match op {
                    BinaryOp::And | BinaryOp::Or => {
                        stack.push((lhs, Some(Scalar::Bool), selection, true));
                        stack.push((rhs, Some(Scalar::Bool), selection, true));
                    }
                    BinaryOp::Lt
                    | BinaryOp::Le
                    | BinaryOp::Gt
                    | BinaryOp::Ge
                    | BinaryOp::Is
                    | BinaryOp::IsNot => {
                        if matches!(op, BinaryOp::Is | BinaryOp::IsNot)
                            && selection != Selection::Evaluated
                            && self.crossed_enums(node, lhs, rhs)?
                        {
                            continue;
                        }
                        let left = self.synth(lhs)?;
                        let right = self.synth(rhs)?;
                        let pair = self.pair(lhs, rhs)?;
                        stack.push((lhs, compared(left, pair), selection, false));
                        stack.push((rhs, compared(right, pair), selection, false));
                    }
                    BinaryOp::IntDiv if ty.is_some_and(is_float) => {
                        self.synth(lhs)?;
                        self.synth(rhs)?;
                        let inner = self.combine(&[lhs, rhs])?;
                        stack.push((lhs, inner, selection, false));
                        stack.push((rhs, inner, selection, false));
                    }
                    BinaryOp::Add
                    | BinaryOp::Sub
                    | BinaryOp::Mul
                    | BinaryOp::Div
                    | BinaryOp::IntDiv
                    | BinaryOp::Rem
                    | BinaryOp::Pow => {
                        stack.push((lhs, ty, selection, flows));
                        stack.push((rhs, ty, selection, flows));
                    }
                },
                Op::If(condition, then, otherwise) => {
                    stack.push((condition, Some(Scalar::Bool), selection, true));
                    let chosen = match selection {
                        Selection::Chosen | Selection::Evaluated => {
                            self.chosen(condition, EvalTy::Bool)?
                        }
                        Selection::PassedOver => None,
                    };
                    match chosen {
                        Some(CVal::Bool(true)) => {
                            stack.push((otherwise, ty, passed_over, flows));
                            stack.push((then, ty, selection, flows));
                        }
                        Some(CVal::Bool(false)) => {
                            stack.push((then, ty, passed_over, flows));
                            stack.push((otherwise, ty, selection, flows));
                        }
                        Some(_) => {}
                        None => {
                            stack.push((then, ty, selection, flows));
                            stack.push((otherwise, ty, selection, flows));
                        }
                    }
                }
                Op::Match {
                    scrutinee,
                    arms,
                    otherwise,
                } => {
                    let scrutinee_ty = self.synth(scrutinee)?;
                    stack.push((scrutinee, scrutinee_ty, selection, false));
                    let scrutinee_sig = self.known_sig(scrutinee)?;
                    if scrutinee_sig.kind == Kind::Int {
                        for (label, _) in &arms {
                            self.label_fit(*label, scrutinee_ty)?;
                        }
                    }
                    let chosen = match (selection, self.scrutinee_ty(scrutinee, &scrutinee_sig)) {
                        (Selection::Chosen | Selection::Evaluated, Some(label_ty)) => self
                            .chosen(scrutinee, label_ty)?
                            .map(|value| (value, label_ty)),
                        (Selection::Chosen | Selection::Evaluated | Selection::PassedOver, _) => {
                            None
                        }
                    };
                    let values = arms.iter().map(|(_, value)| *value).chain(otherwise);
                    match chosen {
                        Some((CVal::Error, _)) => {}
                        Some((value, label_ty)) => {
                            let mut chosen = otherwise;
                            for (label, arm) in &arms {
                                if self.demand_cval(*label, label_ty)? == value {
                                    chosen = Some(*arm);
                                    break;
                                }
                            }
                            for value in values {
                                if Some(value) != chosen {
                                    stack.push((value, ty, passed_over, flows));
                                }
                            }
                            stack.extend(chosen.map(|arm| (arm, ty, selection, flows)));
                        }
                        None => stack.extend(values.map(|value| (value, ty, selection, flows))),
                    }
                }
                Op::Pick { index, items } => {
                    let index_ty = self.synth(index)?;
                    stack.push((index, index_ty, selection, false));
                    let position_ty = own_type(&self.known_sig(index)?, EvalTy::Exact);
                    let chosen = match selection {
                        Selection::Chosen | Selection::Evaluated => {
                            self.chosen(index, position_ty)?
                        }
                        Selection::PassedOver => None,
                    };
                    match chosen {
                        Some(CVal::Int(position)) => {
                            let picked = usize::try_from(position)
                                .ok()
                                .filter(|position| *position < items.len());
                            for (row, item) in items.iter().enumerate() {
                                if Some(row) != picked {
                                    stack.push((*item, ty, passed_over, flows));
                                }
                            }
                            match picked.and_then(|position| items.get(position)) {
                                Some(item) => stack.push((*item, ty, selection, flows)),
                                None if selection == Selection::Evaluated => {}
                                None => self.index_outside(index, position, length_of(&items))?,
                            }
                        }
                        Some(_) => {}
                        None => {
                            stack.extend(items.into_iter().map(|item| (item, ty, selection, flows)))
                        }
                    }
                }
                Op::Element { base, index } => {
                    let index_ty = self.synth(index)?;
                    stack.push((index, index_ty, selection, false));
                    stack.push((base, ty, selection, flows));
                    if selection == Selection::Chosen {
                        self.fixed_cell_index(base, index)?;
                    }
                }
                Op::Array(elements) => {
                    stack.extend(
                        elements
                            .into_iter()
                            .map(|element| (element, ty, selection, flows)),
                    );
                }
                Op::Convert(target, operand) => {
                    let converts = is_float(target) && self.known_sig(operand)?.is_numeric();
                    let operand_ty = if converts {
                        Some(target)
                    } else {
                        self.synth(operand)?
                    };
                    stack.push((operand, operand_ty, selection, converts));
                }
                Op::Annotate(target, operand) => {
                    stack.push((operand, Some(target), selection, true))
                }
                Op::Builtin(builtin, args) => {
                    let operands = match builtin {
                        Builtin::Rand => Some(Scalar::I64),
                        Builtin::Abs
                        | Builtin::Min
                        | Builtin::Max
                        | Builtin::Clamp
                        | Builtin::Floor
                        | Builtin::Ceil
                        | Builtin::Round
                        | Builtin::Sqrt
                        | Builtin::Sin
                        | Builtin::Cos
                        | Builtin::Tan
                        | Builtin::Exp
                        | Builtin::Log
                        | Builtin::Lerp => ty,
                        Builtin::Len | Builtin::Bytes => {
                            for arg in args {
                                let arg_ty = self.synth(arg)?;
                                stack.push((arg, arg_ty, selection, false));
                            }
                            continue;
                        }
                    };
                    stack.extend(
                        args.into_iter()
                            .map(|arg| (arg, operands, selection, flows)),
                    );
                }
                Op::Join(lhs, rhs) => {
                    stack.push((lhs, ty, selection, flows));
                    stack.push((rhs, ty, selection, flows));
                }
                Op::Slice { base, lower, upper } => {
                    for (bound, _) in lower.iter().chain(upper.iter()) {
                        let bound_ty = self.synth(*bound)?;
                        stack.push((*bound, bound_ty, selection, false));
                        if selection != Selection::Chosen {
                            continue;
                        }
                        let position_ty = own_type(&self.known_sig(*bound)?, EvalTy::Exact);
                        if let Some(CVal::Int(value)) = self.chosen(*bound, position_ty)?
                            && i64::try_from(value).is_err()
                        {
                            self.error_at_node(Code::IndexOutOfRange, *bound, outside_i64(value))?;
                        }
                    }
                    stack.push((base, ty, selection, flows));
                }
                Op::History { fallback, .. } => {
                    if let Some(fallback) = fallback {
                        let cell_ty = self.history_type(node)?.or(ty);
                        stack.push((fallback, cell_ty, selection, true));
                    }
                }
                Op::Native { native, args } => {
                    self.native_arg_checks(native, &args)?;
                    let params = self.native_params(native)?;
                    let arg_selection = match selection {
                        Selection::Evaluated => Selection::Chosen,
                        Selection::Chosen | Selection::PassedOver => selection,
                    };
                    for (arg, param) in args.into_iter().zip(params) {
                        for leaf in self.native_argument(arg, param)?.unwrap_or_default() {
                            stack.push((leaf, param, arg_selection, true));
                        }
                    }
                }
                other => {
                    for operand in other.operands() {
                        let operand_ty = self.synth(operand)?;
                        stack.push((operand, operand_ty, selection, false));
                    }
                }
            }
        }
        Ok(())
    }

    pub(super) fn history_type(&self, node: NodeId) -> Result<Option<Scalar>, Internal> {
        Ok(self
            .history_bindings(node)?
            .first()
            .and_then(|binding| self.typing.own.get(binding).copied()))
    }

    pub(super) fn native_params(
        &self,
        native: symphonyscript_core::NativeId,
    ) -> Result<Vec<Option<Scalar>>, Internal> {
        let decl = self
            .vocabulary
            .native(native)
            .ok_or(Internal::missed("native function"))?;
        Ok(decl
            .signature
            .params
            .iter()
            .map(|param| Some(crate::semantics::analysis::evaluation::native_ty(param).scalar))
            .collect())
    }

    pub(in crate::semantics::analysis) fn native_argument(
        &mut self,
        arg: NodeId,
        param: Option<Scalar>,
    ) -> Result<Option<Vec<NodeId>>, Internal> {
        if !matches!(self.node(arg)?.op, Op::Array(_)) {
            return Ok(Some(vec![arg]));
        }
        let sig = self.known_sig(arg)?;
        if sig.is_error() {
            return Ok(None);
        }
        if let Some(param) = param
            && !self.compatible(arg, &sig, param, false)?
        {
            return Ok(None);
        }
        let mut leaves = Vec::new();
        let mut pending = vec![arg];
        while let Some(node) = pending.pop() {
            match &self.node(node)?.op {
                Op::Array(elements) => pending.extend(elements.iter().rev()),
                _ => leaves.push(node),
            }
        }
        Ok(Some(leaves))
    }

    fn grouped_sig(
        &self,
        node: NodeId,
        sig: Sig,
    ) -> Result<Option<(Sig, Option<NodeId>)>, Internal> {
        if sig.is_error() {
            return Ok(Some((sig, None)));
        }
        let Some(group) = self.memo.groups.get(&node).copied() else {
            return Ok(Some((sig, None)));
        };
        let group_sig = self.known_sig(group)?;
        if group_sig.is_error() {
            return Ok(None);
        }
        if sig.reactive && sig.is_numeric() && group_sig.is_numeric() {
            return Ok(Some((sig, None)));
        }
        Ok(Some((
            Sig {
                kind: group_sig.kind,
                ..sig
            },
            Some(group),
        )))
    }

    pub(super) fn refuses(&mut self, node: NodeId, expected: Scalar) -> Result<bool, Internal> {
        let sig = self.known_sig(node)?;
        if sig.is_error() {
            return Ok(false);
        }
        let Some((sig, _)) = self.grouped_sig(node, sig)? else {
            return Ok(false);
        };
        Ok(self.incompatibility(node, &sig, expected, false)?.is_some())
    }

    fn holds_placeholder(&self, node: NodeId) -> Result<bool, Internal> {
        Ok(self
            .node(node)?
            .op
            .operands()
            .iter()
            .any(|operand| self.memo.sigs.get(operand).is_some_and(Sig::is_error)))
    }

    fn chosen(&mut self, node: NodeId, ty: EvalTy) -> Result<Option<CVal>, Internal> {
        let sig = self.known_sig(node)?;
        if sig.reactive || sig.frame || sig.is_error() {
            return Ok(None);
        }
        Ok(Some(self.demand_cval(node, ty)?))
    }

    pub(super) fn destination_labels(&mut self, root: NodeId) -> Result<(), Internal> {
        let mut stack = vec![root];
        while let Some(node) = stack.pop() {
            match self.node(node)?.op.clone() {
                Op::If(_, then, otherwise) => stack.extend([then, otherwise]),
                Op::Pick { items, .. } => stack.extend(items),
                Op::Use(inner) | Op::Chosen(inner) | Op::Annotate(_, inner) => stack.push(inner),
                Op::Match {
                    scrutinee,
                    arms,
                    otherwise,
                } => {
                    if self.known_sig(scrutinee)?.kind == Kind::Int {
                        let scrutinee_ty = self.synth(scrutinee)?;
                        for (label, _) in &arms {
                            self.label_fit(*label, scrutinee_ty)?;
                        }
                    }
                    stack.extend(arms.iter().map(|(_, value)| *value).chain(otherwise));
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn label_fit(&mut self, label: NodeId, scrutinee: Option<Scalar>) -> Result<(), Internal> {
        let Op::Number(_, suffix) = self.node(label)?.op else {
            return Ok(());
        };
        let ty = match suffix {
            Suffix::Type(scalar) => scalar,
            Suffix::None | Suffix::Unit { .. } => scrutinee
                .filter(|scalar| is_integer(*scalar))
                .unwrap_or(Scalar::I64),
        };
        self.demand_cval(label, eval_ty_of(Some(ty)))?;
        Ok(())
    }

    fn pair(&mut self, lhs: NodeId, rhs: NodeId) -> Result<Option<Scalar>, Internal> {
        let (left, right) = (self.known_sig(lhs)?, self.known_sig(rhs)?);
        let (left_kind, right_kind) = (
            self.carried_kind(lhs, left.kind)?,
            self.carried_kind(rhs, right.kind)?,
        );
        Ok(match (left_kind, right_kind) {
            (Kind::Bool, _) => Some(Scalar::Bool),
            (Kind::Str, _) => Some(Scalar::Str),
            (Kind::Enum(id), _) | (_, Kind::Enum(id)) => Some(Scalar::Enum(id)),
            _ => self.combine(&[lhs, rhs])?,
        })
    }

    fn carried_kind(&self, node: NodeId, kind: Kind) -> Result<Kind, Internal> {
        if kind != Kind::Bare {
            return Ok(kind);
        }
        Ok(match self.carried_enum_of(node)? {
            Some(id) => Kind::Enum(id),
            None => kind,
        })
    }

    fn holds_a_float_cell(&mut self, node: NodeId) -> Result<bool, Internal> {
        let Op::Array(elements) = &self.node(node)?.op else {
            return Ok(false);
        };
        let mut pending = elements.clone();
        while let Some(element) = pending.pop() {
            match &self.node(self.through_uses(element)?)?.op {
                Op::Array(inner) => pending.extend(inner.iter().copied()),
                Op::Read(_) | Op::History { .. } => {
                    if self.known_sig(element)?.kind == Kind::Float {
                        return Ok(true);
                    }
                }
                _ => {}
            }
        }
        Ok(false)
    }

    pub(in crate::semantics::analysis) fn compatible(
        &mut self,
        node: NodeId,
        sig: &Sig,
        expected: Scalar,
        condition: bool,
    ) -> Result<bool, Internal> {
        let Some((code, message)) = self.incompatibility(node, sig, expected, condition)? else {
            return Ok(true);
        };
        self.error_at_node(code, node, message)?;
        Ok(false)
    }

    pub(super) fn incompatibility(
        &mut self,
        node: NodeId,
        sig: &Sig,
        expected: Scalar,
        condition: bool,
    ) -> Result<Option<(Code, String)>, Internal> {
        let is_cell = matches!(
            self.node(self.through_uses(node)?)?.op,
            Op::Read(_) | Op::History { .. }
        ) || matches!(sig.width, Width::Adaptive)
            && matches!(self.node(node)?.op, Op::ToRead { .. });
        let fits_kind = match (self.carried_kind(node, sig.kind)?, expected) {
            (Kind::Str, Scalar::Str) if sig.reactive => {
                return Ok(Some((
                    Code::ReactiveString,
                    "a string exists only at compile time, and this one reads a cell".into(),
                )));
            }
            (Kind::Bool, Scalar::Bool) | (Kind::Str, Scalar::Str) => true,
            (Kind::Enum(id), Scalar::Enum(other)) => id == other,
            (Kind::Bare, Scalar::Enum(_)) | (Kind::Any, _) => true,
            (Kind::Int, scalar) => is_integer(scalar) || is_float(scalar),
            (Kind::Float, scalar) if is_float(scalar) => true,
            (Kind::Float, scalar) if is_integer(scalar) => {
                if self.holds_a_float_cell(node)? {
                    return Ok(None);
                }
                return Ok(Some(if is_cell {
                    (
                        Code::FloatToInteger,
                        "a float cell is used where an integer is expected; a conversion states the rounding".into(),
                    )
                } else {
                    (
                        Code::TypeMismatch,
                        "a float value stands where an integer is expected; a conversion states the rounding".into(),
                    )
                }));
            }
            _ => false,
        };
        if !fits_kind && condition {
            return Ok(Some((
                Code::ConditionType,
                "a gate or a guard is a `bool`".into(),
            )));
        }
        if !fits_kind {
            return Ok(Some((
                Code::TypeMismatch,
                format!(
                    "this value has the wrong type here; `{}` is expected",
                    self.scalar_name(expected)?
                ),
            )));
        }
        if let Width::Fixed(scalar) | Width::Widened(scalar) = sig.width
            && !is_cell
            && (is_integer(scalar) || is_float(scalar))
            && !widens(scalar, expected)
        {
            return Ok(Some((
                Code::TypeMismatch,
                format!(
                    "a `{}` value does not widen into `{}`",
                    self.scalar_name(scalar)?,
                    self.scalar_name(expected)?
                ),
            )));
        }
        Ok(None)
    }

    pub(in crate::semantics::analysis) fn scalar_name(
        &self,
        scalar: Scalar,
    ) -> Result<String, Internal> {
        Ok(match scalar {
            Scalar::U8 => "u8".into(),
            Scalar::I32 => "i32".into(),
            Scalar::I64 => "i64".into(),
            Scalar::F32 => "f32".into(),
            Scalar::F64 => "f64".into(),
            Scalar::Bool => "bool".into(),
            Scalar::Str => "string".into(),
            Scalar::Enum(id) => self.enum_decl(id)?.name.clone(),
        })
    }

    fn fixed_cell_index(&mut self, base: NodeId, index: NodeId) -> Result<(), Internal> {
        if !self.known_sig(base)?.reactive || self.memo.projected_indexes.contains(&index) {
            return Ok(());
        }
        let found = self.known_layout(base)?;
        let (Ok(Some(layout)), true) = (found.shaped, found.certain) else {
            return Ok(());
        };
        let (false, Some(length)) = (layout.variable, layout.outer()) else {
            return Ok(());
        };
        let position_ty = own_type(&self.known_sig(index)?, EvalTy::Exact);
        if let Some(CVal::Int(position)) = self.chosen(index, position_ty)?
            && !(0..i128::from(length)).contains(&position)
        {
            self.index_outside(index, position, u64::from(length))?;
        }
        Ok(())
    }
}

fn length_of<T>(items: &[T]) -> u64 {
    u64::try_from(items.len()).unwrap_or(u64::MAX)
}

fn compared(own: Option<Scalar>, pair: Option<Scalar>) -> Option<Scalar> {
    let numeric = |scalar: Scalar| is_integer(scalar) || is_float(scalar);
    match (own, pair) {
        (Some(own), Some(pair)) if numeric(own) && numeric(pair) => Some(own),
        _ => pair,
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Selection {
    Chosen,
    Evaluated,
    PassedOver,
}
