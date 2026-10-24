use super::Parser;
use crate::diagnostics::Code;
use crate::syntax::syntax_kind::SyntaxKind;

impl Parser<'_> {
    pub(super) fn block(&mut self, pattern: bool) {
        self.start(SyntaxKind::Block);
        if self.eat(SyntaxKind::Newline) {
            if self.eat(SyntaxKind::Indent) {
                if pattern {
                    while self.at(SyntaxKind::LetKw) {
                        self.let_decl(SyntaxKind::LocalLet);
                    }
                }
                let mut items = 0;
                while !self.at_end() && !self.at(SyntaxKind::Dedent) {
                    if self.at(SyntaxKind::LetKw) {
                        let code = if pattern {
                            Code::PatternSyntax
                        } else {
                            Code::LetSyntax
                        };
                        self.error(
                            code,
                            "a `let` line stands only at the start of a pattern body",
                        );
                        self.let_decl(SyntaxKind::LocalLet);
                        continue;
                    }
                    self.body_item();
                    items += 1;
                }
                if items == 0 {
                    self.error(
                        Code::PlacementSyntax,
                        "a block holds at least one body item",
                    );
                }
                self.eat(SyntaxKind::Dedent);
            } else {
                let found = self.describe();
                self.error(
                    Code::PlacementSyntax,
                    format!("expected an indented block, found {found}"),
                );
            }
        } else {
            self.members(SyntaxKind::Members);
            self.finish_line(Code::InlinePlacement);
        }
        self.finish();
    }

    fn body_item(&mut self) {
        match self.peek() {
            Some(SyntaxKind::ForKw) => self.for_block(),
            Some(SyntaxKind::WhenKw) => self.when_block(),
            Some(SyntaxKind::Ident) if self.at_wire_kind() => self.wire_placement(),
            Some(SyntaxKind::Ident | SyntaxKind::LParen | SyntaxKind::LBracket) => self.line(),
            Some(SyntaxKind::Indent) => self.unexpected_indent(),
            _ => {
                let found = self.describe();
                self.error(
                    Code::PlacementSyntax,
                    format!("expected a placement, found {found}"),
                );
                self.skip_line();
                self.eat(SyntaxKind::Newline);
            }
        }
    }

    fn for_block(&mut self) {
        self.start(SyntaxKind::ForBlock);
        self.bump();
        self.expect_name(Code::ForSyntax, "the loop variable");
        self.expect(SyntaxKind::InKw, Code::ForSyntax, "`in`");
        self.iterable();
        if self.expect(SyntaxKind::Colon, Code::ForSyntax, "`:` and the body") {
            self.block(false);
        } else {
            self.finish_line(Code::ForSyntax);
        }
        self.finish();
    }

    pub(super) fn iterable(&mut self) {
        let checkpoint = self.checkpoint();
        self.expr();
        if self.at(SyntaxKind::Dot2) {
            self.start_at(checkpoint, SyntaxKind::RangeExpr);
            self.bump();
            self.expr();
            self.finish();
        }
    }

    fn when_block(&mut self) {
        self.start(SyntaxKind::WhenBlock);
        self.bump();
        if self.eat(SyntaxKind::Colon) {
            if !self.expect(
                SyntaxKind::Newline,
                Code::WhenSyntax,
                "the guards on the following lines",
            ) {
                self.skip_line();
                self.eat(SyntaxKind::Newline);
            }
            if self.eat(SyntaxKind::Indent) {
                let mut arms = 0;
                while !self.at_end() && !self.at(SyntaxKind::Dedent) {
                    if self.at(SyntaxKind::ElseKw) {
                        self.when_else();
                        if !self.at(SyntaxKind::Dedent) && !self.at_end() {
                            self.error(
                                Code::WhenSyntax,
                                "`else` is the last arm of a `when` block",
                            );
                        }
                        continue;
                    }
                    self.start(SyntaxKind::GuardArm);
                    self.expr();
                    if self.expect(SyntaxKind::Colon, Code::WhenSyntax, "`:` after the guard") {
                        self.block(false);
                    } else {
                        self.finish_line(Code::WhenSyntax);
                    }
                    self.finish();
                    arms += 1;
                }
                if arms == 0 {
                    self.error(Code::WhenSyntax, "a `when:` block holds at least one guard");
                }
                self.eat(SyntaxKind::Dedent);
            } else {
                self.error(Code::WhenSyntax, "expected the guards, indented");
            }
        } else {
            self.expr();
            if self.expect(
                SyntaxKind::Colon,
                Code::WhenSyntax,
                "`:` after the condition",
            ) {
                self.block(false);
                if self.at(SyntaxKind::ElseKw) {
                    self.when_else();
                }
            } else {
                self.finish_line(Code::WhenSyntax);
            }
        }
        self.finish();
    }

    fn when_else(&mut self) {
        self.start(SyntaxKind::WhenElse);
        self.bump();
        if self.expect(SyntaxKind::Colon, Code::WhenSyntax, "`:` after `else`") {
            self.block(false);
        } else {
            self.finish_line(Code::WhenSyntax);
        }
        self.finish();
    }

    pub(super) fn match_block(&mut self) {
        self.start(SyntaxKind::MatchBlock);
        self.bump();
        self.expr();
        let colon = self.expect(
            SyntaxKind::Colon,
            Code::MatchSyntax,
            "`:` after the matched value",
        );
        if !self.at(SyntaxKind::Newline) {
            self.error(
                Code::MatchSyntax,
                "a match's arms start on the following lines",
            );
            self.skip_line();
        }
        self.eat(SyntaxKind::Newline);
        if self.eat(SyntaxKind::Indent) {
            let mut arms = 0;
            let mut after_else = false;
            while !self.at_end() && !self.at(SyntaxKind::Dedent) {
                if after_else {
                    self.error(Code::MatchSyntax, "`else:` is the last arm of a match");
                }
                if self.at(SyntaxKind::ElseKw) {
                    self.start(SyntaxKind::ElseArm);
                    self.bump();
                    after_else = true;
                } else {
                    self.start(SyntaxKind::MatchArm);
                    self.match_label();
                    arms += 1;
                }
                self.expect(
                    SyntaxKind::Colon,
                    Code::MatchSyntax,
                    "`:` after the arm label",
                );
                self.expr();
                self.finish_line(Code::MatchSyntax);
                self.finish();
            }
            if arms == 0 {
                self.error(
                    Code::MatchSyntax,
                    "a match holds at least one arm before `else:`",
                );
            }
            self.eat(SyntaxKind::Dedent);
        } else if colon {
            self.error(Code::MatchSyntax, "expected the match arms, indented");
        }
        self.finish();
    }

    fn match_label(&mut self) {
        match self.peek() {
            Some(SyntaxKind::Ident) => {
                self.start(SyntaxKind::NameRef);
                self.bump();
                self.finish();
            }
            Some(SyntaxKind::Number | SyntaxKind::TrueKw | SyntaxKind::FalseKw) => {
                self.start(SyntaxKind::Literal);
                self.bump();
                self.finish();
            }
            Some(SyntaxKind::Minus) if self.nth(1) == Some(SyntaxKind::Number) => {
                self.start(SyntaxKind::PrefixExpr);
                self.bump();
                self.start(SyntaxKind::Literal);
                self.bump();
                self.finish();
                self.finish();
            }
            _ => {
                let found = self.describe();
                self.error(
                    Code::MatchSyntax,
                    format!("expected an arm label, a variant name or a literal, found {found}"),
                );
            }
        }
    }
}
