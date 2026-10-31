use super::Parser;
use super::task::{CellHead, Task};
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
            Some(SyntaxKind::LetKw) => self.then([Task::LetDecl(SyntaxKind::LetDecl)]),
            Some(SyntaxKind::StateKw | SyntaxKind::StreamKw | SyntaxKind::ChannelKw) => {
                self.then([Task::CellDecl]);
            }
            Some(SyntaxKind::FnKw) => self.then([Task::FnDecl]),
            Some(SyntaxKind::PatternKw) => self.then([Task::PatternDecl]),
            Some(SyntaxKind::TypeKw) => self.then([Task::TypeDecl]),
            Some(SyntaxKind::RequireKw) => self.require(),
            Some(SyntaxKind::UseKw) => self.use_item(),
            Some(SyntaxKind::PublishKw) => self.publish(),
            Some(SyntaxKind::Ident) if self.at_record_literal() => {
                self.start(SyntaxKind::PackageDecl);
                self.then([Task::RecordLit, Task::FinishLineFinish(Code::PackageSyntax)]);
            }
            Some(SyntaxKind::Ident)
                if self.current_text() == "input" && self.nth(1) == Some(SyntaxKind::Ident) =>
            {
                let reported = self.diagnostics.len();
                self.error(
                    Code::TopLevelSyntax,
                    "expected a declaration or a placement, found `input`",
                );
                self.help_since(reported, Code::TopLevelSyntax, INPUT_FORM);
                self.skip_line();
                self.eat(SyntaxKind::Newline);
            }
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

    pub(super) fn let_decl(&mut self, kind: SyntaxKind) {
        let code = Code::LetSyntax;
        self.start(kind);
        self.bump();
        self.expect_name(code, "the let's name");
        if self.eat(SyntaxKind::Colon) {
            self.then([Task::Type, Task::LetValue]);
        } else {
            self.let_value();
        }
    }

    pub(super) fn let_value(&mut self) {
        let code = Code::LetSyntax;
        if !self.eat(SyntaxKind::Eq) {
            self.match_after_fault(SyntaxKind::Eq, code, "`=` and the value");
        }
        self.rule(code);
    }

    fn rule(&mut self, code: Code) {
        if self.at(SyntaxKind::MatchKw) {
            self.then([Task::MatchBlock, Task::Finish]);
        } else {
            self.then([
                Task::WholeExpr { own_line: true },
                Task::FinishLineFinish(code),
            ]);
        }
    }

    pub(super) fn cell_decl(&mut self) {
        let code = Code::CellSyntax;
        let word = match self.peek() {
            Some(SyntaxKind::StreamKw) => "stream",
            Some(SyntaxKind::ChannelKw) => "channel",
            Some(_) | None => "state",
        };
        self.start(SyntaxKind::CellDecl);
        self.bump();
        if self.eat(SyntaxKind::LBracket) {
            let reported = self.diagnostics.len();
            if !self.digits(code, "a capacity in digits") {
                self.help_since(reported, code, CELL_CAPACITY);
            }
            self.expect(SyntaxKind::RBracket, code, "`]`");
        }
        self.expect_name(code, "the cell's name");
        let head = CellHead {
            word,
            init: self.at(SyntaxKind::LParen),
            clock: false,
        };
        if head.init {
            self.start(SyntaxKind::Init);
            self.bump();
            self.then([
                Task::Expr,
                Task::Close {
                    kind: SyntaxKind::RParen,
                    code,
                    what: "`)`",
                },
                Task::CellAfterInit(head),
            ]);
        } else {
            self.then([Task::CellAfterInit(head)]);
        }
    }

    pub(super) fn cell_after_init(&mut self, head: CellHead) {
        if self.eat(SyntaxKind::Colon) {
            self.then([Task::Type, Task::CellAfterType(head)]);
        } else {
            self.cell_after_type(head);
        }
    }

    pub(super) fn cell_after_type(&mut self, head: CellHead) {
        if !self.at(SyntaxKind::OnKw) {
            self.then([Task::CellAfterClock(head)]);
            return;
        }
        let head = CellHead {
            clock: true,
            ..head
        };
        self.start(SyntaxKind::Clock);
        self.bump();
        if self.eat(SyntaxKind::LParen) {
            self.then([Task::Expr, Task::ClockList, Task::CellAfterClock(head)]);
        } else {
            self.then([Task::Atom, Task::Finish, Task::CellAfterClock(head)]);
        }
    }

    pub(super) fn clock_list(&mut self) {
        if self.eat(SyntaxKind::Comma) {
            self.then([Task::Expr, Task::ClockList]);
        } else {
            self.then([Task::Close {
                kind: SyntaxKind::RParen,
                code: Code::CellSyntax,
                what: "`,` or `)`",
            }]);
        }
    }

    pub(super) fn cell_after_clock(&mut self, head: CellHead) {
        let code = Code::CellSyntax;
        if self.eat(SyntaxKind::Eq) {
            self.rule(code);
            return;
        }
        let ends = self.at_end() || self.at_any(&[SyntaxKind::Newline, SyntaxKind::Dedent]);
        if !ends {
            let what = if head.init {
                "`=` and the rule, or the end of the line"
            } else {
                "`(` and the init, or `=` and the rule"
            };
            if self.match_after_fault(SyntaxKind::Eq, code, what) {
                self.rule(code);
            } else {
                self.then([Task::FinishLineFinish(code)]);
            }
            return;
        }
        let word = head.word;
        let reported = self.diagnostics.len();
        if head.clock {
            self.error(
                code,
                "a clock stands on a derived cell, before `=` and its rule",
            );
            let help = format!(
                "write the rule after the clock, as in `{word} held(0) on gate = level`, or remove the clock"
            );
            self.help_since(reported, code, &help);
        } else if !head.init {
            self.error(
                code,
                "a cell has an init in parentheses, a rule after `=`, or both",
            );
            let help = format!(
                "write `{word} <name>(<init>)` for a cell the host writes, or `{word} <name> = <rule>` for a cell the program computes"
            );
            self.help_since(reported, code, &help);
        }
        self.then([Task::FinishLineFinish(code)]);
    }

    pub(super) fn at_cell_word(&mut self) -> bool {
        self.at_any(&[
            SyntaxKind::StateKw,
            SyntaxKind::StreamKw,
            SyntaxKind::ChannelKw,
        ])
    }

    pub(super) fn misplaced_cell(&mut self) {
        let word = self.current_text();
        self.error(
            Code::CellSyntax,
            format!("a `{word}` is declared at top level"),
        );
    }

    pub(super) fn digits(&mut self, code: Code, what: &str) -> bool {
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
        self.generic_params_then(Task::FnParams);
    }

    pub(super) fn fn_params(&mut self) {
        self.param_list(SyntaxKind::ParamList, Code::FnSyntax, false, Task::FnResult);
    }

    pub(super) fn pattern_decl(&mut self) {
        self.start(SyntaxKind::PatternDecl);
        self.bump();
        self.expect_name(Code::PatternSyntax, "the pattern's name");
        self.generic_params_then(Task::PatternParams);
    }

    pub(super) fn pattern_params(&mut self) {
        self.param_list(
            SyntaxKind::PatternParamList,
            Code::PatternSyntax,
            true,
            Task::PatternType,
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
        if pattern && self.eat(SyntaxKind::ChildrenKw) {
            self.expect_name(code, "the `children` parameter's name");
            if self.expect(
                SyntaxKind::Colon,
                code,
                "`:` and the placement type of the `children` parameter",
            ) {
                self.placement_type(code);
            }
            self.then([Task::ParamAfter { pattern }]);
            return;
        }
        if pattern {
            self.eat(SyntaxKind::DefaultKw);
            self.marker();
        } else if self.at_any(MARKERS)
            && self.nth(1) == Some(SyntaxKind::Ident)
            && !self.eat(SyntaxKind::CellKw)
        {
            let word = self.current_text();
            let reported = self.diagnostics.len();
            self.error(
                code,
                format!("a function parameter takes the marker `cell` alone, not `{word}`"),
            );
            self.help_since(reported, code, FN_MARKER);
        }
        let named = self.expect_name(code, "a parameter name");
        if named && self.eat(SyntaxKind::Colon) {
            self.then([Task::Type, Task::ParamDefault { pattern }]);
        } else {
            self.then([Task::ParamDefault { pattern }]);
        }
    }

    fn marker(&mut self) {
        if !self.at_any(MARKERS) {
            return;
        }
        if self.eat(SyntaxKind::OutKw) {
            if self.at_any(&[
                SyntaxKind::StateKw,
                SyntaxKind::StreamKw,
                SyntaxKind::ChannelKw,
            ]) {
                self.bump();
            }
        } else {
            self.bump();
        }
    }

    pub(super) fn param_default(&mut self, pattern: bool) {
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

    pub(super) fn fn_result(&mut self) {
        if self.at(SyntaxKind::Arrow) {
            self.start(SyntaxKind::FnResult);
            self.bump();
            self.then([Task::Type, Task::Finish, Task::FnColon]);
        } else {
            self.fn_colon();
        }
    }

    pub(super) fn fn_colon(&mut self) {
        let help = if self.at(SyntaxKind::Eq) {
            Some(FN_COLON)
        } else if self.at(SyntaxKind::Ident) && self.current_text() == "bounded" {
            Some(FN_BOUND)
        } else {
            None
        };
        let reported = self.diagnostics.len();
        let colon = self.colon_or_skip(Code::FnSyntax, "`:` and the body");
        if let (Some(help), Some(diagnostic)) = (help, self.diagnostics.get_mut(reported)) {
            diagnostic.help = Some(help.to_string());
        }
        if colon {
            self.bump();
            self.then([Task::FnBody, Task::Finish]);
        } else {
            self.finish();
        }
    }

    pub(super) fn pattern_type(&mut self) {
        if self.eat(SyntaxKind::Arrow) {
            self.placement_type(Code::PatternSyntax);
        }
        self.then([Task::PatternAfterParams]);
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
            self.then([
                Task::WholeExpr { own_line: false },
                Task::FinishLineFinish(Code::FnSyntax),
            ]);
        }
    }

    pub(super) fn fn_lets(&mut self) {
        if self.at(SyntaxKind::LetKw) {
            self.then([Task::LetDecl(SyntaxKind::LocalLet), Task::FnLets]);
        } else if self.at_cell_word() {
            self.misplaced_cell();
            self.then([Task::CellDecl, Task::FnLets]);
        } else if self.at(SyntaxKind::MatchKw) {
            self.then([Task::MatchBlock, Task::FnBodyBlockEnd]);
        } else {
            self.then([
                Task::WholeExpr { own_line: true },
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
}

const CELL_CAPACITY: &str = "write the capacity in digits, as in `stream[16] notes(0)`";

const FN_COLON: &str = "write `:` before a function's body, as in `fn double(x): x * 2`";

const FN_BOUND: &str = "remove the bound and write the loop as a `for` expression, as in `for x in held into low from 127: min(low, x)`";

const INPUT_FORM: &str = "write `state <name>(<init>)`";

const MARKERS: &[SyntaxKind] = &[
    SyntaxKind::ConstKw,
    SyntaxKind::CellKw,
    SyntaxKind::StateKw,
    SyntaxKind::StreamKw,
    SyntaxKind::ChannelKw,
    SyntaxKind::OutKw,
];

const FN_MARKER: &str =
    "mark the parameter `cell`, as in `fn rise(cell x): x.past(1)`, or write no marker";
