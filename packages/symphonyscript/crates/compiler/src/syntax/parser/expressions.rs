use super::Parser;
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
            Some(SyntaxKind::IfKw) => self.if_expr(),
            Some(SyntaxKind::MatchKw) => self.misplaced_match(),
            _ => self.or_expr(),
        }
    }

    fn misplaced_match(&mut self) {
        self.error(
            Code::MatchPosition,
            "a match is the whole value of a let, a function body or a wire destination",
        );
        self.skip_token();
        self.or_expr();
    }

    fn if_expr(&mut self) {
        self.start(SyntaxKind::IfExpr);
        self.bump();
        self.expr();
        self.expect(SyntaxKind::ThenKw, Code::ExpressionSyntax, "`then`");
        self.expr();
        self.expect(SyntaxKind::ElseKw, Code::ExpressionSyntax, "`else`");
        self.expr();
        self.finish();
    }

    fn or_expr(&mut self) {
        let checkpoint = self.checkpoint();
        self.and_expr();
        while self.at(SyntaxKind::OrKw) {
            self.start_at(checkpoint, SyntaxKind::BinaryExpr);
            self.bump();
            self.and_expr();
            self.finish();
        }
    }

    fn and_expr(&mut self) {
        let checkpoint = self.checkpoint();
        self.not_expr();
        while self.at(SyntaxKind::AndKw) {
            self.start_at(checkpoint, SyntaxKind::BinaryExpr);
            self.bump();
            self.not_expr();
            self.finish();
        }
    }

    fn not_expr(&mut self) {
        if self.at(SyntaxKind::NotKw) {
            self.start(SyntaxKind::PrefixExpr);
            self.bump();
            self.not_expr();
            self.finish();
        } else {
            self.cmp_expr();
        }
    }

    fn cmp_expr(&mut self) {
        let checkpoint = self.checkpoint();
        self.add_expr();
        let mut chained = false;
        while self.at_any(COMPARISONS) {
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
            self.add_expr();
            self.finish();
            chained = true;
        }
    }

    fn add_expr(&mut self) {
        let checkpoint = self.checkpoint();
        self.mul_expr();
        while self.at_any(&[SyntaxKind::Plus, SyntaxKind::Minus]) {
            self.start_at(checkpoint, SyntaxKind::BinaryExpr);
            self.bump();
            self.mul_expr();
            self.finish();
        }
    }

    fn mul_expr(&mut self) {
        let checkpoint = self.checkpoint();
        self.unary_expr();
        while self.at_any(PRODUCTS) {
            self.start_at(checkpoint, SyntaxKind::BinaryExpr);
            self.bump();
            self.unary_expr();
            self.finish();
        }
    }

    fn unary_expr(&mut self) {
        if self.at(SyntaxKind::Minus) {
            self.start(SyntaxKind::PrefixExpr);
            self.bump();
            self.unary_expr();
            self.finish();
        } else {
            self.pow_expr();
        }
    }

    fn pow_expr(&mut self) {
        let checkpoint = self.checkpoint();
        self.postfix_expr();
        if self.at(SyntaxKind::Star2) {
            self.start_at(checkpoint, SyntaxKind::BinaryExpr);
            self.bump();
            self.unary_expr();
            self.finish();
        }
    }

    fn postfix_expr(&mut self) {
        let checkpoint = self.checkpoint();
        if !self.primary() {
            return;
        }
        loop {
            match self.peek() {
                Some(SyntaxKind::Dot) => {
                    self.start_at(checkpoint, SyntaxKind::FieldExpr);
                    self.bump();
                    self.expect_name(Code::ExpressionSyntax, "an attribute name after `.`");
                    self.finish();
                }
                Some(SyntaxKind::LBracket) => {
                    self.start_at(checkpoint, SyntaxKind::IndexExpr);
                    self.bump();
                    self.expr();
                    self.expect(SyntaxKind::RBracket, Code::ExpressionSyntax, "`]`");
                    self.finish();
                }
                Some(SyntaxKind::LParen) => {
                    self.start_at(checkpoint, SyntaxKind::CallExpr);
                    self.arg_list();
                    self.finish();
                }
                _ => break,
            }
        }
    }

    fn primary(&mut self) -> bool {
        match self.peek() {
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
                self.start(SyntaxKind::NameRef);
                self.bump();
                self.finish();
            }
            Some(SyntaxKind::ToKw) => {
                self.start(SyntaxKind::ToExpr);
                self.bump();
                self.finish();
            }
            Some(SyntaxKind::I32Kw | SyntaxKind::I64Kw | SyntaxKind::F32Kw | SyntaxKind::F64Kw) => {
                self.start(SyntaxKind::Conversion);
                self.bump();
                if self.expect(
                    SyntaxKind::LParen,
                    Code::ConversionSyntax,
                    "`(` after the conversion's type",
                ) {
                    self.expr();
                    self.expect(SyntaxKind::RParen, Code::ConversionSyntax, "`)`");
                }
                self.finish();
            }
            Some(SyntaxKind::LBracket) => self.array_lit(),
            Some(SyntaxKind::LParen) => {
                self.start(SyntaxKind::ParenExpr);
                self.bump();
                self.expr();
                self.expect(SyntaxKind::RParen, Code::ExpressionSyntax, "`)`");
                self.finish();
            }
            Some(SyntaxKind::IfKw) => {
                self.error(
                    Code::UnparenthesizedIf,
                    "an `if` that is an operand of another operator is parenthesized",
                );
                self.if_expr();
            }
            Some(SyntaxKind::MatchKw) => self.misplaced_match(),
            _ => {
                let found = self.describe();
                self.error(
                    Code::ExpressionSyntax,
                    format!("expected an expression, found {found}"),
                );
                return false;
            }
        }
        true
    }

    pub(super) fn variant_ref(&mut self) {
        self.start(SyntaxKind::VariantRef);
        self.bump();
        self.bump();
        self.expect_name(Code::ExpressionSyntax, "a variant name after `::`");
        self.finish();
    }

    fn array_lit(&mut self) {
        self.start(SyntaxKind::ArrayLit);
        self.bump();
        if self.at(SyntaxKind::RBracket) {
            self.error(
                Code::ExpressionSyntax,
                "an array holds at least one element",
            );
        } else {
            loop {
                self.expr();
                if !self.eat(SyntaxKind::Comma) || self.at(SyntaxKind::RBracket) {
                    break;
                }
            }
        }
        self.expect(SyntaxKind::RBracket, Code::ExpressionSyntax, "`,` or `]`");
        self.finish();
    }

    fn arg_list(&mut self) {
        self.start(SyntaxKind::ArgList);
        self.bump();
        if !self.at(SyntaxKind::RParen) {
            let mut named_call = None;
            loop {
                let named = self.at(SyntaxKind::Ident) && self.nth(1) == Some(SyntaxKind::Eq);
                if named_call.is_some_and(|first| first != named) {
                    self.error(
                        Code::MixedArguments,
                        "a call's arguments are all positional or all named",
                    );
                }
                named_call.get_or_insert(named);
                if named {
                    self.start(SyntaxKind::NamedArg);
                    self.bump();
                    self.bump();
                    self.expr();
                    self.finish();
                } else {
                    self.expr();
                }
                if !self.eat(SyntaxKind::Comma) {
                    break;
                }
            }
        }
        self.expect(SyntaxKind::RParen, Code::ExpressionSyntax, "`,` or `)`");
        self.finish();
    }

    pub(super) fn string_lit(&mut self) {
        self.start(SyntaxKind::StringLit);
        self.bump();
        loop {
            match self.peek() {
                Some(SyntaxKind::StringText) => self.bump(),
                Some(SyntaxKind::InterpStart) => {
                    self.start(SyntaxKind::Interpolation);
                    self.bump();
                    self.expr();
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
                }
                Some(SyntaxKind::StringEnd) => {
                    self.bump();
                    self.finish();
                    return;
                }
                _ => break,
            }
        }
        self.finish();
        self.abandon_bracket();
    }
}
