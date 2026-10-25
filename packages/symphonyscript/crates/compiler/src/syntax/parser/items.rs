use super::Parser;
use super::task::Task;
use crate::diagnostics::Code;
use crate::syntax::syntax_kind::SyntaxKind;

impl Parser<'_> {
    pub(super) fn file_step(&mut self) {
        match self.peek() {
            None => {}
            Some(SyntaxKind::Indent) => {
                self.unexpected_indent();
                self.then([Task::File]);
            }
            Some(SyntaxKind::Newline | SyntaxKind::Dedent) => {
                self.skip_token();
                self.then([Task::File]);
            }
            Some(_) => self.then([Task::TopItem, Task::File]),
        }
    }

    pub(super) fn top_item(&mut self) {
        match self.peek() {
            Some(SyntaxKind::InputKw) => self.then([Task::InputDecl]),
            Some(SyntaxKind::LetKw) => self.then([Task::LetDecl(SyntaxKind::LetDecl)]),
            Some(SyntaxKind::StateKw) => self.then([Task::LetDecl(SyntaxKind::StateDecl)]),
            Some(SyntaxKind::FnKw) => self.then([Task::FnDecl]),
            Some(SyntaxKind::PatternKw) => self.then([Task::PatternDecl]),
            Some(SyntaxKind::Ident) if self.at_wire_kind() => {
                let name = self.current_text();
                self.error(
                    Code::TopLevelWire,
                    format!(
                        "`{name}` is a wire kind, and a wire is placed inside its source's body"
                    ),
                );
                self.then([Task::WirePlacement]);
            }
            Some(SyntaxKind::Ident) => self.then([Task::TopPlacement]),
            _ => {
                let found = self.describe();
                self.error(
                    Code::TopLevelSyntax,
                    format!("expected a declaration or a placement, found {found}"),
                );
                self.skip_line();
                self.eat(SyntaxKind::Newline);
            }
        }
    }

    pub(super) fn input_decl(&mut self) {
        self.start(SyntaxKind::InputDecl);
        self.bump();
        self.expect_name(Code::InputSyntax, "the input's name");
        if self.eat(SyntaxKind::Colon) {
            self.type_ref();
        }
        self.expect(
            SyntaxKind::Eq,
            Code::InputSyntax,
            "`=` and the initial value",
        );
        self.then([Task::Expr, Task::FinishLineFinish(Code::InputSyntax)]);
    }

    pub(super) fn let_decl(&mut self, kind: SyntaxKind) {
        let state = kind == SyntaxKind::StateDecl;
        let code = if state {
            Code::StateSyntax
        } else {
            Code::LetSyntax
        };
        self.start(kind);
        self.bump();
        if state && self.eat(SyntaxKind::LBracket) {
            self.digits(code, "the deepest offset in digits");
            self.expect(SyntaxKind::RBracket, code, "`]`");
        }
        if state {
            self.expect_name(code, "the state's name");
        } else {
            self.expect_name(code, "the let's name");
        }
        if kind != SyntaxKind::LocalLet && self.eat(SyntaxKind::Colon) {
            self.type_ref();
        }
        if !self.eat(SyntaxKind::Eq) {
            self.match_after_fault(SyntaxKind::Eq, code, "`=` and the value");
        }
        if self.at(SyntaxKind::MatchKw) {
            self.then([Task::MatchBlock, Task::Finish]);
        } else {
            self.then([Task::Expr, Task::FinishLineFinish(code)]);
        }
    }

    fn digits(&mut self, code: Code, what: &str) -> bool {
        let plain = self.at(SyntaxKind::Number)
            && self
                .current_text()
                .bytes()
                .all(|byte| byte.is_ascii_digit() || byte == b'_');
        if plain {
            self.bump();
            return true;
        }
        let found = self.describe();
        self.error(code, format!("expected {what}, found {found}"));
        if self.at(SyntaxKind::Number) {
            self.bump();
        }
        false
    }

    pub(super) fn fn_decl(&mut self) {
        self.start(SyntaxKind::FnDecl);
        self.bump();
        self.expect_name(Code::FnSyntax, "the function's name");
        self.param_list(
            SyntaxKind::ParamList,
            Code::FnSyntax,
            false,
            Task::FnAfterParams,
        );
    }

    pub(super) fn pattern_decl(&mut self) {
        self.start(SyntaxKind::PatternDecl);
        self.bump();
        self.expect_name(Code::PatternSyntax, "the pattern's name");
        self.param_list(
            SyntaxKind::PatternParamList,
            Code::PatternSyntax,
            true,
            Task::PatternAfterParams,
        );
    }

    fn param_list(&mut self, kind: SyntaxKind, code: Code, pattern: bool, after: Task) {
        self.start(kind);
        let close = Task::Close {
            kind: SyntaxKind::RParen,
            code,
            what: "`)`",
        };
        if self.expect(SyntaxKind::LParen, code, "`(` and the parameters") {
            if self.at(SyntaxKind::RParen) {
                self.then([close, after]);
            } else {
                self.then([Task::Param { pattern }, close, after]);
            }
        } else {
            self.finish();
            self.then([after]);
        }
    }

    pub(super) fn param(&mut self, pattern: bool) {
        let (kind, code) = if pattern {
            (SyntaxKind::PatternParam, Code::PatternSyntax)
        } else {
            (SyntaxKind::Param, Code::FnSyntax)
        };
        self.start(kind);
        if pattern {
            self.eat(SyntaxKind::DefaultKw);
        }
        self.expect_name(code, "a parameter name");
        if self.eat(SyntaxKind::Eq) {
            self.then([Task::Expr, Task::ParamAfter { pattern }]);
        } else {
            self.then([Task::ParamAfter { pattern }]);
        }
    }

    pub(super) fn param_after(&mut self, pattern: bool) {
        self.finish();
        if self.eat(SyntaxKind::Comma) {
            self.then([Task::Param { pattern }]);
        }
    }

    pub(super) fn fn_after_params(&mut self) {
        if !self.at(SyntaxKind::BoundedKw) {
            self.fn_colon();
            return;
        }
        self.start(SyntaxKind::Bounded);
        self.bump();
        let zero = self.at(SyntaxKind::Number)
            && self
                .current_text()
                .bytes()
                .all(|byte| byte == b'0' || byte == b'_');
        if zero {
            self.error(Code::FnSyntax, "a bound is a positive integer");
            self.bump();
        } else {
            self.digits(Code::FnSyntax, "the bound, a positive integer in digits");
        }
        self.expect(
            SyntaxKind::ElseKw,
            Code::FnSyntax,
            "`else` and the fallback",
        );
        self.then([Task::Expr, Task::Finish, Task::FnColon]);
    }

    pub(super) fn fn_colon(&mut self) {
        if self.colon_or_skip(Code::FnSyntax, "`:` and the body") {
            self.bump();
            self.then([Task::FnBody, Task::Finish]);
        } else {
            self.finish();
        }
    }

    pub(super) fn fn_body(&mut self) {
        self.start(SyntaxKind::FnBody);
        if self.eat(SyntaxKind::Newline) {
            if self.eat(SyntaxKind::Indent) {
                self.then([Task::FnLets]);
            } else {
                self.missing_block(Code::FnSyntax, "expected the function body, indented");
                self.finish();
            }
        } else {
            self.then([Task::Expr, Task::FinishLineFinish(Code::FnSyntax)]);
        }
    }

    pub(super) fn fn_lets(&mut self) {
        if self.at(SyntaxKind::LetKw) {
            self.then([Task::LetDecl(SyntaxKind::LocalLet), Task::FnLets]);
        } else if self.at(SyntaxKind::MatchKw) {
            self.then([Task::MatchBlock, Task::FnBodyBlockEnd]);
        } else {
            self.then([
                Task::Expr,
                Task::FinishLine(Code::FnSyntax),
                Task::FnBodyBlockEnd,
            ]);
        }
    }

    pub(super) fn fn_body_block_end(&mut self) {
        self.rest_of_block(Code::FnSyntax, "a function body ends with one value");
        self.finish();
    }

    pub(super) fn pattern_after_params(&mut self) {
        if self.colon_or_skip(Code::PatternSyntax, "`:` and the body") {
            self.bump();
            if self.at(SyntaxKind::Newline) {
                self.then([Task::Block { pattern: true }, Task::Finish]);
            } else {
                self.error(
                    Code::PatternSyntax,
                    "a pattern's body is an indented block on the following lines",
                );
                self.skip_line();
                self.eat(SyntaxKind::Newline);
                self.finish();
            }
        } else {
            self.finish();
        }
    }

    pub(super) fn rest_of_block(&mut self, code: Code, message: &str) {
        let mut reported = false;
        while !self.at_end() && !self.at(SyntaxKind::Dedent) {
            if !reported {
                self.error(code, message);
                reported = true;
            }
            if self.at(SyntaxKind::Indent) {
                self.skip_block();
            } else {
                self.skip_line();
                self.eat(SyntaxKind::Newline);
            }
        }
        self.eat(SyntaxKind::Dedent);
    }

    pub(super) fn type_ref(&mut self) {
        self.start(SyntaxKind::TypeRef);
        if self.at_any(&[
            SyntaxKind::U8Kw,
            SyntaxKind::I32Kw,
            SyntaxKind::I64Kw,
            SyntaxKind::F32Kw,
            SyntaxKind::F64Kw,
            SyntaxKind::BoolKw,
            SyntaxKind::Ident,
        ]) {
            self.bump();
        } else {
            let found = self.describe();
            self.error(Code::TypeSyntax, format!("expected a type, found {found}"));
        }
        while self.at(SyntaxKind::LBracket) {
            if self.nth(1) == Some(SyntaxKind::Dot2) {
                self.bump();
                self.bump();
                self.digits(Code::TypeSyntax, "a capacity in digits");
                self.expect(SyntaxKind::RBracket, Code::TypeSyntax, "`]`");
                if self.at(SyntaxKind::LBracket) {
                    self.error(
                        Code::TypeSyntax,
                        "only the last, outermost length of a type may be a capacity",
                    );
                }
                break;
            }
            self.bump();
            self.digits(Code::TypeSyntax, "an array length in digits");
            self.expect(SyntaxKind::RBracket, Code::TypeSyntax, "`]`");
        }
        self.finish();
    }
}
