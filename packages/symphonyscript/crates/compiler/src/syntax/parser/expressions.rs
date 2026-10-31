use super::checkpoint::Checkpoint;

use super::Parser;
use super::task::Task;
use crate::diagnostics::Code;
use crate::syntax::syntax_kind::SyntaxKind;

const COMPARISONS: &[SyntaxKind] = &[
    SyntaxKind::IsKw,
    SyntaxKind::Lt,
    SyntaxKind::Le,
    SyntaxKind::Gt,
    SyntaxKind::Ge,
];

const PRODUCTS: &[SyntaxKind] = &[
    SyntaxKind::Star,
    SyntaxKind::Slash,
    SyntaxKind::DivKw,
    SyntaxKind::Percent,
];

impl Parser<'_> {
    pub(super) fn expr(&mut self) {
        match self.peek() {
            Some(SyntaxKind::IfKw) => self.then([Task::IfExpr]),
            Some(SyntaxKind::ForKw) => {
                self.unparenthesized_for();
                self.then([Task::ForExpr { own_line: true }]);
            }
            Some(SyntaxKind::MatchKw) => {
                self.misplaced_match();
                self.then([Task::OrExpr]);
            }
            _ => self.then([Task::OrExpr]),
        }
    }

    fn misplaced_match(&mut self) {
        self.error(
            Code::MatchPosition,
            "a match is the whole value of a let, a function body or a wire destination",
        );
        self.skip_token();
    }

    pub(super) fn if_expr(&mut self) {
        self.start(SyntaxKind::IfExpr);
        self.bump();
        self.then([Task::Expr, Task::IfThen]);
    }

    pub(super) fn if_then(&mut self) {
        self.expect(SyntaxKind::ThenKw, Code::ExpressionSyntax, "`then`");
        self.then([Task::Expr, Task::IfElse]);
    }

    pub(super) fn if_else(&mut self) {
        self.expect(SyntaxKind::ElseKw, Code::ExpressionSyntax, "`else`");
        self.then([Task::Expr, Task::Finish]);
    }

    pub(super) fn or_expr(&mut self) {
        let checkpoint = self.checkpoint();
        self.then([Task::AndExpr, Task::OrLoop(checkpoint)]);
    }

    pub(super) fn or_loop(&mut self, checkpoint: Checkpoint) {
        if self.at(SyntaxKind::OrKw) {
            self.start_at(checkpoint, SyntaxKind::BinaryExpr);
            self.bump();
            self.then([Task::AndExpr, Task::Finish, Task::OrLoop(checkpoint)]);
        }
    }

    pub(super) fn and_expr(&mut self) {
        let checkpoint = self.checkpoint();
        self.then([Task::NotExpr, Task::AndLoop(checkpoint)]);
    }

    pub(super) fn and_loop(&mut self, checkpoint: Checkpoint) {
        if self.at(SyntaxKind::AndKw) {
            self.start_at(checkpoint, SyntaxKind::BinaryExpr);
            self.bump();
            self.then([Task::NotExpr, Task::Finish, Task::AndLoop(checkpoint)]);
        }
    }

    pub(super) fn not_expr(&mut self) {
        if self.at(SyntaxKind::NotKw) {
            self.start(SyntaxKind::PrefixExpr);
            self.bump();
            self.then([Task::NotExpr, Task::Finish]);
        } else {
            self.then([Task::CmpExpr]);
        }
    }

    pub(super) fn cmp_expr(&mut self) {
        let checkpoint = self.checkpoint();
        self.then([
            Task::AddExpr,
            Task::CmpLoop {
                checkpoint,
                chained: false,
            },
        ]);
    }

    pub(super) fn cmp_loop(&mut self, checkpoint: Checkpoint, chained: bool) {
        if !self.at_any(COMPARISONS) {
            return;
        }
        if chained {
            self.error(
                Code::ChainedComparison,
                "comparisons do not chain; combine them with `and`",
            );
        }
        self.start_at(checkpoint, SyntaxKind::BinaryExpr);
        if self.eat(SyntaxKind::IsKw) {
            self.eat(SyntaxKind::NotKw);
        } else {
            self.bump();
        }
        self.then([
            Task::AddExpr,
            Task::Finish,
            Task::CmpLoop {
                checkpoint,
                chained: true,
            },
        ]);
    }

    pub(super) fn add_expr(&mut self) {
        let checkpoint = self.checkpoint();
        self.then([Task::MulExpr, Task::AddLoop(checkpoint)]);
    }

    pub(super) fn add_loop(&mut self, checkpoint: Checkpoint) {
        if self.at_any(&[SyntaxKind::Plus, SyntaxKind::Minus]) {
            self.start_at(checkpoint, SyntaxKind::BinaryExpr);
            self.bump();
            self.then([Task::MulExpr, Task::Finish, Task::AddLoop(checkpoint)]);
        }
    }

    pub(super) fn mul_expr(&mut self) {
        let checkpoint = self.checkpoint();
        self.then([Task::UnaryExpr, Task::MulLoop(checkpoint)]);
    }

    pub(super) fn mul_loop(&mut self, checkpoint: Checkpoint) {
        if self.at_any(PRODUCTS) {
            self.start_at(checkpoint, SyntaxKind::BinaryExpr);
            self.bump();
            self.then([Task::UnaryExpr, Task::Finish, Task::MulLoop(checkpoint)]);
        }
    }

    pub(super) fn unary_expr(&mut self) {
        if self.at(SyntaxKind::Minus) {
            self.start(SyntaxKind::PrefixExpr);
            self.bump();
            self.then([Task::UnaryExpr, Task::Finish]);
        } else {
            self.then([Task::PowExpr]);
        }
    }

    pub(super) fn pow_expr(&mut self) {
        let checkpoint = self.checkpoint();
        self.then([Task::Postfix, Task::PowTail(checkpoint)]);
    }

    pub(super) fn pow_tail(&mut self, checkpoint: Checkpoint) {
        if self.at(SyntaxKind::Star2) {
            self.start_at(checkpoint, SyntaxKind::BinaryExpr);
            self.bump();
            self.then([Task::UnaryExpr, Task::Finish]);
        }
    }

    pub(super) fn postfix(&mut self) {
        let checkpoint = self.checkpoint();
        self.then([Task::Primary(checkpoint)]);
    }

    pub(super) fn postfix_loop(&mut self, checkpoint: Checkpoint) {
        match self.peek() {
            Some(SyntaxKind::Dot) => {
                self.start_at(checkpoint, SyntaxKind::FieldExpr);
                self.bump();
                self.expect_member_name(Code::ExpressionSyntax, "an attribute name after `.`");
                self.finish();
                self.then([Task::PostfixLoop(checkpoint)]);
            }
            Some(SyntaxKind::LBracket) => {
                self.start_at(checkpoint, SyntaxKind::IndexExpr);
                self.bump();
                self.then([
                    Task::IndexBody,
                    Task::Close {
                        kind: SyntaxKind::RBracket,
                        code: Code::ExpressionSyntax,
                        what: "`]`",
                    },
                    Task::PostfixLoop(checkpoint),
                ]);
            }
            Some(SyntaxKind::LParen) => {
                self.start_at(checkpoint, SyntaxKind::CallExpr);
                self.then([Task::ArgList, Task::Finish, Task::PostfixLoop(checkpoint)]);
            }
            _ => {}
        }
    }

    pub(super) fn primary(&mut self, checkpoint: Checkpoint) {
        let rest = Task::PostfixLoop(checkpoint);
        match self.peek() {
            Some(SyntaxKind::Number | SyntaxKind::TrueKw | SyntaxKind::FalseKw) => {
                self.start(SyntaxKind::Literal);
                self.bump();
                self.finish();
                self.then([rest]);
            }
            Some(SyntaxKind::StringStart) => self.then([Task::StringLit, rest]),
            Some(SyntaxKind::Ident) if self.nth(1) == Some(SyntaxKind::Colon2) => {
                self.variant_ref();
                self.then([rest]);
            }
            Some(SyntaxKind::Ident) => {
                self.start(SyntaxKind::NameRef);
                self.bump();
                self.finish();
                self.then([rest]);
            }
            Some(SyntaxKind::ToKw) => {
                self.start(SyntaxKind::ToExpr);
                self.bump();
                self.finish();
                self.then([rest]);
            }
            Some(
                SyntaxKind::U8Kw
                | SyntaxKind::I32Kw
                | SyntaxKind::I64Kw
                | SyntaxKind::F32Kw
                | SyntaxKind::F64Kw,
            ) => {
                self.start(SyntaxKind::Conversion);
                self.bump();
                if self.expect(
                    SyntaxKind::LParen,
                    Code::ConversionSyntax,
                    "`(` after the conversion's type",
                ) {
                    self.then([
                        Task::Expr,
                        Task::Close {
                            kind: SyntaxKind::RParen,
                            code: Code::ConversionSyntax,
                            what: "`)`",
                        },
                        rest,
                    ]);
                } else {
                    self.finish();
                    self.then([rest]);
                }
            }
            Some(SyntaxKind::JsonKw) => self.then([Task::JsonLit, rest]),
            Some(SyntaxKind::LBracket) => self.then([Task::ArrayLit, rest]),
            Some(SyntaxKind::LParen) => {
                self.start(SyntaxKind::ParenExpr);
                self.bump();
                self.then([
                    Task::ParenValue,
                    Task::Close {
                        kind: SyntaxKind::RParen,
                        code: Code::ExpressionSyntax,
                        what: "`)`",
                    },
                    rest,
                ]);
            }
            Some(SyntaxKind::IfKw) => {
                self.error(
                    Code::UnparenthesizedIf,
                    "an `if` that is an operand of another operator is parenthesized",
                );
                self.then([Task::IfExpr, rest]);
            }
            Some(SyntaxKind::ForKw) => {
                self.unparenthesized_for();
                self.then([Task::ForExpr { own_line: true }, rest]);
            }
            Some(SyntaxKind::MatchKw) => {
                self.misplaced_match();
                self.then([Task::OrExpr, rest]);
            }
            _ => {
                let found = self.describe();
                self.error(
                    Code::ExpressionSyntax,
                    format!("expected an expression, found {found}"),
                );
            }
        }
    }

    pub(super) fn qualified_name(&mut self, code: Code) {
        self.start(SyntaxKind::VariantRef);
        self.bump();
        self.bump();
        self.expect_name(code, "a name after `::`");
        self.finish();
    }

    pub(super) fn variant_ref(&mut self) {
        self.start(SyntaxKind::VariantRef);
        self.bump();
        self.bump();
        self.expect_name(Code::ExpressionSyntax, "a variant name after `::`");
        self.finish();
    }

    pub(super) fn array_lit(&mut self) {
        self.start(SyntaxKind::ArrayLit);
        self.bump();
        if self.at(SyntaxKind::RBracket) {
            self.then([ARRAY_CLOSE]);
        } else {
            self.then([Task::Expr, Task::ArrayLoop]);
        }
    }

    pub(super) fn index_body(&mut self) {
        if self.eat(SyntaxKind::Dot2) {
            self.then([Task::SliceUpper]);
        } else {
            self.then([Task::Bound, Task::IndexAfterBound]);
        }
    }

    pub(super) fn index_after_bound(&mut self) {
        if self.eat(SyntaxKind::Dot2) {
            self.then([Task::SliceUpper]);
        }
    }

    pub(super) fn slice_upper(&mut self) {
        if !self.at(SyntaxKind::RBracket) {
            self.then([Task::Bound]);
        }
    }

    pub(super) fn bound(&mut self) {
        let absent = [
            SyntaxKind::RBracket,
            SyntaxKind::Newline,
            SyntaxKind::Indent,
            SyntaxKind::Dedent,
        ];
        if self.at_end() || absent.into_iter().any(|kind| self.at(kind)) {
            self.then([Task::Expr]);
            return;
        }
        self.start(SyntaxKind::Bound);
        self.eat(SyntaxKind::Caret);
        self.then([Task::Expr, Task::Finish]);
    }

    pub(super) fn array_loop(&mut self) {
        if self.eat(SyntaxKind::Comma) && !self.at(SyntaxKind::RBracket) {
            self.then([Task::Expr, Task::ArrayLoop]);
        } else {
            self.then([ARRAY_CLOSE]);
        }
    }

    pub(super) fn arg_list(&mut self) {
        self.start(SyntaxKind::ArgList);
        self.bump();
        if self.at(SyntaxKind::RParen) {
            self.then([ARGS_CLOSE]);
        } else {
            self.then([Task::ArgItem { named_call: None }]);
        }
    }

    pub(super) fn arg_item(&mut self, named_call: Option<bool>) {
        let named = self.at(SyntaxKind::Ident) && self.nth(1) == Some(SyntaxKind::Eq);
        if named_call.is_some_and(|first| first != named) {
            self.error(
                Code::MixedArguments,
                "a call's arguments are all positional or all named",
            );
        }
        let named_call = named_call.or(Some(named));
        if named {
            self.start(SyntaxKind::NamedArg);
            self.bump();
            self.bump();
            self.then([Task::Expr, Task::Finish, Task::ArgAfter { named_call }]);
        } else {
            self.then([Task::Expr, Task::ArgAfter { named_call }]);
        }
    }

    pub(super) fn arg_after(&mut self, named_call: Option<bool>) {
        if self.eat(SyntaxKind::Comma) {
            self.then([Task::ArgItem { named_call }]);
        } else {
            self.then([ARGS_CLOSE]);
        }
    }

    pub(super) fn string_lit(&mut self) {
        self.start(SyntaxKind::StringLit);
        self.bump();
        self.then([Task::StringLoop]);
    }

    pub(super) fn string_loop(&mut self) {
        match self.peek() {
            Some(SyntaxKind::StringText) => {
                self.bump();
                self.then([Task::StringLoop]);
            }
            Some(SyntaxKind::InterpStart) => {
                self.start(SyntaxKind::Interpolation);
                self.bump();
                self.then([Task::Expr, Task::InterpEnd]);
            }
            Some(SyntaxKind::StringEnd) => {
                self.bump();
                self.finish();
            }
            _ => {
                self.finish();
                self.abandon_bracket();
            }
        }
    }

    pub(super) fn interp_end(&mut self) {
        if !self.at(SyntaxKind::InterpEnd) {
            let found = self.describe();
            self.error(
                Code::ExpressionSyntax,
                format!("expected `}}` after the interpolated value, found {found}"),
            );
            self.start(SyntaxKind::ErrorNode);
            while !self.at_end()
                && !self.at_any(&[
                    SyntaxKind::InterpEnd,
                    SyntaxKind::StringEnd,
                    SyntaxKind::Newline,
                    SyntaxKind::Dedent,
                ])
            {
                self.bump();
            }
            self.finish();
        }
        self.eat(SyntaxKind::InterpEnd);
        self.finish();
        self.then([Task::StringLoop]);
    }
}

const ARRAY_CLOSE: Task = Task::Close {
    kind: SyntaxKind::RBracket,
    code: Code::ExpressionSyntax,
    what: "`,` or `]`",
};

const ARGS_CLOSE: Task = Task::Close {
    kind: SyntaxKind::RParen,
    code: Code::ExpressionSyntax,
    what: "`,` or `)`",
};
