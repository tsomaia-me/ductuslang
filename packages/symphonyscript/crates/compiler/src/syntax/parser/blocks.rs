use super::Parser;
use super::checkpoint::Checkpoint;
use super::task::Task;
use crate::diagnostics::Code;
use crate::syntax::syntax_kind::SyntaxKind;

impl Parser<'_> {
    pub(super) fn block(&mut self, pattern: bool) {
        self.start(SyntaxKind::Block);
        if self.eat(SyntaxKind::Newline) {
            if self.eat(SyntaxKind::Indent) {
                let items = Task::BlockItems { pattern, items: 0 };
                if pattern {
                    self.then([Task::PatternLets, items]);
                } else {
                    self.then([items]);
                }
            } else {
                let found = self.describe();
                let reported = self.diagnostics.len();
                self.missing_block(
                    Code::PlacementSyntax,
                    format!("expected an indented block, found {found}"),
                );
                self.help_since(
                    reported,
                    Code::PlacementSyntax,
                    "indent the body's items on the lines after the `:`",
                );
                self.finish();
            }
        } else {
            self.then([
                Task::Members(SyntaxKind::Members),
                Task::FinishLineFinish(Code::InlinePlacement),
            ]);
        }
    }

    pub(super) fn pattern_lets(&mut self) {
        if self.at(SyntaxKind::LetKw) {
            self.then([Task::LetDecl(SyntaxKind::LocalLet), Task::PatternLets]);
        }
    }

    pub(super) fn block_items(&mut self, pattern: bool, items: usize) {
        if self.at_end() || self.at(SyntaxKind::Dedent) {
            if items == 0 {
                let reported = self.diagnostics.len();
                self.error(
                    Code::PlacementSyntax,
                    "a block holds at least one body item",
                );
                self.help_since(
                    reported,
                    Code::PlacementSyntax,
                    "put a placement, a wire, a `for` or a `when` in the block",
                );
            }
            self.eat(SyntaxKind::Dedent);
            self.finish();
        } else if self.at(SyntaxKind::LetKw) {
            let code = if pattern {
                Code::PatternSyntax
            } else {
                Code::LetSyntax
            };
            self.error(
                code,
                "a `let` line stands only at the start of a pattern body",
            );
            self.then([
                Task::LetDecl(SyntaxKind::LocalLet),
                Task::BlockItems { pattern, items },
            ]);
        } else if self.at_cell_word() {
            self.misplaced_cell();
            self.then([Task::CellDecl, Task::BlockItems { pattern, items }]);
        } else {
            self.then([
                Task::BodyItem,
                Task::BlockItems {
                    pattern,
                    items: items + 1,
                },
            ]);
        }
    }

    pub(super) fn body_item(&mut self) {
        match self.peek() {
            Some(SyntaxKind::ForKw) => self.then([Task::ForBlock]),
            Some(SyntaxKind::WhenKw) => self.then([Task::WhenBlock]),
            Some(SyntaxKind::Ident) if self.at_wire_kind() => self.then([Task::WirePlacement]),
            Some(SyntaxKind::Ident | SyntaxKind::LParen | SyntaxKind::LBracket) => {
                self.then([Task::Line]);
            }
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

    pub(super) fn for_block(&mut self) {
        self.start(SyntaxKind::ForBlock);
        self.loop_head(Code::ForSyntax);
        self.head_value("the iterable", None);
        self.then([Task::Iterable, Task::ForAfterIterable]);
    }

    pub(super) fn for_after_iterable(&mut self) {
        if self.colon_or_skip(Code::ForSyntax, "`:` and the body") {
            self.bump();
            self.then([Task::Block { pattern: false }, Task::Finish]);
        } else {
            self.finish();
        }
    }

    pub(super) fn iterable(&mut self) {
        let checkpoint = self.checkpoint();
        self.then([Task::Expr, Task::IterableEnd(checkpoint)]);
    }

    pub(super) fn iterable_end(&mut self, checkpoint: Checkpoint) {
        if self.at(SyntaxKind::Dot2) {
            self.start_at(checkpoint, SyntaxKind::RangeExpr);
            self.bump();
            self.then([Task::Expr, Task::Finish]);
        }
    }

    pub(super) fn when_block(&mut self) {
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
                self.then([Task::WhenArms { arms: 0 }]);
            } else {
                self.missing_block(Code::WhenSyntax, "expected the guards, indented");
                self.finish();
            }
        } else {
            self.then([Task::Expr, Task::WhenAfterCondition]);
        }
    }

    pub(super) fn when_arms(&mut self, arms: usize) {
        if self.at_end() || self.at(SyntaxKind::Dedent) {
            if arms == 0 {
                self.error(Code::WhenSyntax, "a `when:` block holds at least one guard");
            }
            self.eat(SyntaxKind::Dedent);
            self.finish();
        } else if self.at(SyntaxKind::ElseKw) {
            self.then([Task::WhenElse, Task::WhenAfterElse { arms }]);
        } else {
            self.start(SyntaxKind::GuardArm);
            self.then([Task::Expr, Task::GuardAfter { arms }]);
        }
    }

    pub(super) fn when_after_else(&mut self, arms: usize) {
        if !self.at(SyntaxKind::Dedent) && !self.at_end() {
            self.error(Code::WhenSyntax, "`else` is the last arm of a `when` block");
        }
        self.then([Task::WhenArms { arms }]);
    }

    pub(super) fn guard_after(&mut self, arms: usize) {
        let next = Task::WhenArms { arms: arms + 1 };
        if self.colon_or_skip(Code::WhenSyntax, "`:` after the guard") {
            self.bump();
            self.then([Task::Block { pattern: false }, Task::Finish, next]);
        } else {
            self.finish();
            self.then([next]);
        }
    }

    pub(super) fn when_after_condition(&mut self) {
        if self.colon_or_skip(Code::WhenSyntax, "`:` after the condition") {
            self.bump();
            self.then([Task::Block { pattern: false }, Task::WhenElseOptional]);
        } else {
            self.finish();
        }
    }

    pub(super) fn when_else_optional(&mut self) {
        if self.at(SyntaxKind::ElseKw) {
            self.then([Task::WhenElse, Task::Finish]);
        } else {
            self.finish();
        }
    }

    pub(super) fn when_else(&mut self) {
        self.start(SyntaxKind::WhenElse);
        self.bump();
        if self.colon_or_skip(Code::WhenSyntax, "`:` after `else`") {
            self.bump();
            self.then([Task::Block { pattern: false }, Task::Finish]);
        } else {
            self.finish();
        }
    }

    pub(super) fn match_block(&mut self) {
        self.start(SyntaxKind::MatchBlock);
        self.bump();
        self.then([Task::Expr, Task::MatchAfterScrutinee]);
    }

    pub(super) fn match_after_scrutinee(&mut self) {
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
            self.then([Task::MatchArms {
                arms: 0,
                after_else: false,
            }]);
        } else {
            if colon {
                self.missing_block(Code::MatchSyntax, "expected the match arms, indented");
            }
            self.finish();
        }
    }

    pub(super) fn match_arms(&mut self, arms: usize, after_else: bool) {
        if self.at_end() || self.at(SyntaxKind::Dedent) {
            if arms == 0 {
                self.error(
                    Code::MatchSyntax,
                    "a match holds at least one arm before `else:`",
                );
            }
            self.eat(SyntaxKind::Dedent);
            self.finish();
            return;
        }
        if after_else {
            self.error(Code::MatchSyntax, "`else:` is the last arm of a match");
        }
        let (arms, after_else) = if self.at(SyntaxKind::ElseKw) {
            self.start(SyntaxKind::ElseArm);
            self.bump();
            (arms, true)
        } else {
            self.start(SyntaxKind::MatchArm);
            self.match_label();
            (arms + 1, after_else)
        };
        self.expect(
            SyntaxKind::Colon,
            Code::MatchSyntax,
            "`:` after the arm label",
        );
        self.then([Task::Expr, Task::MatchArmEnd { arms, after_else }]);
    }

    pub(super) fn match_arm_end(&mut self, arms: usize, after_else: bool) {
        self.finish_line(Code::MatchSyntax);
        self.finish();
        self.then([Task::MatchArms { arms, after_else }]);
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
