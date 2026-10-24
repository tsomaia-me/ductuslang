use symphonyscript_core::Scalar;

use crate::diagnostics::Code;
use crate::semantics::analysis::binding_kind::BindingKind;
use crate::semantics::analysis::engine::Engine;
use crate::semantics::analysis::eval_ty_of::eval_ty_of;
use crate::semantics::analysis::fast_hash::FastMap;
use crate::semantics::analysis::ids::{BindingId, NodeId};
use crate::semantics::analysis::lengths_of::lengths_of;
use crate::semantics::analysis::op::Op;
use crate::semantics::analysis::shape_need::ShapeNeed;
use crate::semantics::analysis::sig::Sig;

impl Engine<'_> {
    pub(super) fn reactive_shapes(&mut self) {
        let mut needs: Vec<(NodeId, ShapeNeed)> = self
            .memo
            .shapes
            .iter()
            .filter(|(node, _)| self.memo.sigs.get(node).is_some_and(|sig| sig.reactive))
            .map(|(node, need)| (*node, need.clone()))
            .collect();
        if needs.is_empty() {
            return;
        }
        needs.sort_by_key(|(node, _)| *node);
        let mut types: FastMap<NodeId, Vec<Option<Scalar>>> = FastMap::default();
        for (node, ty) in &self.typing.checked {
            types.entry(*node).or_default().push(*ty);
        }
        for (node, need) in needs {
            for ty in types.remove(&node).unwrap_or_default() {
                if self
                    .shape_at(node, ty)
                    .is_some_and(|lengths| lengths != need.lengths)
                {
                    self.error_at_node(need.code, node, need.message);
                    break;
                }
            }
        }
    }

    pub(super) fn reactive_array_bindings(&mut self) {
        for index in 0..self.bindings.len() {
            let binding = BindingId(u32::try_from(index).unwrap_or(u32::MAX));
            let record = self.binding(binding).clone();
            if !matches!(record.kind, BindingKind::Let | BindingKind::Param)
                || !self.arrays.contains(&record.init)
                || !self
                    .memo
                    .sigs
                    .get(&record.init)
                    .is_some_and(|sig| sig.reactive)
            {
                continue;
            }
            let own = self.typing.own.get(&binding).copied();
            if self
                .shape_at(record.init, own)
                .is_some_and(|lengths| !lengths.is_empty())
            {
                self.typing.held.insert(binding);
                let scope = self.node(record.init).scope;
                self.error(
                    Code::TypeMismatch,
                    record.loc,
                    scope,
                    format!(
                        "`{}` reads a cell and holds an array a recursive call yields, which a reactive value does not hold",
                        record.name
                    ),
                );
            }
        }
    }

    fn shape_at(&mut self, root: NodeId, ty: Option<Scalar>) -> Option<Vec<u32>> {
        let mut stack = vec![(root, false)];
        while let Some((node, ready)) = stack.pop() {
            if self.typing.shapes.contains_key(&(node, ty)) {
                continue;
            }
            let sig = self.memo.sigs.get(&node).copied().unwrap_or(Sig::ERROR);
            if sig.is_error() {
                self.typing.shapes.insert((node, ty), None);
                continue;
            }
            if !sig.reactive {
                let value = self.demand_cval(node, eval_ty_of(ty));
                let shape = (!value.is_error()).then(|| lengths_of(&value));
                self.typing.shapes.insert((node, ty), shape);
                continue;
            }
            let op = self.node(node).op.clone();
            if let Op::Read(binding) = op
                && self.typing.held.contains(&binding)
            {
                self.typing.shapes.insert((node, ty), None);
                continue;
            }
            let parts = shape_parts(self, &op);
            if !self.arrays.contains(&node) || parts.is_empty() {
                self.typing.shapes.insert((node, ty), Some(Vec::new()));
                continue;
            }
            if !ready {
                stack.push((node, true));
                for part in &parts {
                    if !self.typing.shapes.contains_key(&(*part, ty)) {
                        stack.push((*part, false));
                    }
                }
                continue;
            }
            let shapes: Option<Vec<Vec<u32>>> = parts
                .iter()
                .map(|part| self.typing.shapes.get(&(*part, ty)).cloned().flatten())
                .collect();
            let shape = shapes.and_then(|shapes| self.combined_shape(node, &op, &shapes));
            self.typing.shapes.insert((node, ty), shape);
        }
        self.typing.shapes.get(&(root, ty)).cloned().flatten()
    }

    fn combined_shape(&mut self, node: NodeId, op: &Op, shapes: &[Vec<u32>]) -> Option<Vec<u32>> {
        let first = shapes.first().cloned().unwrap_or_default();
        let agree = shapes.iter().all(|shape| *shape == first);
        match op {
            Op::Element { base, .. } => {
                let mut lengths = first;
                if lengths.pop().is_none() {
                    self.error_at_node(Code::NotAnArray, *base, "only an array is indexed");
                    return None;
                }
                Some(lengths)
            }
            Op::If(..) | Op::Match { .. } => {
                if !agree {
                    self.error_at_node(
                        Code::TypeMismatch,
                        node,
                        "the branches hold arrays of different shapes",
                    );
                    return None;
                }
                Some(first)
            }
            Op::Array(elements) => {
                if !agree {
                    self.error_at_node(
                        Code::RaggedArray,
                        node,
                        "the elements of an array are arrays of one length, or all single values",
                    );
                    return None;
                }
                let mut lengths = first;
                lengths.push(u32::try_from(elements.len()).unwrap_or(u32::MAX));
                Some(lengths)
            }
            Op::Use(_)
            | Op::Annotate(..)
            | Op::Number(..)
            | Op::Const(_)
            | Op::Bool(_)
            | Op::Str(_)
            | Op::Concat(_)
            | Op::Variant(..)
            | Op::Bare(_)
            | Op::Pi
            | Op::NodeRef(_)
            | Op::Read(_)
            | Op::Arg(_)
            | Op::Unary(..)
            | Op::Binary(..)
            | Op::Pick { .. }
            | Op::Convert(..)
            | Op::Builtin(..)
            | Op::Recursive { .. }
            | Op::ToRead { .. } => Some(first),
        }
    }
}

fn shape_parts(engine: &Engine<'_>, op: &Op) -> Vec<NodeId> {
    match op {
        Op::Read(binding) => vec![engine.binding(*binding).init],
        Op::Use(operand) | Op::Annotate(_, operand) => vec![*operand],
        Op::Element { base, .. } => vec![*base],
        Op::If(_, then, otherwise) => vec![*then, *otherwise],
        Op::Match {
            arms, otherwise, ..
        } => {
            let mut parts: Vec<NodeId> = arms.iter().map(|(_, value)| *value).collect();
            parts.extend(otherwise.iter().copied());
            parts
        }
        Op::Array(elements) => elements.clone(),
        Op::Number(..)
        | Op::Const(_)
        | Op::Bool(_)
        | Op::Str(_)
        | Op::Concat(_)
        | Op::Variant(..)
        | Op::Bare(_)
        | Op::Pi
        | Op::NodeRef(_)
        | Op::Arg(_)
        | Op::Unary(..)
        | Op::Binary(..)
        | Op::Pick { .. }
        | Op::Convert(..)
        | Op::Builtin(..)
        | Op::Recursive { .. }
        | Op::ToRead { .. } => Vec::new(),
    }
}
