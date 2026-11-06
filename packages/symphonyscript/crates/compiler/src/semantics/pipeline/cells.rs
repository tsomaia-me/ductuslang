use super::annotation::{Refusal, annotated_fact};
use super::core::{Core, Stage};
use super::decl_ref::DeclRef;
use super::fact::Fact;
use super::leaf::Leaf;
use super::values::flexible;
use crate::diagnostics::Code;
use crate::semantics::Internal;
use crate::semantics::ir::flat::{NONE, Op, TopRec};
use crate::semantics::operators::{EvalTy, Value, int_range};

impl Core<'_> {
    pub fn cell_root(&self, at: u32) -> Result<u32, Internal> {
        let rec = self
            .ir
            .cells
            .get(at as usize)
            .ok_or(Internal::missed("IR cell"))?;
        Ok(if rec.rule == NONE { rec.init } else { rec.rule })
    }

    pub fn cell_read_fact(&self, at: u32) -> Result<Fact, Internal> {
        if self.is_cyclic_cell(at) {
            return Ok(Fact::PLACEHOLDER);
        }
        if let Some(ty) = self.cell_annotation_ty(at)? {
            let mut fact = Fact::numeric(annotated_fact(ty).leaf, None, Some(ty));
            fact.dims = self.binder_dims(
                self.ir
                    .cells
                    .get(at as usize)
                    .map_or(NONE, |rec| rec.binder),
            )?;
            return Ok(fact);
        }
        let mut fact = self.fact(self.cell_root(at)?)?;
        if let Some(init) = self.open_member(at, &fact) {
            fact = self.fact(init)?;
        }
        if self.group_variable.contains(&at)
            && let Some(first) = fact.dims.first_mut()
        {
            *first = super::folds::ANY_LENGTH;
        }
        if !self.derived(at) && fact.fixed.is_none() && fact.read.is_none() {
            fact.read = match fact.leaf {
                Leaf::Int | Leaf::Empty => Some(EvalTy::I64),
                Leaf::Float | Leaf::Bool | Leaf::Placeholder | Leaf::Error | Leaf::Enum(_) => None,
            };
        }
        Ok(fact)
    }

    pub fn is_cyclic_cell(&self, at: u32) -> bool {
        self.cell_cyclic.get(at as usize).copied().unwrap_or(false)
    }

    pub fn derived(&self, at: u32) -> bool {
        self.ir
            .cells
            .get(at as usize)
            .is_some_and(|rec| rec.rule != NONE)
    }

    pub fn node_reactive(&mut self, node: u32) -> Result<bool, Internal> {
        let (op, _, _, _) = self.ir.node(node)?;
        if self.unbound_nodes.contains(&node) {
            return Ok(false);
        }
        if matches!(op, Op::Past | Op::Stepped) {
            return Ok(true);
        }
        if op == Op::Name {
            return Ok(match self.bound(node)? {
                DeclRef::Cell(_) => true,
                DeclRef::Let(at) => self.let_reads_cell(at)?,
                DeclRef::Param(position) => self.param_fact(node, position)?.1,
                DeclRef::None
                | DeclRef::Instance(_)
                | DeclRef::Duplicate
                | DeclRef::Fn(_)
                | DeclRef::Pattern(_) => false,
            });
        }
        if op == Op::Field {
            let Some(root) = self.read_root(node)? else {
                return Ok(self.to_choice(node).is_some());
            };
            let env = match self.path_read(node)?.cloned() {
                Some(read) => self.read_env(&read)?,
                None => NONE,
            };
            if env == NONE {
                return self.is_reactive(root);
            }
            let saved = self.enter_env(env);
            let reactive = self.is_reactive(root);
            self.leave_env(saved);
            return reactive;
        }
        if matches!(op, Op::Call | Op::NamedCall) {
            return self.call_reactive(node);
        }
        if op == Op::For
            && let Some(body) = self.body_of_call(node)
        {
            let rec = self.fold_rec(node)?;
            let outer = std::mem::replace(&mut self.inst, body);
            let reactive = self.is_reactive(rec.body);
            self.inst = outer;
            if reactive? {
                return Ok(true);
            }
        }
        if op == Op::If {
            let (_, condition, then, otherwise) = self.ir.node(node)?;
            let test = self.fact(condition)?;
            if test.leaf == Leaf::Bool && test.dims.is_empty() && !self.is_reactive(condition)? {
                let taken = match self.evaluate(condition, EvalTy::Bool)? {
                    Ok(Value::Bool(true)) => Some(then),
                    Ok(Value::Bool(false)) => Some(otherwise),
                    Ok(_) | Err(_) => None,
                };
                if let Some(arm) = taken {
                    return self.is_reactive(arm);
                }
            }
        }
        for operand in self.operands(node)? {
            if self.is_reactive(operand)? {
                return Ok(true);
            }
        }
        Ok(false)
    }

    pub fn let_reads_cell(&mut self, at: u32) -> Result<bool, Internal> {
        let root = self.let_root(at)?;
        if self.has_fact(root)? {
            return self.is_reactive(root);
        }
        if let Some(Some(known)) = self.reads_cell.get(at as usize) {
            return Ok(*known);
        }
        let mut seen = std::collections::HashSet::new();
        let mut seen_roots = std::collections::HashSet::new();
        let mut stack = vec![at];
        let mut paths = Vec::new();
        let mut found = false;
        loop {
            let root = if let Some(current) = stack.pop() {
                if let Some(Some(known)) = self.reads_cell.get(current as usize) {
                    found = found || *known;
                    continue;
                }
                if !seen.insert(current) {
                    continue;
                }
                self.let_root(current)?
            } else if let Some(root) = paths.pop() {
                root
            } else {
                break;
            };
            for node in self.deep_postorder(root)? {
                let (op, _, _, _) = self.ir.node(node)?;
                if op == Op::Field {
                    if let Some(inner) = self.read_root(node)?
                        && seen_roots.insert(inner)
                    {
                        paths.push(inner);
                    }
                    continue;
                }
                if op != Op::Name {
                    continue;
                }
                match self.bound(node)? {
                    DeclRef::Cell(_) => found = true,
                    DeclRef::Let(other) => stack.push(other),
                    DeclRef::None
                    | DeclRef::Instance(_)
                    | DeclRef::Duplicate
                    | DeclRef::Param(_)
                    | DeclRef::Fn(_)
                    | DeclRef::Pattern(_) => {}
                }
            }
        }
        if let Some(slot) = self.reads_cell.get_mut(at as usize) {
            *slot = Some(found);
        }
        Ok(found)
    }

    pub fn check_own_cell(&mut self, at: u32, root: u32) -> Stage<()> {
        Ok(match self.own_ty(root)? {
            Some(own) if self.derived(at) && !self.is_kept(root)? => {
                let (ty, owning) = self.derived_width(at, root, own)?;
                let owning = owning || self.owning;
                let owning = std::mem::replace(&mut self.owning, owning);
                let outcome = self.evaluate(root, ty);
                self.owning = owning;
                outcome?.map(|_| ())
            }
            Some(_) | None => self.evaluate_own(root)?,
        })
    }

    fn derived_width(&self, at: u32, root: u32, own: EvalTy) -> Result<(EvalTy, bool), Internal> {
        let fact = self.fact(root)?;
        let reads = self
            .rule_reads
            .get(at as usize)
            .copied()
            .unwrap_or_default();
        let read = |classes: &[usize]| {
            classes
                .iter()
                .find_map(|class| reads.get(*class).copied().flatten())
        };
        Ok(match fact.leaf {
            Leaf::Float if fact.fixed.is_none() => {
                (read(&[2, 1]).map_or(EvalTy::F32, |(ty, _)| ty), false)
            }
            Leaf::Int if fact.dims.is_empty() && int_range(own).is_some() => {
                match read(&[6, 5, 4]) {
                    Some(_) => (flexible(own), false),
                    None => (own, true),
                }
            }
            _ => (flexible(own), false),
        })
    }

    pub fn run_rule_reads(&mut self, at: u32) -> Stage<()> {
        let Some(rec) = self.ir.cells.get(at as usize).copied() else {
            return Err(Internal::missed("IR cell"));
        };
        if rec.rule == NONE {
            return Ok(Ok(()));
        }
        let reads = self
            .rule_reads
            .get(at as usize)
            .copied()
            .unwrap_or_default();
        let widest = |classes: &[usize]| {
            classes
                .iter()
                .rev()
                .find_map(|at| reads.get(*at).copied().flatten())
        };
        let runs = [
            reads.first().copied().flatten(),
            widest(&[1, 2]),
            reads.get(3).copied().flatten(),
            widest(&[4, 5, 6]),
        ];
        for (ty, unflowed) in runs.into_iter().flatten() {
            if matches!(ty, EvalTy::Exact | EvalTy::Default) {
                continue;
            }
            if unflowed {
                self.unflowed.insert(rec.rule);
            }
            if let Err(declined) = self.evaluate(rec.rule, ty)? {
                return Ok(Err(declined));
            }
        }
        Ok(Ok(()))
    }

    pub fn check_reactive_lets(&mut self) -> Stage<()> {
        let mut reaching = None;
        for top in self.ir.top.clone() {
            let TopRec::Let(at) = top else {
                continue;
            };
            if self.faulty_declaration(top)?
                || self
                    .ir
                    .lets
                    .get(at as usize)
                    .is_some_and(|rec| rec.line_fault)
                || self.cyclic.get(at as usize).copied().unwrap_or(false)
                || !(self.let_reads_cell(at)?
                    || self.calls_reach_a_cell(self.let_root(at)?, &mut reaching)?)
            {
                continue;
            }
            let root = self.let_root(at)?;
            if let Err(declined) = self.ensure_typed(root)? {
                return Ok(Err(declined));
            }
            if !self.is_reactive(root)? {
                continue;
            }
            let expected = match self.annotation_ty(at)? {
                Some(ty)
                    if self.annotation_dims(at)?.is_empty()
                        && self.reads_only_annotated_cells(at)?
                        && self.annotation_refusal(at, ty)? == Refusal::Kept =>
                {
                    Some(ty)
                }
                Some(_) | None => None,
            };
            if let Err(declined) = self.evaluate_own_in(root, expected)? {
                return Ok(Err(declined));
            }
            let rec = self
                .ir
                .lets
                .get(at as usize)
                .ok_or(Internal::missed("IR let"))?;
            let name = self.ir.binder(rec.binder)?.name;
            let span =
                self.name_span(self.let_file.get(at as usize).copied().unwrap_or(0), name)?;
            let text = self.name_text(name)?;
            self.draw(
                Code::ReactiveLet,
                span,
                format!("`{text}` is a `let`, a compile-time value, and its value reads a cell"),
            );
        }
        Ok(Ok(()))
    }

    pub fn check_integer_cells(&mut self) -> Result<(), Internal> {
        for at in 0..u32::try_from(self.ir.cells.len()).unwrap_or(0) {
            if self.derived(at) || self.cell_uses.get(at as usize) != Some(&[false, true]) {
                continue;
            }
            let variable = self.cell_capacity_annotation(at)?.is_some();
            let fact = self.fact(self.cell_root(at)?)?;
            let integer = if variable {
                matches!(
                    self.cell_annotation_ty(at)?,
                    Some(EvalTy::U8 | EvalTy::I32 | EvalTy::I64)
                )
            } else {
                matches!(fact.leaf, Leaf::Int)
                    || (matches!(fact.leaf, Leaf::Empty)
                        && fact.dims.is_empty()
                        && fact.fixed.is_none())
            };
            if !integer {
                continue;
            }
            let rec = self
                .ir
                .cells
                .get(at as usize)
                .ok_or(Internal::missed("IR cell"))?;
            let name = self.ir.binder(rec.binder)?.name;
            let file = self.file_of(rec.init);
            let span = self.name_span(file, name)?;
            let text = self.name_text(name)?;
            self.draw(
                Code::IntegerCell,
                span,
                format!(
                    "`{text}` is an integer host-fed cell whose every use expects a float, so its host cannot write a fraction; a float init such as `1.0` makes it a float cell"
                ),
            );
        }
        Ok(())
    }

    pub fn name_text(&self, name: u32) -> Result<String, Internal> {
        let entry = self.ir.name(name).ok_or(Internal::missed("IR name"))?;
        Ok(self.ir.symbols.text(entry.sym).to_string())
    }
}

impl Core<'_> {
    pub fn reads_float_cell(&mut self, root: u32) -> Result<bool, Internal> {
        let outer = self.inst;
        let found = self.reads_float_cell_in(root);
        self.inst = outer;
        found
    }

    fn local_or_param(&self, node: u32) -> Result<bool, Internal> {
        Ok(match self.bound(node)? {
            DeclRef::Param(_) => true,
            DeclRef::Let(at) => self
                .ir
                .lets
                .get(at as usize)
                .is_some_and(|rec| rec.owner != NONE),
            DeclRef::None
            | DeclRef::Cell(_)
            | DeclRef::Instance(_)
            | DeclRef::Duplicate
            | DeclRef::Fn(_)
            | DeclRef::Pattern(_) => false,
        })
    }

    fn reads_float_cell_in(&mut self, root: u32) -> Result<bool, Internal> {
        let mut node = root;
        loop {
            let (op, a, b, _) = self.ir.node(node)?;
            if matches!(op, Op::Call | Op::NamedCall)
                && let Some(body) = self.body_of_call(node)
            {
                self.inst = body;
                node = self.body_value(body)?;
                continue;
            }
            if op == Op::Name && self.local_or_param(node)? {
                if let DeclRef::Param(position) = self.bound(node)?
                    && let Some((root, caller)) = self.use_argument(node, position)?
                {
                    let saved = self.enter_env(caller);
                    let found = self.reads_float_cell(root);
                    self.leave_env(saved);
                    return found;
                }
                return self.is_reactive(node);
            }
            if matches!(op, Op::Index | Op::IndexFromEnd) {
                let index = self.fact(b)?;
                if index.leaf != Leaf::Int || !index.dims.is_empty() || self.is_reactive(b)? {
                    return Ok(false);
                }
                node = a;
                continue;
            }
            if op == Op::Field {
                let Some(root) = self.read_root(node)? else {
                    return Ok(false);
                };
                if self.bare_cell_read(node)?.is_some() {
                    node = root;
                    continue;
                }
                return self.reads_cell_directly(root);
            }
            if op == Op::Past {
                node = a;
                continue;
            }
            if op == Op::Slice {
                node = a;
                continue;
            }
            return Ok(op == Op::Name && matches!(self.bound(node)?, DeclRef::Cell(_)));
        }
    }
}

impl Core<'_> {
    fn reads_only_annotated_cells(&self, at: u32) -> Result<bool, Internal> {
        let mut seen = std::collections::HashSet::new();
        let mut stack = vec![at];
        while let Some(current) = stack.pop() {
            if !seen.insert(current) {
                continue;
            }
            for node in self.deep_postorder(self.let_root(current)?)? {
                if let Some(cell) = self.bare_cell_read(node)?
                    && self.cell_annotation_ty(cell)?.is_none()
                {
                    return Ok(false);
                }
                if self.ir.node(node)?.0 != Op::Name {
                    continue;
                }
                match self.bound(node)? {
                    DeclRef::Cell(cell) if self.cell_annotation_ty(cell)?.is_none() => {
                        return Ok(false);
                    }
                    DeclRef::Let(other) => stack.push(other),
                    DeclRef::Cell(_)
                    | DeclRef::None
                    | DeclRef::Instance(_)
                    | DeclRef::Duplicate
                    | DeclRef::Param(_)
                    | DeclRef::Fn(_)
                    | DeclRef::Pattern(_) => {}
                }
            }
        }
        Ok(true)
    }
}

impl Core<'_> {
    fn reads_cell_directly(&self, root: u32) -> Result<bool, Internal> {
        let mut seen = std::collections::HashSet::new();
        let mut stack = vec![root];
        while let Some(current) = stack.pop() {
            if !seen.insert(current) {
                continue;
            }
            for node in self.deep_postorder(current)? {
                let (op, _, _, _) = self.ir.node(node)?;
                if op == Op::Field {
                    stack.extend(self.read_root(node)?);
                    continue;
                }
                if op == Op::Name && matches!(self.bound(node)?, DeclRef::Cell(_)) {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }
}

impl Core<'_> {
    fn use_argument(&self, node: u32, position: u32) -> Result<Option<(u32, u32)>, Internal> {
        let owner = self.binding_owner(node)?;
        if self.pattern_at(owner).is_none() {
            return Ok(None);
        }
        let bodies = &self.bodies;
        let Some(env) = self
            .own_calls
            .iter()
            .rev()
            .find(|(env, inst, _)| {
                *env != NONE
                    && bodies
                        .get(*inst as usize)
                        .is_some_and(|body| body.function == owner)
            })
            .map(|(env, _, _)| *env)
        else {
            return Ok(None);
        };
        let Some((root, caller)) = self
            .use_args
            .get(&env)
            .and_then(|args| args.get(position as usize))
            .copied()
        else {
            return Ok(None);
        };
        if root == NONE || self.default_nodes.contains_key(&root) {
            return Ok(None);
        }
        Ok(Some((root, caller)))
    }
}
