use super::Parser;
use super::task::Task;
use crate::diagnostics::Code;
use crate::syntax::syntax_kind::SyntaxKind;

impl Parser<'_> {
    pub(super) fn generic_params_then(&mut self, after: Task) {
        if !self.at(SyntaxKind::LBracket) {
            self.then([after]);
            return;
        }
        self.start(SyntaxKind::GenericParams);
        self.bump();
        self.then([Task::GenericParam, Task::GenericParamLoop, after]);
    }

    pub(super) fn generic_param(&mut self) {
        self.start(SyntaxKind::GenericParam);
        self.eat(SyntaxKind::ConstKw);
        self.expect_name(Code::GenericSyntax, "a bracket parameter's name");
        if self.eat(SyntaxKind::Colon) {
            self.then([Task::Type, Task::Finish]);
        } else {
            self.finish();
        }
    }

    pub(super) fn generic_param_loop(&mut self) {
        if self.eat(SyntaxKind::Comma) {
            self.then([Task::GenericParam, Task::GenericParamLoop]);
        } else {
            self.then([Task::Close {
                kind: SyntaxKind::RBracket,
                code: Code::GenericSyntax,
                what: "`,` or `]`",
            }]);
        }
    }

    pub(super) fn type_decl(&mut self) {
        self.start(SyntaxKind::TypeDecl);
        self.bump();
        self.expect_name(Code::RecordSyntax, "the type's name");
        self.generic_params_then(Task::TypeDeclBody);
    }

    pub(super) fn type_decl_body(&mut self) {
        let code = Code::RecordSyntax;
        if self.eat(SyntaxKind::Eq) {
            self.then([Task::Type, Task::FinishLineFinish(code)]);
            return;
        }
        if !self.colon_or_skip(code, "`=` and a type, or `:` and the fields") {
            self.finish();
            return;
        }
        self.bump();
        if !self.at(SyntaxKind::Newline) {
            let reported = self.diagnostics.len();
            self.error(code, "a record type's fields stand on the following lines");
            self.help_since(reported, code, FIELDS);
            self.skip_line();
        }
        self.eat(SyntaxKind::Newline);
        if self.eat(SyntaxKind::Indent) {
            self.then([Task::FieldDecls { fields: 0 }]);
        } else {
            self.missing_block(code, "expected the record's fields, indented");
            self.finish();
        }
    }

    pub(super) fn field_decls(&mut self, fields: usize) {
        if self.at_end() || self.at(SyntaxKind::Dedent) {
            if fields == 0 {
                self.error(Code::RecordSyntax, "a record type holds at least one field");
            }
            self.eat(SyntaxKind::Dedent);
            self.finish();
            return;
        }
        if self.at(SyntaxKind::Indent) {
            self.unexpected_indent();
            self.then([Task::FieldDecls { fields }]);
            return;
        }
        self.start(SyntaxKind::FieldDecl);
        self.expect_member_name(Code::RecordSyntax, "a field's name");
        let next = Task::FieldDecls { fields: fields + 1 };
        if self.expect(
            SyntaxKind::Colon,
            Code::RecordSyntax,
            "`:` and the field's type",
        ) {
            self.then([Task::Type, Task::FinishLineFinish(Code::RecordSyntax), next]);
        } else {
            self.skip_line();
            self.eat(SyntaxKind::Newline);
            self.finish();
            self.then([next]);
        }
    }

    pub(super) fn at_record_literal(&mut self) -> bool {
        match self.nth(1) {
            Some(SyntaxKind::LBrace) => true,
            Some(SyntaxKind::Colon2) => {
                self.nth(2) == Some(SyntaxKind::Ident) && self.nth(3) == Some(SyntaxKind::LBrace)
            }
            Some(SyntaxKind::LBracket) => {
                let Some(open) = self.significant(1) else {
                    return false;
                };
                let Some((close, _)) = self.group(open) else {
                    return false;
                };
                let mut index = close + 1;
                while let Some(kind) = self.kind_at(index) {
                    if !kind.is_trivia() {
                        return kind == SyntaxKind::LBrace;
                    }
                    index += 1;
                }
                false
            }
            Some(_) | None => false,
        }
    }

    pub(super) fn record_lit(&mut self) {
        self.start(SyntaxKind::RecordLit);
        if self.nth(1) == Some(SyntaxKind::Colon2) {
            self.qualified_type_name(Code::RecordSyntax);
        } else {
            self.bump();
        }
        if self.at(SyntaxKind::LBracket) {
            self.then([Task::TypeArgs, Task::RecordFields]);
        } else {
            self.record_fields();
        }
    }

    pub(super) fn record_fields(&mut self) {
        if !self.expect(
            SyntaxKind::LBrace,
            Code::RecordSyntax,
            "`{` and the record's fields",
        ) {
            self.finish();
            return;
        }
        if self.at(SyntaxKind::RBrace) {
            self.then([RECORD_CLOSE]);
        } else {
            self.then([Task::RecordField, Task::RecordFieldLoop]);
        }
    }

    pub(super) fn record_field(&mut self) {
        self.start(SyntaxKind::Field);
        self.expect_member_name(Code::RecordSyntax, "a field's name");
        self.expect(
            SyntaxKind::Colon,
            Code::RecordSyntax,
            "`:` and the field's value",
        );
        self.then([Task::Expr, Task::Finish]);
    }

    pub(super) fn record_field_loop(&mut self) {
        if self.eat(SyntaxKind::Comma) && !self.at(SyntaxKind::RBrace) {
            self.then([Task::RecordField, Task::RecordFieldLoop]);
        } else {
            self.then([RECORD_CLOSE]);
        }
    }

    pub(super) fn require(&mut self) {
        let code = Code::PackageSyntax;
        self.start(SyntaxKind::Require);
        self.bump();
        if self.at(SyntaxKind::StringStart) {
            self.then([Task::StringLit, Task::RequireAfter]);
        } else {
            let found = self.describe();
            let reported = self.diagnostics.len();
            self.error(
                code,
                format!("expected the library's source in quotes, found {found}"),
            );
            self.help_since(reported, code, REQUIRE);
            self.then([Task::FinishLineFinish(code)]);
        }
    }

    pub(super) fn require_after(&mut self) {
        let code = Code::PackageSyntax;
        if self.eat(SyntaxKind::AsKw) {
            self.expect_name(code, "the library's name after `as`");
        }
        self.then([Task::FinishLineFinish(code)]);
    }

    pub(super) fn use_item(&mut self) {
        let code = Code::PackageSyntax;
        self.start(SyntaxKind::Use);
        self.bump();
        self.expect_name(code, "the library's name");
        if self.eat(SyntaxKind::AsKw) {
            self.expect_name(code, "the library's name after `as`");
        } else if self.eat(SyntaxKind::Colon) {
            loop {
                self.start(SyntaxKind::UseName);
                self.expect_name(code, "a name the library publishes");
                if self.eat(SyntaxKind::AsKw) {
                    self.expect_name(code, "the name after `as`");
                }
                self.finish();
                if !self.eat(SyntaxKind::Comma) {
                    break;
                }
            }
        }
        self.then([Task::FinishLineFinish(code)]);
    }

    pub(super) fn publish(&mut self) {
        let code = Code::PackageSyntax;
        self.start(SyntaxKind::Publish);
        self.bump();
        loop {
            self.start(SyntaxKind::PublishName);
            if self.at(SyntaxKind::Ident) && self.nth(1) == Some(SyntaxKind::Colon2) {
                self.qualified_type_name(code);
            } else {
                self.expect_name(code, "a name to publish");
            }
            if self.eat(SyntaxKind::AsKw) {
                self.expect_name(code, "the name after `as`");
            }
            self.finish();
            if !self.eat(SyntaxKind::Comma) {
                break;
            }
        }
        self.then([Task::FinishLineFinish(code)]);
    }

    pub(super) fn placement_type(&mut self, code: Code) {
        self.start(SyntaxKind::PlacementType);
        if self.eat(SyntaxKind::LParen) {
            loop {
                self.placement_entry(code);
                if !self.eat(SyntaxKind::Comma) {
                    break;
                }
            }
            self.expect(SyntaxKind::RParen, code, "`,` or `)`");
        } else {
            self.placement_entry(code);
        }
        self.finish();
    }

    fn placement_entry(&mut self, code: Code) {
        let mut opens = 0usize;
        loop {
            self.start(SyntaxKind::PlacementEntry);
            if !self.eat(SyntaxKind::LBracket) {
                break;
            }
            opens += 1;
        }
        match self.peek() {
            Some(SyntaxKind::Ident) if self.nth(1) == Some(SyntaxKind::Colon2) => {
                self.qualified_type_name(code);
            }
            Some(SyntaxKind::Ident | SyntaxKind::NodeKw | SyntaxKind::WireKw) => self.bump(),
            _ => {
                let found = self.describe();
                self.error(
                    code,
                    format!(
                        "expected a kind, a pattern, `node`, `wire` or a bundle, found {found}"
                    ),
                );
            }
        }
        self.count(code);
        self.finish();
        for _ in 0..opens {
            self.expect(SyntaxKind::RBracket, code, "`]`");
            self.count(code);
            self.finish();
        }
    }

    fn count(&mut self, code: Code) {
        if self.eat(SyntaxKind::Plus) || self.eat(SyntaxKind::Star) {
            return;
        }
        if !self.at(SyntaxKind::LBrace) {
            return;
        }
        self.start(SyntaxKind::Count);
        self.bump();
        if self.eat(SyntaxKind::Dot2) {
            self.digits(code, "a count in digits");
        } else {
            self.digits(code, "a count in digits");
            if !self.eat(SyntaxKind::Plus) && self.eat(SyntaxKind::Dot2) {
                self.digits(code, "a count in digits");
            }
        }
        self.expect(SyntaxKind::RBrace, code, "`}`");
        self.finish();
    }
}

const RECORD_CLOSE: Task = Task::Close {
    kind: SyntaxKind::RBrace,
    code: Code::RecordSyntax,
    what: "`,` or `}`",
};

const FIELDS: &str = "write each field on its own line under the `type` line, as in `root: i32`";

const REQUIRE: &str =
    "write the source as a string, as in `require \"github.com/acme/util@1.2.0\"`";
