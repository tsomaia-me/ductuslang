use super::super::name_kind::NameKind;
use super::super::op::Op;
use super::super::{NONE, index};
use super::constructs::LAMBDA;
use super::ir_builder::{IrBuilder, span};
use super::marks::Marks;
use super::value::{Pending, Value};
use crate::diagnostics::Code;
use crate::syntax::ast::{self, AstNode};
use crate::syntax::{NodeView, SyntaxKind};

enum Arg {
    Positional(u32),
    Named(u32, u32),
}

impl<'p> IrBuilder<'p, '_> {
    pub(super) fn arg_list(&mut self, view: NodeView<'p>, base: usize, marks: Marks) -> Value {
        if view.kind() == SyntaxKind::NamedArg {
            let Some(node) = ast::NamedArg::cast(view) else {
                return Value::Nothing;
            };
            let name = self.name_of(node.name(), NameKind::NamedArgument);
            let value = node
                .value()
                .and_then(|value| self.value_at(base, value.syntax()));
            let value = self.force(value).unwrap_or(NONE);
            return Value::Named { name, value };
        }
        self.stage(base, marks)
    }

    pub(super) fn call(&mut self, view: NodeView<'p>, base: usize, marks: Marks) -> u32 {
        let whole = span(&view);
        let Some(node) = ast::CallExpr::cast(view) else {
            return self.bad(whole);
        };
        let callee = node.callee();
        let args = node.args();
        let args_value = args
            .as_ref()
            .and_then(|args| self.value_at(base, args.syntax()));
        let callee_value = callee
            .as_ref()
            .and_then(|callee| self.value_at(base, callee.syntax()));
        if let Some(ast::Expr::Field(field)) = &callee
            && field.field().is_some_and(|name| name.text() == "past")
            && let Some(Value::Pending(pending)) = callee_value
        {
            return self.past(pending, args.as_ref(), args_value, marks, whole);
        }
        let Some(args) = args else {
            return self.dead(marks, whole);
        };
        let mut lambdas = Vec::new();
        let mut entries = Vec::new();
        let mut positional = false;
        let mut named = false;
        for child in args.syntax().children() {
            if child.kind() == SyntaxKind::NamedArg {
                lambdas.extend(
                    child
                        .children()
                        .find_map(ast::Lambda::cast)
                        .map(|lambda| span(lambda.syntax())),
                );
                named = true;
                entries.push((child, true));
            } else if child.kind() == SyntaxKind::Lambda {
                lambdas.push(span(&child));
            } else if ast::Expr::cast(child).is_some() {
                positional = true;
                entries.push((child, false));
            }
        }
        if !lambdas.is_empty() {
            marks.truncate(self.ir);
            let from = self.drawn_mark();
            for lambda in lambdas {
                self.draw_unbuilt(lambda, LAMBDA);
            }
            return self.bad_drawn(whole, from);
        }
        if positional && named {
            return self.dead(marks, whole);
        }
        let mut built = Vec::with_capacity(entries.len());
        for (child, is_named) in entries {
            let value = self.staged_at(args_value, &child);
            if is_named {
                let Some(Value::Named { name, value }) = value else {
                    return self.dead(marks, whole);
                };
                if !self.named(name) {
                    return self.dead(marks, whole);
                }
                built.push(Arg::Named(name, value));
            } else {
                let value = self.force(value).unwrap_or(NONE);
                built.push(Arg::Positional(value));
            }
        }
        let cut = self
            .first_in(span(args.syntax()).0..span(args.syntax()).1)
            .is_some();
        let method = match (&callee, callee_value) {
            (Some(ast::Expr::Field(_)), Some(Value::Pending(pending)))
                if self.named(pending.field) =>
            {
                let receiver = if pending.receiver_missing {
                    NONE
                } else {
                    pending.receiver
                };
                Some((receiver, None, (pending.field, pending.lo, pending.hi)))
            }
            (Some(ast::Expr::Field(_)), Some(Value::Expr(id))) => {
                if let Ok((Op::Field, receiver, field, missing)) = self.ir.node(id) {
                    let receiver = if missing == 1 {
                        self.set_span(receiver, whole);
                        receiver
                    } else {
                        receiver
                    };
                    self.set_node(id, Op::Name, (field, 0, 0));
                    Some((receiver, Some(id), (NONE, 0, 0)))
                } else {
                    None
                }
            }
            _ => None,
        };
        let callee = match &method {
            Some(_) => NONE,
            None => match self.force(callee_value) {
                Some(id) => id,
                None => self.bad(whole),
            },
        };
        let mut finished = Vec::with_capacity(built.len() * 2);
        for arg in built {
            match arg {
                Arg::Positional(value) => finished.push(self.or_bad(value, whole)),
                Arg::Named(name, value) => {
                    let value = self.or_bad(value, whole);
                    finished.extend([name, value]);
                }
            }
        }
        let count = if named {
            index(finished.len() / 2)
        } else {
            index(finished.len())
        };
        let start = index(self.ir.kids.len());
        let shape = 2 * count + u32::from(cut);
        match method {
            Some((receiver, name_node, (field, lo, hi))) => {
                let receiver = self.or_bad(receiver, whole);
                let name_node = match name_node {
                    Some(id) => id,
                    None => self.emit(Op::Name, (field, 0, 0), (lo, hi)),
                };
                self.ir.kids.push(receiver);
                self.ir.kids.extend(finished);
                let op = if named {
                    Op::NamedMethodCall
                } else {
                    Op::MethodCall
                };
                self.emit(op, (name_node, start, shape), whole)
            }
            None => {
                self.ir.kids.extend(finished);
                let op = if named { Op::NamedCall } else { Op::Call };
                self.emit(op, (callee, start, shape), whole)
            }
        }
    }

    fn past(
        &mut self,
        pending: Pending,
        args: Option<&ast::ArgList<'p>>,
        args_value: Option<Value>,
        marks: Marks,
        whole: (u32, u32),
    ) -> u32 {
        let target_ok = pending.receiver_is_name_ref && pending.receiver_name != NONE;
        let wrong_receiver = !pending.receiver_missing && !pending.receiver_is_name_ref;
        let positional: Vec<NodeView<'p>> = args
            .map(|args| {
                args.syntax()
                    .children()
                    .filter(|child| ast::Expr::cast(*child).is_some())
                    .collect()
            })
            .unwrap_or_default();
        let has_named = args.is_some_and(|args| {
            args.syntax()
                .children()
                .any(|child| child.kind() == SyntaxKind::NamedArg)
        });
        let values: Vec<Option<u32>> = positional
            .iter()
            .map(|child| {
                let value = self.staged_at(args_value, child);
                self.force(value)
            })
            .collect();
        let failure = !target_ok || args.is_none() || has_named || !matches!(values.len(), 1 | 2);
        if failure {
            marks.truncate(self.ir);
            let from = self.drawn_mark();
            if wrong_receiver {
                self.draw(
                    Code::HistoryTarget,
                    pending.receiver_range,
                    "only a cell has a history, and `.past` reads the history of the cell it follows by name",
                );
            }
            if target_ok && args.is_some() {
                if has_named {
                    self.draw(
                        Code::NamedBuiltinArguments,
                        whole,
                        "`past` takes positional arguments only",
                    );
                } else {
                    self.draw(
                        Code::ArgumentCount,
                        whole,
                        &format!(
                            "`past` takes the offset and an optional fallback, 1 or 2 arguments, not {}",
                            values.len()
                        ),
                    );
                }
            }
            return self.bad_drawn(whole, from);
        }
        let offset = values.first().copied().flatten().unwrap_or(NONE);
        let offset = self.or_bad(offset, whole);
        let fallback = match values.get(1) {
            Some(value) => {
                let value = value.unwrap_or(NONE);
                self.or_bad(value, whole)
            }
            None => NONE,
        };
        self.emit(Op::Past, (pending.receiver, offset, fallback), whole)
    }
}
