use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use core::fmt::{self, Write};

use super::binding::Binding;
use super::entry::Entry;
use super::expr::Expr;
use super::expr_link::ExprLink;
use super::expr_node::ExprNode;
use super::expr_ref::ExprRef;
use super::program::Program;
use super::target::Target;
use crate::ids::{AttrId, EnumId, FamilyId, InstanceId};
use crate::source::Span;
use crate::types::{Scalar, Ty, Value};
use crate::vocab::{AttrDecl, Vocabulary};

pub fn dump(program: &Program, vocabulary: &Vocabulary) -> String {
    let mut out = String::new();
    let mut printer = Printer {
        out: &mut out,
        vocabulary,
    };
    let _ = printer.program(program);
    out
}

struct Printer<'a> {
    out: &'a mut String,
    vocabulary: &'a Vocabulary,
}

impl Printer<'_> {
    fn program(&mut self, program: &Program) -> fmt::Result {
        self.out.push_str("vocabulary ");
        self.string(&program.vocabulary.name)?;
        writeln!(self.out, " 0x{:016x}", program.vocabulary.hash)?;
        self.out.push_str("top_level [");
        for (position, id) in program.top_level.iter().enumerate() {
            if position > 0 {
                self.out.push_str(", ");
            }
            self.instance_id(*id)?;
        }
        self.out.push_str("]\n");
        for (index, instance) in program.instances.iter().enumerate() {
            writeln!(self.out, "instance i{index}")?;
            let decl = self.vocabulary.node_kind(instance.kind);
            match decl {
                Some(decl) => writeln!(self.out, "  kind {}", decl.name)?,
                None => writeln!(self.out, "  kind #{}", instance.kind.index())?,
            }
            writeln!(self.out, "  path {}", instance.path)?;
            self.out.push_str("  span ");
            self.span(&instance.span)?;
            self.out.push('\n');
            self.spans(&instance.expanded_from)?;
            self.attrs(&instance.attrs, decl.map(|decl| decl.attrs.as_slice()))?;
            self.gate(instance.gate.as_ref())?;
            self.out.push_str("  sequence\n");
            for entry in &instance.sequence {
                match entry {
                    Entry::Child(id) => {
                        self.out.push_str("    child ");
                        self.instance_id(*id)?;
                    }
                    Entry::Bundle(members) => {
                        self.out.push_str("    bundle [");
                        for (position, id) in members.iter().enumerate() {
                            if position > 0 {
                                self.out.push_str(", ");
                            }
                            self.instance_id(*id)?;
                        }
                        self.out.push(']');
                    }
                    Entry::Wire(id) => write!(self.out, "    wire w{}", id.0)?,
                }
                self.out.push('\n');
            }
        }
        for (index, wire) in program.wires.iter().enumerate() {
            writeln!(self.out, "wire w{index}")?;
            let decl = self.vocabulary.wire_kind(wire.kind);
            match decl {
                Some(decl) => writeln!(self.out, "  kind {}", decl.name)?,
                None => writeln!(self.out, "  kind #{}", wire.kind.index())?,
            }
            writeln!(self.out, "  path {}", wire.path)?;
            self.out.push_str("  span ");
            self.span(&wire.span)?;
            self.out.push('\n');
            self.spans(&wire.expanded_from)?;
            self.out.push_str("  source ");
            self.instance_id(wire.source)?;
            self.out.push('\n');
            match &wire.target {
                Target::Fixed(id) => {
                    self.out.push_str("  target fixed ");
                    self.instance_id(*id)?;
                }
                Target::Select { candidates, index } => {
                    self.out.push_str("  target select [");
                    for (position, id) in candidates.iter().enumerate() {
                        if position > 0 {
                            self.out.push_str(", ");
                        }
                        self.instance_id(*id)?;
                    }
                    self.out.push_str("] index ");
                    self.binding(index)?;
                }
            }
            self.out.push('\n');
            self.attrs(&wire.attrs, decl.map(|decl| decl.attrs.as_slice()))?;
            self.gate(wire.gate.as_ref())?;
        }
        for (index, input) in program.inputs.iter().enumerate() {
            writeln!(self.out, "input {index}")?;
            self.out.push_str("  name ");
            self.string(&input.name)?;
            self.out.push_str("\n  ty ");
            self.ty(&input.ty)?;
            self.out.push_str("\n  family ");
            match input.family {
                Some(family) => self.family(family)?,
                None => self.out.push_str("none"),
            }
            writeln!(
                self.out,
                "\n  slots {}..{}",
                input.slots.start.0, input.slots.end.0
            )?;
            self.out.push_str("  initial [");
            for (position, value) in input.initial.iter().enumerate() {
                if position > 0 {
                    self.out.push_str(", ");
                }
                self.value(*value)?;
            }
            self.out.push_str("]\n  span ");
            self.span(&input.span)?;
            self.out.push('\n');
        }
        for (index, cell) in program.cells.iter().enumerate() {
            writeln!(self.out, "cell c{index}")?;
            self.out.push_str("  ty ");
            self.ty(&cell.ty)?;
            self.out.push_str("\n  expr ");
            self.expr(&cell.expr)?;
            self.out.push_str("\n  span ");
            self.span(&cell.span)?;
            self.out.push('\n');
        }
        for (index, text) in program.strings.iter().enumerate() {
            write!(self.out, "string s{index} ")?;
            self.string(text)?;
            self.out.push('\n');
        }
        Ok(())
    }

    fn instance_id(&mut self, id: InstanceId) -> fmt::Result {
        write!(self.out, "i{}", id.0)
    }

    fn spans(&mut self, spans: &[Span]) -> fmt::Result {
        self.out.push_str("  expanded_from [");
        for (position, span) in spans.iter().enumerate() {
            if position > 0 {
                self.out.push_str(", ");
            }
            self.span(span)?;
        }
        self.out.push_str("]\n");
        Ok(())
    }

    fn attrs(&mut self, attrs: &[(AttrId, Binding)], decls: Option<&[AttrDecl]>) -> fmt::Result {
        self.out.push_str("  attrs\n");
        for (attr, binding) in attrs {
            match decls.and_then(|decls| decls.get(attr.index())) {
                Some(decl) => write!(self.out, "    {} ", decl.name)?,
                None => write!(self.out, "    #{} ", attr.0)?,
            }
            self.binding(binding)?;
            self.out.push('\n');
        }
        Ok(())
    }

    fn gate(&mut self, gate: Option<&Binding>) -> fmt::Result {
        self.out.push_str("  gate ");
        match gate {
            Some(binding) => self.binding(binding)?,
            None => self.out.push_str("none"),
        }
        self.out.push('\n');
        Ok(())
    }

    fn binding(&mut self, binding: &Binding) -> fmt::Result {
        let mut steps = vec![BindingStep::Visit(binding)];
        while let Some(step) = steps.pop() {
            match step {
                BindingStep::Text(text) => self.out.push_str(text),
                BindingStep::Visit(Binding::Const(value)) => {
                    self.out.push_str("const ");
                    self.value(*value)?;
                }
                BindingStep::Visit(Binding::Input(slot)) => write!(self.out, "slot {}", slot.0)?,
                BindingStep::Visit(Binding::Cell(cell)) => write!(self.out, "cell c{}", cell.0)?,
                BindingStep::Visit(Binding::Array(elements)) => {
                    self.out.push_str("array [");
                    steps.push(BindingStep::Text("]"));
                    for (position, element) in elements.iter().enumerate().rev() {
                        steps.push(BindingStep::Visit(element));
                        if position > 0 {
                            steps.push(BindingStep::Text(", "));
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn expr(&mut self, expr: &Expr) -> fmt::Result {
        let mut steps = vec![ExprStep::Link(None, expr.root)];
        while let Some(step) = steps.pop() {
            let (parent, reference) = match step {
                ExprStep::Text(text) => {
                    self.out.push_str(text);
                    continue;
                }
                ExprStep::Value(value) => {
                    self.value(value)?;
                    continue;
                }
                ExprStep::Link(parent, reference) => (parent, reference),
            };
            let index = match expr.link(parent, reference) {
                ExprLink::Node(index) => index,
                ExprLink::Dangling(reference) => {
                    write!(self.out, "(#{reference})")?;
                    continue;
                }
            };
            let node = &expr.nodes[index];
            let mut tail: Vec<ExprStep> = Vec::new();
            let child = |reference: ExprRef| ExprStep::Link(Some(index), reference);
            match node {
                ExprNode::Const(value) => {
                    self.out.push_str("(const ");
                    self.value(*value)?;
                }
                ExprNode::Input(slot) => write!(self.out, "(slot {}", slot.0)?,
                ExprNode::Cell(cell) => write!(self.out, "(cell c{}", cell.0)?,
                ExprNode::Unary { op, ty, operand } => {
                    write!(self.out, "({} ", op.name())?;
                    self.ty(ty)?;
                    tail.extend([ExprStep::Text(" "), child(*operand)]);
                }
                ExprNode::Binary { op, ty, lhs, rhs } => {
                    write!(self.out, "({} ", op.name())?;
                    self.ty(ty)?;
                    tail.extend([
                        ExprStep::Text(" "),
                        child(*lhs),
                        ExprStep::Text(" "),
                        child(*rhs),
                    ]);
                }
                ExprNode::If {
                    condition,
                    then,
                    otherwise,
                } => {
                    self.out.push_str("(if");
                    tail.extend([
                        ExprStep::Text(" "),
                        child(*condition),
                        ExprStep::Text(" "),
                        child(*then),
                        ExprStep::Text(" "),
                        child(*otherwise),
                    ]);
                }
                ExprNode::Match {
                    ty,
                    scrutinee,
                    arms,
                    otherwise,
                } => {
                    self.out.push_str("(match ");
                    self.ty(ty)?;
                    tail.extend([ExprStep::Text(" "), child(*scrutinee)]);
                    for (label, value) in arms {
                        tail.extend([
                            ExprStep::Text(" ["),
                            ExprStep::Value(*label),
                            ExprStep::Text(" "),
                            child(*value),
                            ExprStep::Text("]"),
                        ]);
                    }
                    if let Some(otherwise) = otherwise {
                        tail.extend([
                            ExprStep::Text(" [else "),
                            child(*otherwise),
                            ExprStep::Text("]"),
                        ]);
                    }
                }
                ExprNode::Pick {
                    index_ty,
                    index: picked,
                    items,
                } => {
                    self.out.push_str("(pick ");
                    self.ty(index_ty)?;
                    tail.extend([ExprStep::Text(" "), child(*picked)]);
                    for item in items {
                        tail.extend([ExprStep::Text(" "), child(*item)]);
                    }
                }
                ExprNode::Convert { from, to, operand } => {
                    self.out.push_str("(convert ");
                    self.ty(from)?;
                    self.out.push(' ');
                    self.ty(to)?;
                    tail.extend([ExprStep::Text(" "), child(*operand)]);
                }
                ExprNode::Call { builtin, ty, args } => {
                    write!(self.out, "(call {} ", builtin.name())?;
                    self.ty(ty)?;
                    for arg in args {
                        tail.extend([ExprStep::Text(" "), child(*arg)]);
                    }
                }
            }
            steps.push(ExprStep::Text(")"));
            steps.extend(tail.into_iter().rev());
        }
        Ok(())
    }

    fn ty(&mut self, ty: &Ty) -> fmt::Result {
        match ty.scalar {
            Scalar::I32 => self.out.push_str("i32"),
            Scalar::I64 => self.out.push_str("i64"),
            Scalar::F32 => self.out.push_str("f32"),
            Scalar::F64 => self.out.push_str("f64"),
            Scalar::Bool => self.out.push_str("bool"),
            Scalar::Str => self.out.push_str("str"),
            Scalar::Enum(id) => self.enum_name(id)?,
        }
        for length in &ty.lengths {
            write!(self.out, "[{length}]")?;
        }
        Ok(())
    }

    fn value(&mut self, value: Value) -> fmt::Result {
        match value {
            Value::I32(v) => write!(self.out, "i32 {v}"),
            Value::I64(v) => write!(self.out, "i64 {v}"),
            Value::F32(v) => {
                self.out.push_str("f32 ");
                if v.is_nan() {
                    if v.to_bits() == 0x7FC0_0000 {
                        self.out.push_str("nan");
                        Ok(())
                    } else {
                        write!(self.out, "nan:0x{:08x}", v.to_bits())
                    }
                } else {
                    write!(self.out, "{v:?}")
                }
            }
            Value::F64(v) => {
                self.out.push_str("f64 ");
                if v.is_nan() {
                    if v.to_bits() == 0x7FF8_0000_0000_0000 {
                        self.out.push_str("nan");
                        Ok(())
                    } else {
                        write!(self.out, "nan:0x{:016x}", v.to_bits())
                    }
                } else {
                    write!(self.out, "{v:?}")
                }
            }
            Value::Bool(v) => write!(self.out, "bool {v}"),
            Value::Enum(id, variant) => {
                self.enum_name(id)?;
                self.out.push_str("::");
                match self
                    .vocabulary
                    .enum_decl(id)
                    .and_then(|decl| decl.variants.get(usize::from(variant)))
                {
                    Some(name) => self.out.push_str(name),
                    None => write!(self.out, "#{variant}")?,
                }
                Ok(())
            }
            Value::Str(id) => write!(self.out, "str s{}", id.0),
        }
    }

    fn enum_name(&mut self, id: EnumId) -> fmt::Result {
        match self.vocabulary.enum_decl(id) {
            Some(decl) => {
                self.out.push_str(&decl.name);
                Ok(())
            }
            None => write!(self.out, "enum#{}", id.index()),
        }
    }

    fn family(&mut self, id: FamilyId) -> fmt::Result {
        match self.vocabulary.family(id) {
            Some(decl) => {
                self.out.push_str(&decl.name);
                Ok(())
            }
            None => write!(self.out, "family#{}", id.index()),
        }
    }

    fn span(&mut self, span: &Span) -> fmt::Result {
        self.string(&span.file)?;
        write!(self.out, " {}..{}", span.range.start, span.range.end)
    }

    fn string(&mut self, text: &str) -> fmt::Result {
        self.out.push('"');
        for c in text.chars() {
            match c {
                '"' => self.out.push_str("\\\""),
                '\\' => self.out.push_str("\\\\"),
                '\n' => self.out.push_str("\\n"),
                '\t' => self.out.push_str("\\t"),
                '\r' => self.out.push_str("\\r"),
                c if (c as u32) < 0x20 || c as u32 == 0x7F => {
                    write!(self.out, "\\u{{{:x}}}", c as u32)?
                }
                c => self.out.push(c),
            }
        }
        self.out.push('"');
        Ok(())
    }
}

enum BindingStep<'a> {
    Visit(&'a Binding),
    Text(&'static str),
}

enum ExprStep {
    Link(Option<usize>, ExprRef),
    Value(Value),
    Text(&'static str),
}
