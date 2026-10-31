use super::Parser;
use super::checkpoint::Checkpoint;
use super::task::Task;
use crate::diagnostics::{Code, Diagnostic};
use crate::syntax::syntax_kind::SyntaxKind;

impl Parser<'_> {
    pub(super) fn loop_head(&mut self, code: Code) {
        self.bump();
        self.loop_name(code);
        if self.eat(SyntaxKind::Comma) {
            self.loop_name(code);
        }
        self.expect(SyntaxKind::InKw, code, "`in`");
    }

    fn loop_name(&mut self, code: Code) {
        self.expect_name(code, "the loop variable");
        if self.eat(SyntaxKind::Colon) {
            self.type_ref();
        }
    }

    pub(super) fn head_value(&mut self, what: &str, help: Option<&str>) {
        let missing = self.at_end()
            || self.at_any(&[
                SyntaxKind::Colon,
                SyntaxKind::IntoKw,
                SyntaxKind::FromKw,
                SyntaxKind::Newline,
                SyntaxKind::Indent,
                SyntaxKind::Dedent,
            ]);
        if !missing {
            return;
        }
        let reported = self.diagnostics.len();
        let found = self.describe();
        self.error(Code::ForSyntax, format!("expected {what}, found {found}"));
        if let Some(help) = help {
            self.help_since(reported, Code::ForSyntax, help);
        }
    }

    fn for_expr_expect(&mut self, kind: SyntaxKind, what: &str) -> bool {
        let reported = self.diagnostics.len();
        let found = self.expect(kind, Code::ForSyntax, what);
        self.help_since(reported, Code::ForSyntax, FOR_EXPR_HEAD);
        found
    }

    pub(super) fn whole_expr(&mut self, own_line: bool) {
        if self.at(SyntaxKind::ForKw) {
            self.then([Task::ForExpr { own_line }]);
        } else {
            self.then([Task::Expr]);
        }
    }

    pub(super) fn paren_value(&mut self) {
        if self.at(SyntaxKind::ForKw) {
            self.then([Task::ForExpr { own_line: true }]);
        } else {
            self.then([Task::Expr]);
        }
    }

    pub(super) fn unparenthesized_for(&mut self) {
        self.error(
            Code::UnparenthesizedFor,
            "a `for` expression that is not the whole value of a `let`, a local `let`, a derived cell's rule or a function body stands in parentheses",
        );
    }

    pub(super) fn for_expr(&mut self, own_line: bool) {
        let checkpoint = self.checkpoint();
        self.start(SyntaxKind::ForExprHead);
        let reported = self.diagnostics.len();
        self.loop_head(Code::ForSyntax);
        self.help_since(reported, Code::ForSyntax, FOR_EXPR_HEAD);
        self.head_value("the iterable", Some(FOR_EXPR_HEAD));
        self.then([
            Task::Iterable,
            Task::ForExprInto {
                checkpoint,
                own_line,
            },
        ]);
    }

    pub(super) fn for_expr_into(&mut self, checkpoint: Checkpoint, own_line: bool) {
        self.for_expr_expect(SyntaxKind::IntoKw, "`into`");
        self.for_expr_expect(SyntaxKind::Ident, "the accumulator's name");
        if self.eat(SyntaxKind::Colon) {
            self.type_ref();
        }
        self.for_expr_expect(SyntaxKind::FromKw, "`from` and the start");
        self.head_value("the start", Some(FOR_EXPR_HEAD));
        self.then([
            Task::Expr,
            Task::ForExprColon {
                checkpoint,
                own_line,
            },
        ]);
    }

    pub(super) fn for_expr_colon(&mut self, checkpoint: Checkpoint, own_line: bool) {
        self.for_expr_expect(SyntaxKind::Colon, "`:` and the body");
        self.finish();
        if !self.at(SyntaxKind::Newline) {
            self.start_at(checkpoint, SyntaxKind::ForExpr);
            self.then([Task::Expr, Task::Finish]);
            return;
        }
        if !own_line {
            let diagnostic = Diagnostic::new(
                Code::FnSyntax,
                self.current_span(),
                "a function body that is a `for` expression's block form starts on its own line",
            )
            .with_help(FOR_BLOCK_BODY);
            self.error_with(diagnostic);
        }
        self.start_at(checkpoint, SyntaxKind::ForExprBlock);
        self.bump();
        if self.eat(SyntaxKind::Indent) {
            self.then([Task::ForExprLets]);
        } else {
            self.missing_block(
                Code::ExpressionSyntax,
                "expected the `for` expression's body, indented",
            );
            self.finish();
            self.block_ended = true;
        }
    }

    pub(super) fn for_expr_lets(&mut self) {
        if self.at(SyntaxKind::LetKw) {
            self.then([Task::LetDecl(SyntaxKind::LocalLet), Task::ForExprLets]);
        } else if self.at(SyntaxKind::StateKw) {
            self.error(Code::StateSyntax, "a state is declared at top level");
            self.then([Task::LetDecl(SyntaxKind::StateDecl), Task::ForExprLets]);
        } else {
            self.then([
                Task::Expr,
                Task::FinishLine(Code::ExpressionSyntax),
                Task::ForExprBlockEnd,
            ]);
        }
    }

    pub(super) fn for_expr_block_end(&mut self) {
        self.rest_of_block(
            Code::ExpressionSyntax,
            "a `for` expression's block ends with one value",
        );
        self.finish();
        self.block_ended = true;
    }
}

const FOR_EXPR_HEAD: &str = "write `for <name> in <iterable> into <name> from <start>:`";

const FOR_BLOCK_BODY: &str =
    "write the `for` expression on the line after the function's `:`, indented under it";
