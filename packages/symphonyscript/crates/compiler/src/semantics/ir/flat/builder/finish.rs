use super::super::index;
use super::super::item_rec::ItemRec;
use super::ir_builder::IrBuilder;
use super::marks::Marks;
use super::value::Value;
use crate::semantics::Internal;
use crate::syntax::ast::{self, AstNode};
use crate::syntax::{NodeView, SyntaxKind};

impl<'p> IrBuilder<'p, '_> {
    pub(super) fn finish(
        &mut self,
        view: NodeView<'p>,
        base: usize,
        marks: Marks,
    ) -> Result<Value, Internal> {
        let value = match view.kind() {
            SyntaxKind::TypeRef | SyntaxKind::SumType | SyntaxKind::FnType => {
                Some(self.type_value(view, marks))
            }
            SyntaxKind::ArgList | SyntaxKind::NamedArg => Some(self.arg_list(view, base, marks)),
            SyntaxKind::ParamList | SyntaxKind::PatternParamList | SyntaxKind::FnBody => {
                Some(self.stage(base, marks))
            }
            SyntaxKind::Param | SyntaxKind::PatternParam => Some(self.param(view, base)),
            SyntaxKind::LocalLet => self.local_value(view, base),
            SyntaxKind::LetDecl => self.let_decl(view, base, marks),
            SyntaxKind::CellDecl => self.cell_decl(view, base),
            SyntaxKind::Init => {
                let value = ast::Init::cast(view)
                    .and_then(|init| init.value())
                    .and_then(|value| self.value_at(base, value.syntax()));
                Some(match self.force(value) {
                    Some(id) => Value::Expr(id),
                    None => Value::Nothing,
                })
            }
            SyntaxKind::Clock => self.clock(view, base),
            SyntaxKind::FnDecl => self.fn_decl(view, base, marks),
            SyntaxKind::PatternDecl => self.pattern_decl(view, base, marks),
            SyntaxKind::TypeDecl => self.type_decl(view, base),
            SyntaxKind::FieldDecl => self.field_decl(view, base),
            SyntaxKind::Field => {
                let name = ast::Field::cast(view).and_then(|field| field.name());
                self.name_of(name, super::super::name_kind::NameKind::FieldKey);
                Some(Value::Nothing)
            }
            SyntaxKind::PackageDecl => Some(self.drawn_item(
                view,
                marks,
                "the `Package` item of a package file (SPEC §15.2)",
            )),
            SyntaxKind::Require => {
                Some(self.drawn_item(view, marks, "a `require` item (SPEC §15.3)"))
            }
            SyntaxKind::Use => Some(self.drawn_item(view, marks, "a `use` item (SPEC §15.4)")),
            SyntaxKind::Publish => {
                Some(self.drawn_item(view, marks, "a `publish` item (SPEC §15.5)"))
            }
            SyntaxKind::Placement => self.placement(view, base),
            SyntaxKind::WirePlacement => self.wire(view, base),
            SyntaxKind::DefaultArg => self.default_arg(view, base, marks),
            SyntaxKind::Gate => {
                let condition = ast::Gate::cast(view).and_then(|gate| gate.condition());
                let range = view.range();
                Some(Value::Expr(self.expr_of(
                    base,
                    condition,
                    (range.start, range.end),
                )))
            }
            SyntaxKind::Attrs => Some(self.attrs(base)),
            SyntaxKind::AttrEntry => self.attr_entry(view, base, marks),
            SyntaxKind::Body => {
                self.errs.truncate(marks.errs);
                self.body(view, base)
            }
            SyntaxKind::Block => Some(self.block(view, base)),
            SyntaxKind::Line => {
                let members = self.member_items(&view, base, true);
                self.ir.items.push(ItemRec::Line(members));
                Some(Value::Item(index(self.ir.items.len() - 1)))
            }
            SyntaxKind::Members => Some(self.members_value(view, base)),
            SyntaxKind::ParenPlacement => self.paren_placement(view, base),
            SyntaxKind::Bundle => self.bundle(view, base),
            SyntaxKind::BundleFor => self.bundle_for(view, base),
            SyntaxKind::ForBlock => self.for_block(view, base),
            SyntaxKind::WhenBlock => self.when_block(view, base),
            SyntaxKind::GuardArm => self.guard_arm(view, base),
            SyntaxKind::WhenElse => self.when_else(view, base),
            SyntaxKind::Destination => self.destination(view, base),
            SyntaxKind::Lambda => {
                marks.truncate(self.ir);
                Some(Value::Nothing)
            }
            SyntaxKind::ErrorNode => Some(self.error_node(view, marks)),
            _ => self.expression(view, base, marks),
        };
        Ok(value.unwrap_or(Value::Nothing))
    }
}
