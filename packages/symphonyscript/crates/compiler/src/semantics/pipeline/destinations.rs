use super::core::{Core, Stage};
use super::decl_ref::DeclRef;
use super::declined::Declined;
use super::fact::Fact;
use super::leaf::Leaf;
use super::reason::Reason;
use crate::diagnostics::Code;
use crate::semantics::Internal;
use crate::semantics::counted_walk::CountedWalk;
use crate::semantics::ir::flat::{NONE, Op};
use crate::semantics::operators::{EvalTy, Value, index_outside};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Choice {
    Known(u32),
    Runtime,
    Unknown,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Test {
    Known(u32),
    Runtime,
    Faulty,
}

impl Test {
    fn takes(self, at: u32) -> bool {
        match self {
            Test::Known(known) => known == at,
            Test::Runtime => true,
            Test::Faulty => false,
        }
    }
}

fn joined(test: Test, choices: &[Choice]) -> Choice {
    match test {
        Test::Known(at) => choices.get(at as usize).copied().unwrap_or(Choice::Unknown),
        Test::Runtime if choices.contains(&Choice::Unknown) => Choice::Unknown,
        Test::Runtime => Choice::Runtime,
        Test::Faulty => Choice::Unknown,
    }
}

impl Core<'_> {
    pub fn choose_destinations(&mut self) -> Stage<()> {
        for at in 0..self.wires.len() {
            let row = *self.wires.get(at).ok_or(Internal::missed("wire row"))?;
            if !row.chosen {
                continue;
            }
            let root = self.wire_rec(row.item)?.destination;
            let saved = self.enter_env(row.env);
            let mut found = Vec::new();
            let chosen = self.chosen_destination(root, &mut found);
            self.leave_env(saved);
            let choice = match chosen? {
                Ok(choice) => choice,
                Err(declined) => return Ok(Err(declined)),
            };
            let first = u32::try_from(self.candidates.len()).unwrap_or(u32::MAX);
            match choice {
                Choice::Known(target) => self.candidates.push(target),
                Choice::Runtime => self.candidates.extend(found),
                Choice::Unknown => {}
            }
            let end = u32::try_from(self.candidates.len()).unwrap_or(u32::MAX);
            if let Some(slot) = self.wires.get_mut(at) {
                slot.candidates = (first, end);
                slot.fixed = matches!(choice, Choice::Known(_));
            }
        }
        Ok(Ok(()))
    }

    fn names_one_instance(&mut self, root: u32, fact: &Fact) -> Result<bool, Internal> {
        if fact.node && fact.dims.is_empty() {
            return Ok(true);
        }
        if !fact.node && matches!(fact.leaf, Leaf::Placeholder | Leaf::Error) {
            return Ok(false);
        }
        let summary = if fact.dims.is_empty() {
            "a destination names an instance"
        } else {
            "a destination is one instance, not an array"
        };
        self.draw_at(Code::DestinationType, root, summary)?;
        Ok(false)
    }

    fn choose(&mut self, node: u32, live: bool, found: &mut Vec<u32>) -> Stage<Choice> {
        CountedWalk::CoreWireDestination.step();
        let (op, a, b, c) = self.ir.node(node)?;
        if op == Op::Name
            && let DeclRef::Let(at) = self.bound(node)?
        {
            if self.cyclic.get(at as usize).copied().unwrap_or(true) {
                return Ok(Ok(Choice::Unknown));
            }
            return self.choose(self.let_root(at)?, live, found);
        }
        let (test, arms) = if op == Op::If {
            let test = match self.condition(node, a, live)? {
                Ok(test) => test,
                Err(declined) => return Ok(Err(declined)),
            };
            (test, vec![b, c])
        } else if op == Op::Index {
            let Some(elements) = self.reference_elements(a)? else {
                if live
                    && self.ir.node(a)?.0 == Op::Name
                    && matches!(self.bound(a)?, DeclRef::Param(_))
                {
                    return Ok(match self.evaluate(node, EvalTy::Exact)? {
                        Ok(Value::Node(row)) if row != NONE => {
                            found.push(row);
                            Ok(Choice::Known(row))
                        }
                        Ok(_) => Ok(Choice::Unknown),
                        Err(declined) => Err(declined),
                    });
                }
                return Ok(Err(Declined(Reason::DestinationValue)));
            };
            let count = u32::try_from(elements.len()).unwrap_or(u32::MAX);
            let test = match self.position(node, b, count, live)? {
                Ok(test) => test,
                Err(declined) => return Ok(Err(declined)),
            };
            (test, elements)
        } else {
            if !live {
                return Ok(Ok(Choice::Unknown));
            }
            if op == Op::Name && matches!(self.bound(node)?, DeclRef::Param(_)) {
                return Ok(match self.evaluate(node, EvalTy::Exact)? {
                    Ok(Value::Node(row)) if row != NONE => {
                        found.push(row);
                        Ok(Choice::Known(row))
                    }
                    Ok(_) => Ok(Choice::Unknown),
                    Err(declined) => Err(declined),
                });
            }
            let target = match self.path_row(node) {
                Some(row) => row,
                None => self.candidate_row(node)?,
            };
            if target == NONE {
                return Ok(Ok(Choice::Unknown));
            }
            found.push(target);
            return Ok(Ok(Choice::Known(target)));
        };
        let mut choices = Vec::with_capacity(arms.len());
        for (at, arm) in (0u32..).zip(arms) {
            let taken = live && test.takes(at);
            match self.choose(arm, taken, found)? {
                Ok(choice) => choices.push(choice),
                Err(declined) => return Ok(Err(declined)),
            }
        }
        Ok(Ok(joined(test, &choices)))
    }

    fn condition(&mut self, node: u32, condition: u32, live: bool) -> Stage<Test> {
        let fact = match self.ensure_typed(condition)? {
            Ok(fact) => fact,
            Err(declined) => return Ok(Err(declined)),
        };
        if !fact.dims.is_empty() {
            self.draw_at(
                Code::ConditionType,
                node,
                "the condition of an `if` is a `bool`, not an array",
            )?;
            return self.refused_test(condition, live);
        }
        match fact.leaf {
            Leaf::Bool => {}
            Leaf::Placeholder | Leaf::Error => return Ok(Ok(Test::Faulty)),
            Leaf::Empty | Leaf::Int | Leaf::Float | Leaf::Enum(_) => {
                self.draw_at_root(Code::ConditionType, condition, "the condition is a `bool`")?;
                return self.refused_test(condition, live);
            }
        }
        self.chosen_value(condition, EvalTy::Bool, live, 2)
    }

    fn refused_test(&mut self, condition: u32, live: bool) -> Stage<Test> {
        if live && self.is_reactive(condition)? {
            return Ok(Ok(Test::Runtime));
        }
        Ok(Ok(Test::Faulty))
    }

    fn position(&mut self, node: u32, index: u32, count: u32, live: bool) -> Stage<Test> {
        let fact = match self.ensure_typed(index)? {
            Ok(fact) => fact,
            Err(declined) => return Ok(Err(declined)),
        };
        if matches!(fact.leaf, Leaf::Placeholder | Leaf::Error) {
            return Ok(Ok(Test::Faulty));
        }
        if !fact.dims.is_empty() {
            self.draw_at(
                Code::IndexType,
                node,
                "an index is an integer, not an array",
            )?;
            return Ok(Ok(Test::Faulty));
        }
        if matches!(fact.leaf, Leaf::Float | Leaf::Bool) {
            self.draw_at_root(Code::IndexType, index, "an index is an integer")?;
            return Ok(Ok(Test::Faulty));
        }
        self.chosen_value(index, EvalTy::Exact, live, count)
    }

    fn chosen_value(&mut self, root: u32, ty: EvalTy, live: bool, count: u32) -> Stage<Test> {
        if !live {
            return Ok(Ok(Test::Faulty));
        }
        if self.is_kept(root)? || self.unchecked {
            return self
                .own_check(root)
                .map(|checked| checked.map(|()| Test::Faulty));
        }
        let reactive = self.is_reactive(root)?;
        let value = match self.evaluate(root, ty)? {
            Ok(value) => value,
            Err(declined) => return Ok(Err(declined)),
        };
        if reactive {
            return Ok(Ok(Test::Runtime));
        }
        Ok(Ok(match value {
            Value::Bool(value) => Test::Known(u32::from(!value)),
            Value::Int(position) => match u32::try_from(position) {
                Ok(position) if position < count => Test::Known(position),
                Ok(_) | Err(_) => {
                    index_outside(self, root, position, u64::from(count))?;
                    Test::Faulty
                }
            },
            Value::F32(_)
            | Value::F64(_)
            | Value::Enum(..)
            | Value::Str(_)
            | Value::Node(_)
            | Value::Array(_)
            | Value::Error => Test::Faulty,
        }))
    }
}

impl Core<'_> {
    fn chosen_destination(&mut self, root: u32, found: &mut Vec<u32>) -> Stage<Choice> {
        let fact = match self.ensure_typed(root)? {
            Ok(fact) => fact,
            Err(declined) => return Ok(Err(declined)),
        };
        if self.names_one_instance(root, &fact)? {
            return self.choose(root, true, found);
        }
        Ok(Ok(Choice::Unknown))
    }
}

impl Core<'_> {
    pub fn destination_names(&mut self) -> Result<(), Internal> {
        let mut names = Vec::new();
        for item in &self.ir.items {
            let crate::semantics::ir::flat::ItemRec::Wire(rec) = item else {
                continue;
            };
            for node in self.postorder(rec.destination)? {
                if self.ir.node(node)?.0 == Op::Name && self.bound(node)? == DeclRef::None {
                    names.push(node);
                }
            }
        }
        if !names.is_empty() {
            names.sort_unstable();
            self.placeholders
                .retain(|node| names.binary_search(node).is_err());
        }
        Ok(())
    }
}
