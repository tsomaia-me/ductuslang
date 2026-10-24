use symphonyscript_core::Scalar;

use crate::semantics::Internal;
use crate::semantics::analysis::combined_shape::combined_shape;
use crate::semantics::analysis::engine::Engine;
use crate::semantics::analysis::eval_ty_of::eval_ty_of;
use crate::semantics::analysis::fast_hash::FastMap;
use crate::semantics::analysis::ids::NodeId;
use crate::semantics::analysis::lengths_of::lengths_of;
use crate::semantics::analysis::shape_need::ShapeNeed;
use crate::semantics::analysis::shape_parts::shape_parts;
use crate::semantics::analysis::sig::Sig;
use crate::semantics::ir::Loc;

impl Engine<'_> {
    pub(super) fn reactive_shapes(&mut self) -> Result<(), Internal> {
        let mut needs: Vec<(NodeId, ShapeNeed)> = self
            .memo
            .shapes
            .iter()
            .filter(|(node, _)| self.memo.sigs.get(node).is_some_and(|sig| sig.reactive))
            .map(|(node, need)| (*node, need.clone()))
            .collect();
        if needs.is_empty() {
            return Ok(());
        }
        needs.sort_by_key(|(node, _)| *node);
        let mut types: FastMap<NodeId, Vec<Option<Scalar>>> = FastMap::default();
        for (node, ty) in &self.typing.checked {
            types.entry(*node).or_default().push(*ty);
        }
        for (node, need) in needs {
            if self.memo.defaulted.contains_key(&node) {
                self.defaulted_uses(node, &need)?;
                continue;
            }
            for ty in types.remove(&node).unwrap_or_default() {
                if self
                    .shape_at(node, ty)?
                    .is_some_and(|lengths| lengths != need.lengths)
                {
                    self.error_at_node(need.code, node, need.message)?;
                    break;
                }
            }
        }
        Ok(())
    }

    fn defaulted_uses(&mut self, node: NodeId, need: &ShapeNeed) -> Result<(), Internal> {
        let elements = self.memo.defaulted.get(&node).cloned().unwrap_or_default();
        let Some(first) = elements.first().copied() else {
            return Ok(());
        };
        let (name, site) = {
            let binding = self.binding(first)?;
            (binding.name.clone(), binding.loc)
        };
        let mut uses: Vec<(Scalar, Loc)> = elements
            .iter()
            .flat_map(|element| {
                self.typing
                    .expectations
                    .get(element)
                    .cloned()
                    .unwrap_or_default()
            })
            .collect();
        uses.sort_by_key(|(_, loc)| (loc.file, loc.start, loc.end));
        for (ty, loc) in uses {
            if self
                .shape_at(node, Some(ty))?
                .is_some_and(|lengths| lengths != need.lengths)
            {
                let scope = self.node(node)?.scope;
                self.error_with_chain(
                    need.code,
                    loc,
                    vec![site],
                    scope,
                    format!(
                        "`{name}` takes the shape its recursive call yields with the defaults, and this use yields another shape"
                    ),
                )?;
                return Ok(());
            }
        }
        Ok(())
    }

    fn shape_at(&mut self, root: NodeId, ty: Option<Scalar>) -> Result<Option<Vec<u32>>, Internal> {
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
                let value = self.demand_cval(node, eval_ty_of(ty))?;
                let shape = (!value.is_error()).then(|| lengths_of(&value));
                self.typing.shapes.insert((node, ty), shape);
                continue;
            }
            let op = self.node(node)?.op.clone();
            let parts = shape_parts(self, &op)?;
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
            let shape = match shapes.map(|shapes| combined_shape(node, &op, &shapes)) {
                Some(Ok(lengths)) => Some(lengths),
                Some(Err(fault)) => {
                    self.error_at_node(fault.code, fault.node, fault.message)?;
                    None
                }
                None => None,
            };
            self.typing.shapes.insert((node, ty), shape);
        }
        Ok(self.typing.shapes.get(&(root, ty)).cloned().flatten())
    }
}
