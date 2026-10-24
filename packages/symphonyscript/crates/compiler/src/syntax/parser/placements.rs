use super::Parser;
use super::checkpoint::Checkpoint;
use super::task::Task;
use crate::diagnostics::{Code, Diagnostic};
use crate::syntax::lexer::is_ident_continue;
use crate::syntax::syntax_kind::SyntaxKind;

pub(super) const FLAG_TOKENS: &[SyntaxKind] = &[
    SyntaxKind::Quote,
    SyntaxKind::Bang,
    SyntaxKind::Question,
    SyntaxKind::Star,
    SyntaxKind::Star2,
    SyntaxKind::Plus,
    SyntaxKind::Tilde,
    SyntaxKind::At,
    SyntaxKind::Dollar,
];

const MEMBER_START: &[SyntaxKind] = &[SyntaxKind::Ident, SyntaxKind::LParen, SyntaxKind::LBracket];

impl Parser<'_> {
    pub(super) fn top_placement(&mut self) {
        self.start(SyntaxKind::Placement);
        let kind = self.kind_token();
        self.bump();
        self.flags();
        self.eat(SyntaxKind::AsKw);
        self.expect_name(Code::PlacementSyntax, "the top-level instance's name");
        self.then([Task::DefaultArg { kind }, Task::PlacementTail, Task::Finish]);
    }

    pub(super) fn wire_placement(&mut self) {
        self.start(SyntaxKind::WirePlacement);
        let kind = self.kind_token();
        self.bump();
        self.flags();
        if self.eat(SyntaxKind::AsKw) {
            self.expect_name(Code::WireSyntax, "the wire's name");
        }
        self.then([Task::DefaultArg { kind }, Task::WireAfterDefault]);
    }

    pub(super) fn wire_after_default(&mut self) {
        if self.at(SyntaxKind::WhenKw) {
            self.then([Task::Gate, Task::WireAfterGate]);
        } else {
            self.then([Task::WireAfterGate]);
        }
    }

    pub(super) fn wire_after_gate(&mut self) {
        if self.at(SyntaxKind::Pipe) {
            self.then([Task::Attrs, Task::WireAfterAttrs]);
        } else {
            self.then([Task::WireAfterAttrs]);
        }
    }

    pub(super) fn wire_after_attrs(&mut self) {
        if self.expect(
            SyntaxKind::Colon,
            Code::WireSyntax,
            "`:` and the wire's destination",
        ) {
            self.start(SyntaxKind::Destination);
            if self.at(SyntaxKind::MatchKw) {
                self.then([Task::MatchBlock, Task::Finish, Task::Finish]);
            } else {
                self.then([
                    Task::Expr,
                    Task::FinishLineFinish(Code::WireSyntax),
                    Task::Finish,
                ]);
            }
        } else {
            self.finish_line(Code::WireSyntax);
            self.finish();
        }
    }

    pub(super) fn line(&mut self) {
        let checkpoint = self.checkpoint();
        if self.at(SyntaxKind::Ident) {
            self.start(SyntaxKind::Placement);
            self.then([Task::PlacementHead, Task::LineAfterHead(checkpoint)]);
        } else {
            self.start_at(checkpoint, SyntaxKind::Line);
            self.then([Task::LineMembers]);
        }
    }

    pub(super) fn line_after_head(&mut self, checkpoint: Checkpoint) {
        if self.at_any(MEMBER_START) {
            self.finish();
            self.start_at(checkpoint, SyntaxKind::Line);
            self.then([Task::LineMembers]);
        } else {
            self.then([Task::PlacementTail, Task::Finish]);
        }
    }

    pub(super) fn line_members(&mut self) {
        if self.at_any(MEMBER_START) {
            self.then([Task::Member, Task::LineMembers]);
            return;
        }
        if self.at_any(&[SyntaxKind::WhenKw, SyntaxKind::Pipe, SyntaxKind::Colon]) {
            self.error(
                Code::InlinePlacement,
                "a placement with a gate, an attribute clause or a body is parenthesized or written on its own line",
            );
        }
        self.finish_line(Code::InlinePlacement);
        self.finish();
    }

    pub(super) fn placement_tail(&mut self) {
        if self.at(SyntaxKind::WhenKw) {
            self.then([Task::Gate, Task::PlacementTailAttrs]);
        } else {
            self.then([Task::PlacementTailAttrs]);
        }
    }

    pub(super) fn placement_tail_attrs(&mut self) {
        if self.at(SyntaxKind::Pipe) {
            self.then([Task::Attrs, Task::PlacementTailBody]);
        } else {
            self.then([Task::PlacementTailBody]);
        }
    }

    pub(super) fn placement_tail_body(&mut self) {
        if self.at(SyntaxKind::Colon) {
            self.start(SyntaxKind::Body);
            self.bump();
            self.then([Task::Block { pattern: false }, Task::Finish]);
        } else {
            self.finish_line(Code::PlacementSyntax);
        }
    }

    pub(super) fn members(&mut self, kind: SyntaxKind) {
        self.start(kind);
        if !self.at_any(MEMBER_START) {
            let found = self.describe();
            self.error(
                Code::PlacementSyntax,
                format!("expected a placement, found {found}"),
            );
        }
        self.then([Task::MembersLoop]);
    }

    pub(super) fn members_loop(&mut self) {
        if self.at_any(MEMBER_START) {
            self.then([Task::Member, Task::MembersLoop]);
        } else {
            self.finish();
        }
    }

    pub(super) fn member(&mut self) {
        match self.peek() {
            Some(SyntaxKind::LParen) => {
                self.start(SyntaxKind::ParenPlacement);
                self.bump();
                if self.at(SyntaxKind::Ident) {
                    self.start(SyntaxKind::Placement);
                    self.then([Task::PlacementHead, Task::ParenAfterHead]);
                } else {
                    let found = self.describe();
                    self.error(
                        Code::PlacementSyntax,
                        format!("expected a placement inside the parentheses, found {found}"),
                    );
                    self.then([PAREN_CLOSE]);
                }
            }
            Some(SyntaxKind::LBracket) => self.then([Task::Bundle]),
            _ => {
                self.start(SyntaxKind::Placement);
                self.then([Task::PlacementHead, Task::Finish]);
            }
        }
    }

    pub(super) fn paren_after_head(&mut self) {
        if self.at(SyntaxKind::WhenKw) {
            self.then([Task::Gate, Task::ParenAfterGate]);
        } else {
            self.then([Task::ParenAfterGate]);
        }
    }

    pub(super) fn paren_after_gate(&mut self) {
        if self.at(SyntaxKind::Pipe) {
            self.then([Task::Attrs, Task::ParenFinishPlacement]);
        } else {
            self.then([Task::ParenFinishPlacement]);
        }
    }

    pub(super) fn paren_finish_placement(&mut self) {
        self.finish();
        self.then([PAREN_CLOSE]);
    }

    pub(super) fn placement_head(&mut self) {
        if self.at_wire_kind() {
            let name = self.current_text();
            self.error(
                Code::MisplacedWire,
                format!("`{name}` is a wire kind; a wire placement stands on its own line"),
            );
        }
        let kind = self.kind_token();
        self.bump();
        self.flags();
        if self.eat(SyntaxKind::AsKw) {
            self.expect_name(Code::PlacementSyntax, "the name after `as`");
        }
        self.then([Task::DefaultArg { kind }]);
    }

    fn flags(&mut self) {
        if !(self.adjacent() && self.at_any(FLAG_TOKENS)) {
            return;
        }
        self.start(SyntaxKind::Flags);
        while self.adjacent() && self.at_any(FLAG_TOKENS) {
            self.bump();
        }
        self.finish();
        if self.adjacent()
            && self
                .current_text()
                .chars()
                .next()
                .is_some_and(is_ident_continue)
        {
            self.error(
                Code::FlagSyntax,
                "an identifier character directly after flags; separate them with a space",
            );
        }
    }

    fn kind_token(&mut self) -> usize {
        self.significant(0).unwrap_or(self.pos)
    }

    pub(super) fn default_arg(&mut self, kind: usize) {
        if !self.at(SyntaxKind::Slash) {
            return;
        }
        self.start(SyntaxKind::DefaultArg);
        let slash = self.significant(0).unwrap_or(self.pos);
        self.bump();
        if self.at(SyntaxKind::LParen) {
            self.start(SyntaxKind::ParenExpr);
            self.bump();
            self.then([
                Task::Expr,
                Task::Close {
                    kind: SyntaxKind::RParen,
                    code: Code::DefaultArgSyntax,
                    what: "`)`",
                },
                Task::Finish,
            ]);
        } else {
            let atom = self.significant(0).unwrap_or(self.pos);
            self.then([Task::Atom, Task::DefaultAfterAtom { kind, slash, atom }]);
        }
    }

    pub(super) fn default_after_atom(&mut self, kind: usize, slash: usize, atom: usize) {
        if !self.line_error
            && self.adjacent()
            && self.at_any(&[SyntaxKind::LBracket, SyntaxKind::LParen])
        {
            let found = self.describe();
            let example = match self.closed_groups() {
                Some(last) => {
                    let prefix = self.text_between(kind, slash);
                    let value = self.text_between(atom, last);
                    format!("{prefix}({value})")
                }
                None => {
                    let kind = self.text_of(kind);
                    format!("{kind}/(…)")
                }
            };
            let diagnostic = Diagnostic::new(
                Code::DefaultArgSyntax,
                self.current_span(),
                format!("an unparenthesized `/` value ends before {found}"),
            )
            .with_hint(format!("parenthesize the whole value, as in `{example}`"));
            self.error_with(diagnostic);
        }
        self.finish();
    }

    pub(super) fn atom(&mut self) {
        match self.peek() {
            Some(SyntaxKind::Minus) if self.nth(1) == Some(SyntaxKind::Number) => {
                self.start(SyntaxKind::PrefixExpr);
                self.bump();
                self.start(SyntaxKind::Literal);
                self.bump();
                self.finish();
                self.finish();
            }
            Some(SyntaxKind::Number | SyntaxKind::TrueKw | SyntaxKind::FalseKw) => {
                self.start(SyntaxKind::Literal);
                self.bump();
                self.finish();
            }
            Some(SyntaxKind::StringStart) => self.then([Task::StringLit]),
            Some(SyntaxKind::Ident) if self.nth(1) == Some(SyntaxKind::Colon2) => {
                self.variant_ref();
            }
            Some(SyntaxKind::Ident) => {
                let checkpoint = self.checkpoint();
                self.start(SyntaxKind::NameRef);
                self.bump();
                self.finish();
                while self.at(SyntaxKind::Dot) && self.nth(1) == Some(SyntaxKind::Ident) {
                    self.start_at(checkpoint, SyntaxKind::FieldExpr);
                    self.bump();
                    self.bump();
                    self.finish();
                }
            }
            Some(SyntaxKind::ToKw) => {
                let checkpoint = self.checkpoint();
                self.start(SyntaxKind::ToExpr);
                self.bump();
                self.finish();
                self.start_at(checkpoint, SyntaxKind::FieldExpr);
                self.expect(
                    SyntaxKind::Dot,
                    Code::DefaultArgSyntax,
                    "`.` and a name after `to`",
                );
                self.expect_name(Code::DefaultArgSyntax, "the name after `to.`");
                self.finish();
            }
            _ => {
                let found = self.describe();
                self.error(
                    Code::DefaultArgSyntax,
                    format!(
                        "expected a literal, a name, a path or a parenthesized expression after `/`, found {found}"
                    ),
                );
            }
        }
    }

    pub(super) fn gate(&mut self) {
        self.start(SyntaxKind::Gate);
        self.bump();
        self.then([Task::Expr, Task::Finish]);
    }

    pub(super) fn attrs(&mut self) {
        self.start(SyntaxKind::Attrs);
        self.bump();
        if !self.at_any(&[SyntaxKind::Ident, SyntaxKind::Bang]) {
            let found = self.describe();
            self.error(
                Code::AttributeSyntax,
                format!("expected an attribute after `|`, found {found}"),
            );
        }
        self.then([Task::AttrsLoop]);
    }

    pub(super) fn attrs_loop(&mut self) {
        if !self.at_any(&[SyntaxKind::Ident, SyntaxKind::Bang]) {
            self.finish();
            return;
        }
        self.start(SyntaxKind::AttrEntry);
        if self.eat(SyntaxKind::Bang) {
            self.expect_name(Code::AttributeSyntax, "the attribute after `!`");
            if self.at(SyntaxKind::Eq) {
                self.error(
                    Code::AttributeSyntax,
                    "`!name` sets an attribute to false and takes no value",
                );
            }
            self.finish();
            self.then([Task::AttrsLoop]);
        } else {
            self.bump();
            if self.eat(SyntaxKind::Eq) {
                self.then([Task::Expr, Task::Finish, Task::AttrsLoop]);
            } else {
                self.finish();
                self.then([Task::AttrsLoop]);
            }
        }
    }

    pub(super) fn bundle(&mut self) {
        self.start(SyntaxKind::Bundle);
        self.bump();
        self.then([Task::BundleLoop { members: 0 }]);
    }

    pub(super) fn bundle_loop(&mut self, members: usize) {
        if self.at_any(MEMBER_START) {
            if self.at(SyntaxKind::LBracket) {
                self.error(
                    Code::BundleSyntax,
                    "a bundle is not a member of another bundle",
                );
            }
            self.then([
                Task::Member,
                Task::BundleLoop {
                    members: members + 1,
                },
            ]);
        } else if self.at(SyntaxKind::ForKw) {
            self.start(SyntaxKind::BundleFor);
            self.bump();
            self.expect_name(Code::BundleSyntax, "the loop variable");
            self.expect(SyntaxKind::InKw, Code::BundleSyntax, "`in`");
            self.then([Task::Iterable, Task::BundleForAfterIterable { members }]);
        } else {
            self.then([Task::BundleEnd { members }]);
        }
    }

    pub(super) fn bundle_for_after_iterable(&mut self, members: usize) {
        self.expect(SyntaxKind::Colon, Code::BundleSyntax, "`:` and the members");
        if !self.at_any(MEMBER_START) {
            let found = self.describe();
            self.error(
                Code::BundleSyntax,
                format!("expected a member, found {found}"),
            );
        }
        self.then([Task::BundleForMembers { members }]);
    }

    pub(super) fn bundle_for_members(&mut self, members: usize) {
        if self.at_any(MEMBER_START) {
            self.then([Task::Member, Task::BundleForMembers { members }]);
        } else {
            self.finish();
            self.then([Task::BundleEnd {
                members: members + 1,
            }]);
        }
    }

    pub(super) fn bundle_end(&mut self, members: usize) {
        if members == 0 {
            let found = self.describe();
            self.error(
                Code::BundleSyntax,
                format!("expected a member of the bundle, found {found}"),
            );
        }
        self.expect(SyntaxKind::RBracket, Code::BundleSyntax, "`]`");
        self.finish();
    }
}

const PAREN_CLOSE: Task = Task::Close {
    kind: SyntaxKind::RParen,
    code: Code::PlacementSyntax,
    what: "`)`",
};
