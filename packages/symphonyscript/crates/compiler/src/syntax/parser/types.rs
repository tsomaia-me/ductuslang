use super::Parser;
use super::checkpoint::Checkpoint;
use super::task::Task;
use crate::diagnostics::Code;
use crate::syntax::syntax_kind::SyntaxKind;

const SCALAR_TYPES: &[SyntaxKind] = &[
    SyntaxKind::U8Kw,
    SyntaxKind::I32Kw,
    SyntaxKind::I64Kw,
    SyntaxKind::F32Kw,
    SyntaxKind::F64Kw,
];

pub(super) const EXPRESSION_START: &[SyntaxKind] = &[
    SyntaxKind::Number,
    SyntaxKind::StringStart,
    SyntaxKind::TrueKw,
    SyntaxKind::FalseKw,
    SyntaxKind::Ident,
    SyntaxKind::ToKw,
    SyntaxKind::U8Kw,
    SyntaxKind::I32Kw,
    SyntaxKind::I64Kw,
    SyntaxKind::F32Kw,
    SyntaxKind::F64Kw,
    SyntaxKind::LBracket,
    SyntaxKind::LParen,
    SyntaxKind::LBrace,
    SyntaxKind::Minus,
    SyntaxKind::NotKw,
    SyntaxKind::IfKw,
    SyntaxKind::ForKw,
    SyntaxKind::JsonKw,
    SyntaxKind::NoneKw,
];

impl Parser<'_> {
    pub(super) fn type_start(&mut self) {
        if self.at(SyntaxKind::LParen) && self.after_group() == Some(SyntaxKind::Arrow) {
            self.start(SyntaxKind::FnType);
            self.bump();
            if self.at(SyntaxKind::RParen) {
                self.then([TYPE_PAREN_EXPECT, Task::FnTypeResult]);
            } else {
                self.then([Task::FnParam, Task::FnParamLoop]);
            }
            return;
        }
        let checkpoint = self.checkpoint();
        self.then([
            Task::Base,
            Task::SumLoop {
                checkpoint,
                open: false,
            },
        ]);
    }

    pub(super) fn fn_param(&mut self) {
        self.start(SyntaxKind::FnParam);
        if self.at(SyntaxKind::Ident) && self.nth(1) == Some(SyntaxKind::Colon) {
            self.bump();
            self.bump();
        }
        self.then([Task::Type, Task::Finish]);
    }

    pub(super) fn fn_param_loop(&mut self) {
        if self.eat(SyntaxKind::Comma) {
            self.then([Task::FnParam, Task::FnParamLoop]);
        } else {
            self.then([TYPE_PAREN_EXPECT, Task::FnTypeResult]);
        }
    }

    pub(super) fn fn_type_result(&mut self) {
        if self.expect(
            SyntaxKind::Arrow,
            Code::TypeSyntax,
            "`->` and the result type",
        ) {
            self.then([Task::Type, Task::Finish]);
        } else {
            self.finish();
        }
    }

    pub(super) fn sum_loop(&mut self, checkpoint: Checkpoint, open: bool) {
        if self.at(SyntaxKind::Pipe) {
            if !open {
                self.start_at(checkpoint, SyntaxKind::SumType);
            }
            self.bump();
            self.then([
                Task::Base,
                Task::SumLoop {
                    checkpoint,
                    open: true,
                },
            ]);
        } else if open {
            self.finish();
        }
    }

    pub(super) fn base(&mut self) {
        self.start(SyntaxKind::TypeRef);
        match self.peek() {
            Some(kind) if SCALAR_TYPES.contains(&kind) => {
                self.bump();
                self.then([Task::BaseTail]);
            }
            Some(SyntaxKind::BoolKw | SyntaxKind::NoneKw) => {
                self.bump();
                self.then([Task::BaseTail]);
            }
            Some(kind @ (SyntaxKind::StrKw | SyntaxKind::JsonKw)) => {
                let node = if kind == SyntaxKind::StrKw {
                    SyntaxKind::StrType
                } else {
                    SyntaxKind::JsonType
                };
                self.start(node);
                self.bump();
                if self.at(SyntaxKind::LBracket) && self.nth(1) == Some(SyntaxKind::Dot2) {
                    self.bump();
                    self.bump();
                    self.then([
                        Task::Size,
                        TYPE_BRACKET_EXPECT,
                        Task::Finish,
                        Task::BaseTail,
                    ]);
                } else {
                    self.finish();
                    self.then([Task::BaseTail]);
                }
            }
            Some(SyntaxKind::LBrace) => {
                self.start(SyntaxKind::MapType);
                self.bump();
                self.then([
                    Task::Type,
                    Task::MapTypeColon,
                    Task::Type,
                    Task::Expect {
                        kind: SyntaxKind::RBrace,
                        code: Code::TypeSyntax,
                        what: "`}`",
                    },
                    Task::MapTypeCapacity,
                    Task::BaseTail,
                ]);
            }
            Some(SyntaxKind::Ident) => {
                if self.nth(1) == Some(SyntaxKind::Colon2) {
                    self.qualified_type_name(Code::TypeSyntax);
                } else {
                    self.bump();
                }
                self.then([Task::TypeArgsOptional, Task::BaseTail]);
            }
            Some(SyntaxKind::LParen) => {
                self.start(SyntaxKind::ParenType);
                self.bump();
                self.then([Task::Type, TYPE_PAREN_CLOSE, Task::BaseTail]);
            }
            _ => {
                let found = self.describe();
                self.error(Code::TypeSyntax, format!("expected a type, found {found}"));
                self.finish();
            }
        }
    }

    pub(super) fn qualified_type_name(&mut self, code: Code) {
        self.start(SyntaxKind::QualifiedName);
        self.bump();
        self.bump();
        self.expect_name(code, "a name after `::`");
        self.finish();
    }

    pub(super) fn map_type_colon(&mut self) {
        self.expect(
            SyntaxKind::Colon,
            Code::TypeSyntax,
            "`:` between the key type and the value type",
        );
    }

    pub(super) fn map_type_capacity(&mut self) {
        if self.at(SyntaxKind::LBracket) && self.nth(1) == Some(SyntaxKind::Dot2) {
            self.bump();
            self.bump();
            self.then([Task::Size, TYPE_BRACKET_EXPECT, Task::Finish]);
        } else {
            let found = self.describe();
            let reported = self.diagnostics.len();
            self.error(
                Code::TypeSyntax,
                format!("expected the map's capacity `[..N]`, found {found}"),
            );
            self.help_since(reported, Code::TypeSyntax, MAP_CAPACITY);
            self.finish();
        }
    }

    pub(super) fn base_tail(&mut self) {
        if self.at(SyntaxKind::LBracket) {
            self.bump();
            self.eat(SyntaxKind::Dot2);
            self.then([Task::Size, TYPE_BRACKET_EXPECT, Task::BaseTail]);
            return;
        }
        if self.at(SyntaxKind::Question2) {
            self.error(
                Code::TypeSyntax,
                "a type takes one `?`, and `T?` already holds `none`",
            );
        } else {
            self.eat(SyntaxKind::Question);
        }
        self.finish();
    }

    pub(super) fn size(&mut self) {
        let plain = self.at_any(&[SyntaxKind::Number, SyntaxKind::JsonNumber])
            && self
                .current_text()
                .bytes()
                .all(|byte| byte.is_ascii_digit() || byte == b'_')
            && self.nth(1) == Some(SyntaxKind::RBracket);
        if plain {
            self.bump();
            return;
        }
        if !self.at_any(EXPRESSION_START) {
            let found = self.describe();
            self.error(Code::TypeSyntax, format!("expected a size, found {found}"));
            return;
        }
        self.start(SyntaxKind::Size);
        self.then([Task::Expr, Task::Finish]);
    }

    pub(super) fn type_args_optional(&mut self) {
        if !self.at(SyntaxKind::LBracket) || self.nth(1) == Some(SyntaxKind::Dot2) {
            return;
        }
        let first = self.nth(1);
        let typed = match first {
            Some(kind) if SCALAR_TYPES.contains(&kind) => self.nth(2) != Some(SyntaxKind::LParen),
            Some(
                SyntaxKind::BoolKw
                | SyntaxKind::StrKw
                | SyntaxKind::JsonKw
                | SyntaxKind::NoneKw
                | SyntaxKind::LBrace,
            ) => true,
            Some(_) | None => false,
        };
        if typed || self.group_has_comma() {
            self.then([Task::TypeArgs]);
        }
    }

    pub(super) fn type_args(&mut self) {
        self.start(SyntaxKind::TypeArgs);
        self.bump();
        self.then([Task::TypeArg, Task::TypeArgLoop]);
    }

    pub(super) fn type_arg(&mut self) {
        let typed = match self.peek() {
            Some(kind) if SCALAR_TYPES.contains(&kind) => self.nth(1) != Some(SyntaxKind::LParen),
            Some(
                SyntaxKind::BoolKw
                | SyntaxKind::StrKw
                | SyntaxKind::NoneKw
                | SyntaxKind::LBrace
                | SyntaxKind::LParen,
            ) => true,
            Some(SyntaxKind::JsonKw) => match self.nth(1) {
                Some(
                    SyntaxKind::LBrace
                    | SyntaxKind::JsonString
                    | SyntaxKind::JsonNumber
                    | SyntaxKind::JsonNull
                    | SyntaxKind::Minus
                    | SyntaxKind::TrueKw
                    | SyntaxKind::FalseKw,
                ) => false,
                Some(SyntaxKind::LBracket) => self.nth(2) == Some(SyntaxKind::Dot2),
                Some(_) | None => true,
            },
            Some(SyntaxKind::Ident) => matches!(
                self.nth(1),
                Some(
                    SyntaxKind::Comma
                        | SyntaxKind::RBracket
                        | SyntaxKind::LBracket
                        | SyntaxKind::Question
                        | SyntaxKind::Pipe
                        | SyntaxKind::Colon2
                )
            ),
            Some(_) | None => false,
        };
        if typed {
            self.then([Task::Type]);
        } else if self.at_any(EXPRESSION_START) {
            self.then([Task::Expr]);
        } else {
            let found = self.describe();
            self.error(
                Code::GenericSyntax,
                format!("expected a type or a compile-time value, found {found}"),
            );
        }
    }

    pub(super) fn type_arg_loop(&mut self) {
        if self.eat(SyntaxKind::Comma) {
            self.then([Task::TypeArg, Task::TypeArgLoop]);
        } else {
            self.then([Task::Close {
                kind: SyntaxKind::RBracket,
                code: Code::GenericSyntax,
                what: "`,` or `]`",
            }]);
        }
    }
}

const TYPE_PAREN_EXPECT: Task = Task::Expect {
    kind: SyntaxKind::RParen,
    code: Code::TypeSyntax,
    what: "`)`",
};

const TYPE_PAREN_CLOSE: Task = Task::Close {
    kind: SyntaxKind::RParen,
    code: Code::TypeSyntax,
    what: "`)`",
};

const TYPE_BRACKET_EXPECT: Task = Task::Expect {
    kind: SyntaxKind::RBracket,
    code: Code::TypeSyntax,
    what: "`]`",
};

const MAP_CAPACITY: &str =
    "write the map type's capacity after its braces, as in `{str[..8]: i32}[..16]`";
