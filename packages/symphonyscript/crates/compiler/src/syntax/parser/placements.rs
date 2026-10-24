use super::Parser;
use crate::diagnostics::Code;
use crate::diagnostics::Diagnostic;
use crate::syntax::lexer::is_ident_continue;
use crate::syntax::syntax_kind::SyntaxKind;

const FLAG_TOKENS: &[SyntaxKind] = &[
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
        self.bump();
        self.flags();
        self.eat(SyntaxKind::AsKw);
        self.expect_name(Code::PlacementSyntax, "the top-level instance's name");
        self.default_arg_if_any();
        self.placement_tail();
        self.finish();
    }

    pub(super) fn wire_placement(&mut self) {
        self.start(SyntaxKind::WirePlacement);
        self.bump();
        self.flags();
        if self.eat(SyntaxKind::AsKw) {
            self.expect_name(Code::WireSyntax, "the wire's name");
        }
        self.default_arg_if_any();
        if self.at(SyntaxKind::WhenKw) {
            self.gate();
        }
        if self.at(SyntaxKind::Pipe) {
            self.attrs();
        }
        if self.expect(
            SyntaxKind::Colon,
            Code::WireSyntax,
            "`:` and the wire's destination",
        ) {
            self.start(SyntaxKind::Destination);
            if self.at(SyntaxKind::MatchKw) {
                self.match_block();
            } else {
                self.expr();
                self.finish_line(Code::WireSyntax);
            }
            self.finish();
        } else {
            self.finish_line(Code::WireSyntax);
        }
        self.finish();
    }

    pub(super) fn line(&mut self) {
        let checkpoint = self.checkpoint();
        if self.at(SyntaxKind::Ident) {
            self.start(SyntaxKind::Placement);
            self.placement_head();
            if !self.at_any(MEMBER_START) {
                self.placement_tail();
                self.finish();
                return;
            }
            self.finish();
        }
        self.start_at(checkpoint, SyntaxKind::Line);
        while self.at_any(MEMBER_START) {
            self.member();
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

    fn placement_tail(&mut self) {
        if self.at(SyntaxKind::WhenKw) {
            self.gate();
        }
        if self.at(SyntaxKind::Pipe) {
            self.attrs();
        }
        if self.at(SyntaxKind::Colon) {
            self.start(SyntaxKind::Body);
            self.bump();
            self.block(false);
            self.finish();
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
        while self.at_any(MEMBER_START) {
            self.member();
        }
        self.finish();
    }

    fn member(&mut self) {
        match self.peek() {
            Some(SyntaxKind::LParen) => {
                self.start(SyntaxKind::ParenPlacement);
                self.bump();
                if self.at(SyntaxKind::Ident) {
                    self.start(SyntaxKind::Placement);
                    self.placement_head();
                    if self.at(SyntaxKind::WhenKw) {
                        self.gate();
                    }
                    if self.at(SyntaxKind::Pipe) {
                        self.attrs();
                    }
                    self.finish();
                } else {
                    let found = self.describe();
                    self.error(
                        Code::PlacementSyntax,
                        format!("expected a placement inside the parentheses, found {found}"),
                    );
                }
                self.expect(SyntaxKind::RParen, Code::PlacementSyntax, "`)`");
                self.finish();
            }
            Some(SyntaxKind::LBracket) => self.bundle(),
            _ => {
                self.start(SyntaxKind::Placement);
                self.placement_head();
                self.finish();
            }
        }
    }

    fn placement_head(&mut self) {
        if self.at_wire_kind() {
            let name = self.current_text();
            self.error(
                Code::MisplacedWire,
                format!("`{name}` is a wire kind; a wire placement stands on its own line"),
            );
        }
        self.bump();
        self.flags();
        if self.eat(SyntaxKind::AsKw) {
            self.expect_name(Code::PlacementSyntax, "the name after `as`");
        }
        self.default_arg_if_any();
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

    fn default_arg_if_any(&mut self) {
        if !self.at(SyntaxKind::Slash) {
            return;
        }
        self.start(SyntaxKind::DefaultArg);
        self.bump();
        if self.at(SyntaxKind::LParen) {
            self.start(SyntaxKind::ParenExpr);
            self.bump();
            self.expr();
            self.expect(SyntaxKind::RParen, Code::DefaultArgSyntax, "`)`");
            self.finish();
        } else {
            self.atom();
            if self.adjacent() && self.at_any(&[SyntaxKind::LBracket, SyntaxKind::LParen]) {
                let found = self.describe();
                let diagnostic = Diagnostic::new(
                    Code::DefaultArgSyntax,
                    self.current_span(),
                    format!("an unparenthesized `/` value ends before {found}"),
                )
                .with_hint("parenthesize the whole value, as in `Note/(a[0])`");
                self.error_with(diagnostic);
            }
        }
        self.finish();
    }

    fn atom(&mut self) {
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
            Some(SyntaxKind::StringStart) => self.string_lit(),
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

    fn gate(&mut self) {
        self.start(SyntaxKind::Gate);
        self.bump();
        self.expr();
        self.finish();
    }

    fn attrs(&mut self) {
        self.start(SyntaxKind::Attrs);
        self.bump();
        if !self.at_any(&[SyntaxKind::Ident, SyntaxKind::Bang]) {
            let found = self.describe();
            self.error(
                Code::AttributeSyntax,
                format!("expected an attribute after `|`, found {found}"),
            );
        }
        while self.at_any(&[SyntaxKind::Ident, SyntaxKind::Bang]) {
            self.start(SyntaxKind::AttrEntry);
            if self.eat(SyntaxKind::Bang) {
                self.expect_name(Code::AttributeSyntax, "the attribute after `!`");
                if self.at(SyntaxKind::Eq) {
                    self.error(
                        Code::AttributeSyntax,
                        "`!name` sets an attribute to false and takes no value",
                    );
                }
            } else {
                self.bump();
                if self.eat(SyntaxKind::Eq) {
                    self.expr();
                }
            }
            self.finish();
        }
        self.finish();
    }

    fn bundle(&mut self) {
        self.start(SyntaxKind::Bundle);
        self.bump();
        let mut members = 0;
        while self.at_any(MEMBER_START) {
            if self.at(SyntaxKind::LBracket) {
                self.error(
                    Code::BundleSyntax,
                    "a bundle is not a member of another bundle",
                );
            }
            self.member();
            members += 1;
        }
        if self.at(SyntaxKind::ForKw) {
            self.start(SyntaxKind::BundleFor);
            self.bump();
            self.expect_name(Code::BundleSyntax, "the loop variable");
            self.expect(SyntaxKind::InKw, Code::BundleSyntax, "`in`");
            self.iterable();
            self.expect(SyntaxKind::Colon, Code::BundleSyntax, "`:` and the members");
            if !self.at_any(MEMBER_START) {
                let found = self.describe();
                self.error(
                    Code::BundleSyntax,
                    format!("expected a member, found {found}"),
                );
            }
            while self.at_any(MEMBER_START) {
                self.member();
            }
            self.finish();
            members += 1;
        }
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
