use super::Parser;
use crate::diagnostics::Code;
use crate::syntax::syntax_kind::SyntaxKind;

impl Parser<'_> {
    pub(super) fn file(&mut self) {
        while let Some(kind) = self.peek() {
            match kind {
                SyntaxKind::Indent => self.unexpected_indent(),
                SyntaxKind::Newline | SyntaxKind::Dedent => self.skip_token(),
                _ => self.top_item(),
            }
        }
    }

    fn top_item(&mut self) {
        match self.peek() {
            Some(SyntaxKind::InputKw) => self.input_decl(),
            Some(SyntaxKind::LetKw) => self.let_decl(SyntaxKind::LetDecl),
            Some(SyntaxKind::FnKw) => self.fn_decl(),
            Some(SyntaxKind::PatternKw) => self.pattern_decl(),
            Some(SyntaxKind::Ident) if self.at_wire_kind() => {
                let name = self.current_text();
                self.error(
                    Code::TopLevelWire,
                    format!(
                        "`{name}` is a wire kind, and a wire is placed inside its source's body"
                    ),
                );
                self.wire_placement();
            }
            Some(SyntaxKind::Ident) => self.top_placement(),
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

    fn input_decl(&mut self) {
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
        self.expr();
        self.finish_line(Code::InputSyntax);
        self.finish();
    }

    pub(super) fn let_decl(&mut self, kind: SyntaxKind) {
        self.start(kind);
        self.bump();
        self.expect_name(Code::LetSyntax, "the let's name");
        if kind == SyntaxKind::LetDecl && self.eat(SyntaxKind::Colon) {
            self.type_ref();
        }
        self.expect(SyntaxKind::Eq, Code::LetSyntax, "`=` and the value");
        if self.at(SyntaxKind::MatchKw) {
            self.match_block();
        } else {
            self.expr();
            self.finish_line(Code::LetSyntax);
        }
        self.finish();
    }

    fn fn_decl(&mut self) {
        self.start(SyntaxKind::FnDecl);
        self.bump();
        self.expect_name(Code::FnSyntax, "the function's name");
        self.start(SyntaxKind::ParamList);
        if self.expect(SyntaxKind::LParen, Code::FnSyntax, "`(` and the parameters") {
            if !self.at(SyntaxKind::RParen) {
                loop {
                    self.start(SyntaxKind::Param);
                    self.expect_name(Code::FnSyntax, "a parameter name");
                    if self.eat(SyntaxKind::Eq) {
                        self.expr();
                    }
                    self.finish();
                    if !self.eat(SyntaxKind::Comma) {
                        break;
                    }
                }
            }
            self.expect(SyntaxKind::RParen, Code::FnSyntax, "`)`");
        }
        self.finish();
        if self.expect(SyntaxKind::Colon, Code::FnSyntax, "`:` and the body") {
            self.fn_body();
        } else {
            self.finish_line(Code::FnSyntax);
        }
        self.finish();
    }

    fn fn_body(&mut self) {
        self.start(SyntaxKind::FnBody);
        if self.eat(SyntaxKind::Newline) {
            if self.eat(SyntaxKind::Indent) {
                while self.at(SyntaxKind::LetKw) {
                    self.let_decl(SyntaxKind::LocalLet);
                }
                if self.at(SyntaxKind::MatchKw) {
                    self.match_block();
                } else {
                    self.expr();
                    self.finish_line(Code::FnSyntax);
                }
                self.rest_of_block(Code::FnSyntax, "a function body ends with one value");
            } else {
                self.error(Code::FnSyntax, "expected the function body, indented");
            }
        } else {
            self.expr();
            self.finish_line(Code::FnSyntax);
        }
        self.finish();
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

    fn pattern_decl(&mut self) {
        self.start(SyntaxKind::PatternDecl);
        self.bump();
        self.expect_name(Code::PatternSyntax, "the pattern's name");
        self.start(SyntaxKind::PatternParamList);
        if self.expect(
            SyntaxKind::LParen,
            Code::PatternSyntax,
            "`(` and the parameters",
        ) {
            if !self.at(SyntaxKind::RParen) {
                loop {
                    self.start(SyntaxKind::PatternParam);
                    self.eat(SyntaxKind::DefaultKw);
                    self.expect_name(Code::PatternSyntax, "a parameter name");
                    if self.eat(SyntaxKind::Eq) {
                        self.expr();
                    }
                    self.finish();
                    if !self.eat(SyntaxKind::Comma) {
                        break;
                    }
                }
            }
            self.expect(SyntaxKind::RParen, Code::PatternSyntax, "`)`");
        }
        self.finish();
        if self.expect(SyntaxKind::Colon, Code::PatternSyntax, "`:` and the body") {
            if self.at(SyntaxKind::Newline) {
                self.block(true);
            } else {
                self.error(
                    Code::PatternSyntax,
                    "a pattern's body is an indented block on the following lines",
                );
                self.skip_line();
                self.eat(SyntaxKind::Newline);
            }
        } else {
            self.finish_line(Code::PatternSyntax);
        }
        self.finish();
    }

    pub(super) fn type_ref(&mut self) {
        self.start(SyntaxKind::TypeRef);
        if self.at_any(&[
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
        while self.eat(SyntaxKind::LBracket) {
            let plain = self.at(SyntaxKind::Number)
                && self
                    .current_text()
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || byte == b'_');
            if plain {
                self.bump();
            } else {
                let found = self.describe();
                self.error(
                    Code::TypeSyntax,
                    format!("expected an array length in digits, found {found}"),
                );
                if self.at(SyntaxKind::Number) {
                    self.bump();
                }
            }
            self.expect(SyntaxKind::RBracket, Code::TypeSyntax, "`]`");
        }
        self.finish();
    }
}
