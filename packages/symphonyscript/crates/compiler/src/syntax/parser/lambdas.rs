use super::Parser;
use super::task::Task;
use crate::diagnostics::Code;
use crate::syntax::syntax_kind::SyntaxKind;

impl Parser<'_> {
    pub(super) fn at_lambda(&mut self) -> bool {
        match self.peek() {
            Some(SyntaxKind::Ident) => self.nth(1) == Some(SyntaxKind::Colon),
            Some(SyntaxKind::LParen) => matches!(
                self.after_group(),
                Some(SyntaxKind::Colon | SyntaxKind::Arrow)
            ),
            Some(_) | None => false,
        }
    }

    pub(super) fn arg_value(&mut self) -> Task {
        if self.at_lambda() {
            Task::Lambda
        } else {
            Task::Expr
        }
    }

    pub(super) fn in_value_bracket(&self) -> bool {
        matches!(
            self.builder.open_kind(),
            Some(
                SyntaxKind::ParenExpr
                    | SyntaxKind::Init
                    | SyntaxKind::ArrayLit
                    | SyntaxKind::Conversion
                    | SyntaxKind::IndexExpr
                    | SyntaxKind::Bound
                    | SyntaxKind::Interpolation
            )
        )
    }

    pub(super) fn misplaced_lambda(&mut self) {
        let reported = self.diagnostics.len();
        self.error(
            Code::ExpressionSyntax,
            "a lambda stands only as an argument or as a `let`'s value",
        );
        self.help_since(reported, Code::ExpressionSyntax, LAMBDA_PLACE);
    }

    pub(super) fn lambda(&mut self) {
        self.start(SyntaxKind::Lambda);
        if self.at(SyntaxKind::Ident) {
            self.bump();
            self.bump();
            self.then([Task::Expr, Task::Finish]);
            return;
        }
        self.bump();
        if self.at(SyntaxKind::RParen) {
            self.then([Task::LambdaClose]);
        } else {
            self.start(SyntaxKind::LambdaParams);
            self.then([Task::LambdaParam, Task::LambdaParamLoop, Task::LambdaClose]);
        }
    }

    pub(super) fn lambda_param(&mut self) {
        self.start(SyntaxKind::LambdaParam);
        let named = self.expect_name(Code::LambdaSyntax, "a parameter name");
        if named && self.eat(SyntaxKind::Colon) {
            self.then([Task::Type, Task::Finish]);
        } else {
            self.finish();
        }
    }

    pub(super) fn lambda_param_loop(&mut self) {
        if self.eat(SyntaxKind::Comma) {
            self.then([Task::LambdaParam, Task::LambdaParamLoop]);
        } else {
            self.finish();
        }
    }

    pub(super) fn lambda_close(&mut self) {
        self.expect(SyntaxKind::RParen, Code::LambdaSyntax, "`,` or `)`");
        if self.at(SyntaxKind::Arrow) {
            self.start(SyntaxKind::FnResult);
            self.bump();
            self.then([Task::Type, Task::Finish, Task::LambdaBody]);
        } else {
            self.lambda_body();
        }
    }

    pub(super) fn lambda_body(&mut self) {
        if self.expect(
            SyntaxKind::Colon,
            Code::LambdaSyntax,
            "`:` and the lambda's body",
        ) {
            self.then([Task::Expr, Task::Finish]);
        } else {
            self.finish();
        }
    }

    pub(super) fn map_lit(&mut self) {
        self.start(SyntaxKind::MapLit);
        self.bump();
        if self.at(SyntaxKind::RBrace) {
            self.then([MAP_CLOSE]);
        } else {
            self.then([Task::MapEntry, Task::MapEntryLoop]);
        }
    }

    pub(super) fn map_entry(&mut self) {
        self.start(SyntaxKind::MapEntry);
        if self.at_any(super::types::EXPRESSION_START) {
            self.then([Task::Expr, Task::MapEntryValue]);
        } else {
            let found = self.describe();
            self.error(
                Code::MapSyntax,
                format!("expected an entry's key, found {found}"),
            );
            self.finish();
        }
    }

    pub(super) fn map_entry_value(&mut self) {
        if self.expect(
            SyntaxKind::Colon,
            Code::MapSyntax,
            "`:` and the entry's value",
        ) {
            self.then([Task::Expr, Task::Finish]);
        } else {
            self.finish();
        }
    }

    pub(super) fn map_entry_loop(&mut self) {
        if self.eat(SyntaxKind::Comma) && !self.at(SyntaxKind::RBrace) {
            self.then([Task::MapEntry, Task::MapEntryLoop]);
        } else {
            self.then([MAP_CLOSE]);
        }
    }
}

const MAP_CLOSE: Task = Task::Close {
    kind: SyntaxKind::RBrace,
    code: Code::MapSyntax,
    what: "`,` or `}`",
};

const LAMBDA_PLACE: &str = "pass the lambda as an argument, as in `count(held, p: p > 60)`, or name it with a `let`, as in `let keep = p: p > 60`";
