use super::Parser;
use super::task::Task;
use crate::diagnostics::Code;
use crate::syntax::syntax_kind::SyntaxKind;

impl Parser<'_> {
    pub(super) fn json_lit(&mut self) {
        self.start(SyntaxKind::JsonLit);
        self.bump();
        self.then([Task::JsonValue, Task::Finish]);
    }

    pub(super) fn json_value(&mut self) {
        match self.peek() {
            Some(SyntaxKind::LBrace) => {
                self.start(SyntaxKind::JsonObject);
                self.bump();
                if self.at(SyntaxKind::RBrace) {
                    self.then([OBJECT_CLOSE]);
                } else {
                    self.then([Task::JsonEntry, Task::JsonObjectLoop]);
                }
            }
            Some(SyntaxKind::LBracket) => {
                self.start(SyntaxKind::JsonArray);
                self.bump();
                if self.at(SyntaxKind::RBracket) {
                    self.then([ARRAY_CLOSE]);
                } else {
                    self.then([Task::JsonValue, Task::JsonArrayLoop]);
                }
            }
            Some(SyntaxKind::Minus) => {
                self.start(SyntaxKind::JsonAtom);
                self.bump();
                if !self.eat(SyntaxKind::JsonNumber) {
                    let found = self.describe();
                    self.error(
                        Code::JsonSyntax,
                        format!("expected a number after `-` in JSON text, found {found}"),
                    );
                }
                self.finish();
            }
            Some(
                SyntaxKind::JsonString
                | SyntaxKind::JsonNumber
                | SyntaxKind::TrueKw
                | SyntaxKind::FalseKw
                | SyntaxKind::JsonNull,
            ) => {
                self.start(SyntaxKind::JsonAtom);
                self.bump();
                self.finish();
            }
            Some(SyntaxKind::Ident | SyntaxKind::ToKw) => self.json_path(),
            Some(SyntaxKind::LParen) => {
                self.start(SyntaxKind::ParenExpr);
                self.bump();
                self.then([
                    Task::ParenValue,
                    Task::Close {
                        kind: SyntaxKind::RParen,
                        code: Code::JsonSyntax,
                        what: "`)`",
                    },
                ]);
            }
            _ => {
                let found = self.describe();
                self.error(
                    Code::JsonSyntax,
                    format!("expected a JSON value, found {found}"),
                );
            }
        }
    }

    fn json_path(&mut self) {
        let checkpoint = self.checkpoint();
        if self.at(SyntaxKind::ToKw) {
            self.start(SyntaxKind::ToExpr);
            self.bump();
            self.finish();
            if !self.at_dot_member() {
                let found = self.describe();
                self.error(
                    Code::JsonSyntax,
                    format!("expected `.` and a name after `to`, found {found}"),
                );
            }
        } else if self.nth(1) == Some(SyntaxKind::Colon2) {
            self.qualified_name(Code::JsonSyntax);
        } else {
            self.start(SyntaxKind::NameRef);
            self.bump();
            self.finish();
        }
        while self.at_dot_member() {
            self.start_at(checkpoint, SyntaxKind::FieldExpr);
            self.bump();
            self.expect_member_name(Code::JsonSyntax, "a name after `.`");
            self.finish();
        }
    }

    pub(super) fn json_entry(&mut self) {
        self.start(SyntaxKind::JsonEntry);
        self.expect(
            SyntaxKind::JsonString,
            Code::JsonSyntax,
            "a key in double quotes",
        );
        self.expect(SyntaxKind::Colon, Code::JsonSyntax, "`:` after the key");
        self.then([Task::JsonValue, Task::Finish]);
    }

    pub(super) fn json_object_loop(&mut self) {
        if self.eat(SyntaxKind::Comma) {
            self.then([Task::JsonEntry, Task::JsonObjectLoop]);
        } else {
            self.then([OBJECT_CLOSE]);
        }
    }

    pub(super) fn json_array_loop(&mut self) {
        if self.eat(SyntaxKind::Comma) {
            self.then([Task::JsonValue, Task::JsonArrayLoop]);
        } else {
            self.then([ARRAY_CLOSE]);
        }
    }
}

const OBJECT_CLOSE: Task = Task::Close {
    kind: SyntaxKind::RBrace,
    code: Code::JsonSyntax,
    what: "`,` or `}`",
};

const ARRAY_CLOSE: Task = Task::Close {
    kind: SyntaxKind::RBracket,
    code: Code::JsonSyntax,
    what: "`,` or `]`",
};
