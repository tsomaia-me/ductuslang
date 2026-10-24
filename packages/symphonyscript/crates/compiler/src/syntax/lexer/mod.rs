mod clause;
mod mode;
mod scan;

use symphonyscript_core::{RESERVED_WORDS, Span, Vocabulary};
use unicode_width::UnicodeWidthChar;

use self::clause::Clause;
use self::mode::Mode;
use self::scan::{is_ident_continue, is_ident_start};
use super::lexed::Lexed;
use super::nfc::nfc;
use super::syntax_kind::SyntaxKind;
use super::token::Token;
use crate::diagnostics::{Code, Diagnostic};
use crate::source::Source;

pub(crate) struct Lexer<'a> {
    source: &'a Source,
    text: &'a str,
    vocabulary: &'a Vocabulary,
    pos: usize,
    tokens: Vec<Token>,
    diagnostics: Vec<Diagnostic>,
    stack: Vec<u32>,
    depth: u32,
    modes: Vec<Mode>,
    line_start: usize,
    line_has_token: bool,
    pending_newline: Option<usize>,
    clause: Option<Clause>,
    clause_pending: bool,
}

impl<'a> Lexer<'a> {
    pub(crate) fn new(source: &'a Source, vocabulary: &'a Vocabulary) -> Self {
        Self {
            source,
            text: &source.text,
            vocabulary,
            pos: 0,
            tokens: Vec::new(),
            diagnostics: Vec::new(),
            stack: vec![0],
            depth: 0,
            modes: Vec::new(),
            line_start: 0,
            line_has_token: false,
            pending_newline: None,
            clause: None,
            clause_pending: false,
        }
    }

    pub(crate) fn run(mut self) -> Lexed {
        while self.pos < self.text.len() {
            self.line();
        }
        if self.line_has_token {
            self.push(SyntaxKind::Newline, self.pos);
        }
        while self.stack.len() > 1 {
            self.stack.pop();
            self.push(SyntaxKind::Dedent, self.pos);
        }
        Lexed {
            tokens: self.tokens,
            diagnostics: self.diagnostics,
        }
    }

    fn line(&mut self) {
        self.line_start = self.pos;
        let rest = &self.text[self.pos..];
        let indent = rest.len() - rest.trim_start_matches([' ', '\t']).len();
        let after = &rest[indent..];
        let blank = after.is_empty()
            || after.starts_with('\n')
            || after.starts_with("\r\n")
            || after.starts_with("//");
        if indent > 0 {
            self.pos += indent;
            self.push(SyntaxKind::Whitespace, self.line_start);
        }
        if !blank {
            self.layout();
        }
        self.rest_of_line();
        self.line_break();
    }

    fn layout(&mut self) {
        if self.depth > 0 {
            return;
        }
        let leading = &self.text[self.line_start..self.pos];
        if let Some(tab) = leading.find('\t') {
            let start = self.line_start + tab;
            self.error(
                Code::TabIndentation,
                start,
                start + 1,
                "a tab in leading whitespace; indent with spaces",
            );
        }
        let column = leading.chars().count() as u32;
        let top = self.top();
        if let Some(clause) = self.clause.take() {
            if column == clause.column && clause.column > top {
                if let Some(index) = self.pending_newline.take() {
                    self.tokens[index].kind = SyntaxKind::Whitespace;
                }
                self.line_has_token = true;
                self.clause = Some(clause);
                return;
            }
            if column == clause.display && column != clause.column {
                let end = self.pos + self.char_len(self.pos);
                self.diagnostics.push(
                    Diagnostic::new(
                        Code::ClauseColumn,
                        self.span(self.pos, end),
                        format!(
                            "this line lines up with the attribute clause only on screen; its first entry is at column {}",
                            clause.column + 1
                        ),
                    )
                    .with_hint(format!(
                        "wide characters before the clause make column {} show at display column {}",
                        clause.column + 1,
                        clause.display + 1
                    )),
                );
            }
        }
        self.pending_newline = None;
        if column > top {
            self.stack.push(column);
            self.push(SyntaxKind::Indent, self.pos);
        } else if column < top {
            while column < self.top() {
                self.stack.pop();
                self.push(SyntaxKind::Dedent, self.pos);
            }
            if column != self.top() {
                let end = self.pos + self.char_len(self.pos);
                self.error(
                    Code::DedentMismatch,
                    self.pos,
                    end,
                    format!(
                        "this line's indentation, {column}, matches no enclosing block; the enclosing block is at {}",
                        self.top()
                    ),
                );
            }
        }
    }

    fn rest_of_line(&mut self) {
        loop {
            if self.at_line_end() {
                if let Some(start) = self.unterminated_start() {
                    self.error(
                        Code::UnterminatedString,
                        start,
                        self.pos,
                        "this string does not end on its line",
                    );
                    self.close_strings();
                }
                return;
            }
            match self.modes.last() {
                Some(Mode::String { .. }) => self.string_part(),
                _ => self.token(),
            }
        }
    }

    fn line_break(&mut self) {
        let start = self.pos;
        if self.text[self.pos..].starts_with("\r\n") {
            self.pos += 2;
        } else if self.text[self.pos..].starts_with('\n') {
            self.pos += 1;
        } else {
            return;
        }
        self.clause_pending = false;
        if self.depth == 0 && self.line_has_token {
            self.push(SyntaxKind::Newline, start);
            self.pending_newline = Some(self.tokens.len() - 1);
            self.line_has_token = false;
        } else {
            self.push(SyntaxKind::Whitespace, start);
        }
    }

    fn at_line_end(&self) -> bool {
        let rest = &self.text[self.pos..];
        rest.is_empty() || rest.starts_with('\n') || rest.starts_with("\r\n")
    }

    fn unterminated_start(&self) -> Option<usize> {
        self.modes.iter().find_map(|mode| match mode {
            Mode::String { start, .. } => Some(*start),
            Mode::Interp { .. } => None,
        })
    }

    fn close_strings(&mut self) {
        if let Some(Mode::String { depth, .. }) = self.modes.first() {
            self.depth = *depth;
        }
        self.modes.clear();
    }

    fn token(&mut self) {
        let start = self.pos;
        let rest = &self.text[start..];
        let mut chars = rest.chars();
        let Some(c) = chars.next() else {
            return;
        };
        let next = chars.next();
        match c {
            ' ' | '\t' => {
                let len = rest.len() - rest.trim_start_matches([' ', '\t']).len();
                self.pos += len;
                self.push(SyntaxKind::Whitespace, start);
            }
            '/' if next == Some('/') => {
                let end = rest.find('\n').unwrap_or(rest.len());
                let end = if rest[..end].ends_with('\r') && end < rest.len() {
                    end - 1
                } else {
                    end
                };
                self.pos += end;
                self.push(SyntaxKind::Comment, start);
            }
            '"' => {
                self.pos += 1;
                self.push(SyntaxKind::StringStart, start);
                self.modes.push(Mode::String {
                    start,
                    depth: self.depth,
                });
            }
            '}' if matches!(self.modes.last(), Some(Mode::Interp { .. })) => {
                self.pos += 1;
                if let Some(Mode::Interp { depth }) = self.modes.pop() {
                    self.depth = depth;
                }
                self.push(SyntaxKind::InterpEnd, start);
            }
            '0'..='9' => self.number(),
            '.' if next.is_some_and(|n| n.is_ascii_digit()) => self.leading_point(),
            c if is_ident_start(c) => self.ident(),
            _ => self.punct(c, next),
        }
    }

    fn string_part(&mut self) {
        let start = self.pos;
        let rest = &self.text[start..];
        match rest.chars().next() {
            Some('"') => {
                self.pos += 1;
                self.push(SyntaxKind::StringEnd, start);
                if let Some(Mode::String { depth, .. }) = self.modes.pop() {
                    self.depth = depth;
                }
            }
            Some('{') => {
                self.pos += 1;
                self.push(SyntaxKind::InterpStart, start);
                self.modes.push(Mode::Interp { depth: self.depth });
            }
            _ => {
                let mut chars = rest.char_indices().peekable();
                let mut end = rest.len();
                while let Some((offset, c)) = chars.next() {
                    match c {
                        '"' | '{' => {
                            end = offset;
                            break;
                        }
                        '\n' => {
                            end = offset;
                            break;
                        }
                        '\r' if rest[offset..].starts_with("\r\n") => {
                            end = offset;
                            break;
                        }
                        '\\' => match chars.peek().copied() {
                            Some((_, '"' | '\\' | 'n' | 't' | '{')) => {
                                chars.next();
                            }
                            Some((escaped_at, escaped)) if escaped != '\n' && escaped != '\r' => {
                                chars.next();
                                let escape_start = start + offset;
                                let escape_end = start + escaped_at + escaped.len_utf8();
                                self.error(
                                    Code::InvalidEscape,
                                    escape_start,
                                    escape_end,
                                    format!("`\\{escaped}` is not an escape"),
                                );
                            }
                            _ => {
                                let escape_start = start + offset;
                                self.error(
                                    Code::InvalidEscape,
                                    escape_start,
                                    escape_start + 1,
                                    "a backslash at the end of a line escapes nothing",
                                );
                            }
                        },
                        _ => {}
                    }
                }
                self.pos += end;
                self.push(SyntaxKind::StringText, start);
            }
        }
    }

    fn ident(&mut self) {
        let start = self.pos;
        let rest = &self.text[start..];
        let len = rest
            .char_indices()
            .find(|(offset, c)| *offset > 0 && !is_ident_continue(*c))
            .map_or(rest.len(), |(offset, _)| offset);
        self.pos += len;
        let text = &self.text[start..self.pos];
        let name = nfc(text);
        let kind = SyntaxKind::keyword(&name).unwrap_or(SyntaxKind::Ident);
        if !text.chars().any(|c| c != '_' && c != '#') {
            self.error(
                Code::InvalidIdentifier,
                start,
                self.pos,
                format!("`{text}` is not an identifier: it holds only `_` and `#`"),
            );
        } else if RESERVED_WORDS.contains(&name.as_ref()) {
            self.error(
                Code::ReservedWord,
                start,
                self.pos,
                format!("`{name}` is reserved and cannot be used"),
            );
        }
        self.push(kind, start);
    }

    fn number(&mut self) {
        let start = self.pos;
        let bytes = self.text.as_bytes();
        let prefix = match (bytes.get(start + 1), bytes.get(start + 2)) {
            (Some(b'x'), Some(d)) if d.is_ascii_hexdigit() => Some(16),
            (Some(b'b'), Some(b'0' | b'1')) => Some(2),
            (Some(b'o'), Some(b'0'..=b'7')) => Some(8),
            _ => None,
        };
        if bytes[start] == b'0'
            && let Some(radix) = prefix
        {
            self.pos += 2;
            let digits = self.digits(radix);
            let tail = self.pos;
            let rest = &self.text[tail..];
            let len = rest
                .char_indices()
                .find(|(_, c)| !is_ident_continue(*c))
                .map_or(rest.len(), |(offset, _)| offset);
            if len > 0 {
                self.pos += len;
                self.error(
                    Code::PrefixedSuffix,
                    tail,
                    self.pos,
                    format!(
                        "`{}` directly follows a prefixed literal, which takes no suffix",
                        &self.text[tail..self.pos]
                    ),
                );
            }
            self.check_magnitude(digits, radix);
            self.push(SyntaxKind::Number, start);
            return;
        }
        let integer = self.digits(10);
        let mut float = false;
        if bytes.get(self.pos) == Some(&b'.')
            && bytes.get(self.pos + 1).is_some_and(u8::is_ascii_digit)
        {
            self.pos += 1;
            self.digits(10);
            float = true;
        }
        if self.exponent() {
            float = true;
        }
        self.suffix(float);
        if !float {
            self.check_magnitude(integer, 10);
        }
        self.push(SyntaxKind::Number, start);
    }

    fn leading_point(&mut self) {
        let start = self.pos;
        self.pos += 1;
        self.digits(10);
        self.exponent();
        self.suffix(true);
        self.error(
            Code::FloatDigits,
            start,
            self.pos,
            "a float needs a digit before its point",
        );
        self.push(SyntaxKind::Number, start);
    }

    fn exponent(&mut self) -> bool {
        let bytes = self.text.as_bytes();
        let at = self.pos;
        if !matches!(bytes.get(at), Some(b'e' | b'E')) {
            return false;
        }
        let digit_at = match bytes.get(at + 1) {
            Some(b'+' | b'-') => at + 2,
            _ => at + 1,
        };
        if !bytes.get(digit_at).is_some_and(u8::is_ascii_digit) {
            return false;
        }
        self.pos = digit_at;
        self.digits(10);
        true
    }

    fn suffix(&mut self, float: bool) {
        let start = self.pos;
        let rest = &self.text[start..];
        if !rest.chars().next().is_some_and(is_ident_start) {
            return;
        }
        let len = rest
            .char_indices()
            .find(|(offset, c)| *offset > 0 && !is_ident_continue(*c))
            .map_or(rest.len(), |(offset, _)| offset);
        self.pos += len;
        let text = &self.text[start..self.pos];
        let name = nfc(text);
        match name.as_ref() {
            "i32" | "i64" if float => self.error(
                Code::FloatTypeSuffix,
                start,
                self.pos,
                format!("`{name}` is an integer type; a float literal cannot take it"),
            ),
            "i32" | "i64" | "f32" | "f64" => {}
            unit if self.vocabulary.unit_named(unit).is_some() => {}
            _ => self.error(
                Code::UnknownSuffix,
                start,
                self.pos,
                format!("`{text}` is neither a type suffix nor a unit of the vocabulary"),
            ),
        }
    }

    fn digits(&mut self, radix: u32) -> (usize, usize) {
        let start = self.pos;
        let bytes = self.text.as_bytes();
        while let Some(&byte) = bytes.get(self.pos) {
            if byte == b'_' || char::from(byte).is_digit(radix) {
                self.pos += 1;
            } else {
                break;
            }
        }
        let run = &bytes[start..self.pos];
        for (offset, byte) in run.iter().enumerate() {
            if *byte != b'_' {
                continue;
            }
            let before = offset > 0 && run[offset - 1] != b'_';
            let after = run.get(offset + 1).is_some_and(|next| *next != b'_');
            if !(before && after) {
                self.error(
                    Code::MisplacedUnderscore,
                    start + offset,
                    start + offset + 1,
                    "an underscore may only separate two digits",
                );
            }
        }
        (start, self.pos)
    }

    fn check_magnitude(&mut self, (start, end): (usize, usize), radix: u32) {
        let mut value: u128 = 0;
        let mut overflow = false;
        for byte in self.text[start..end].bytes().filter(|byte| *byte != b'_') {
            let digit = char::from(byte).to_digit(radix).unwrap_or(0);
            match value
                .checked_mul(u128::from(radix))
                .and_then(|v| v.checked_add(u128::from(digit)))
            {
                Some(next) => value = next,
                None => {
                    overflow = true;
                    break;
                }
            }
        }
        if overflow || value >= 1_u128 << 127 {
            self.error(
                Code::IntegerTooLarge,
                start,
                end,
                "an integer literal's magnitude must stay below 2^127",
            );
        }
    }

    fn punct(&mut self, c: char, next: Option<char>) {
        let start = self.pos;
        let (kind, len) = match (c, next) {
            ('(', _) => (SyntaxKind::LParen, 1),
            (')', _) => (SyntaxKind::RParen, 1),
            ('[', _) => (SyntaxKind::LBracket, 1),
            (']', _) => (SyntaxKind::RBracket, 1),
            (',', _) => (SyntaxKind::Comma, 1),
            (':', Some(':')) => (SyntaxKind::Colon2, 2),
            (':', _) => (SyntaxKind::Colon, 1),
            ('=', Some('=')) => {
                self.pos += 2;
                self.diagnostics.push(
                    Diagnostic::new(
                        Code::ReservedToken,
                        self.span(start, self.pos),
                        "`==` is reserved",
                    )
                    .with_hint("compare with `is`"),
                );
                self.push(SyntaxKind::Error, start);
                return;
            }
            ('!', Some('=')) => {
                self.pos += 2;
                self.diagnostics.push(
                    Diagnostic::new(
                        Code::ReservedToken,
                        self.span(start, self.pos),
                        "`!=` is reserved",
                    )
                    .with_hint("compare with `is not`"),
                );
                self.push(SyntaxKind::Error, start);
                return;
            }
            ('^', _) => {
                self.pos += 1;
                self.error(Code::ReservedToken, start, self.pos, "`^` is reserved");
                self.push(SyntaxKind::Error, start);
                return;
            }
            (';', _) => {
                self.pos += 1;
                self.error(
                    Code::Semicolon,
                    start,
                    self.pos,
                    "a semicolon is not allowed; end the line instead",
                );
                self.push(SyntaxKind::Error, start);
                return;
            }
            ('=', _) => (SyntaxKind::Eq, 1),
            ('|', _) => (SyntaxKind::Pipe, 1),
            ('/', _) => (SyntaxKind::Slash, 1),
            ('.', Some('.')) => (SyntaxKind::Dot2, 2),
            ('.', _) => (SyntaxKind::Dot, 1),
            ('+', _) => (SyntaxKind::Plus, 1),
            ('-', _) => (SyntaxKind::Minus, 1),
            ('*', Some('*')) => (SyntaxKind::Star2, 2),
            ('*', _) => (SyntaxKind::Star, 1),
            ('%', _) => (SyntaxKind::Percent, 1),
            ('<', Some('=')) => (SyntaxKind::Le, 2),
            ('<', _) => (SyntaxKind::Lt, 1),
            ('>', Some('=')) => (SyntaxKind::Ge, 2),
            ('>', _) => (SyntaxKind::Gt, 1),
            ('!', _) => (SyntaxKind::Bang, 1),
            ('\'', _) => (SyntaxKind::Quote, 1),
            ('?', _) => (SyntaxKind::Question, 1),
            ('~', _) => (SyntaxKind::Tilde, 1),
            ('@', _) => (SyntaxKind::At, 1),
            ('$', _) => (SyntaxKind::Dollar, 1),
            _ => {
                self.unexpected();
                return;
            }
        };
        self.pos += len;
        self.push(kind, start);
    }

    fn unexpected(&mut self) {
        let start = self.pos;
        while let Some(c) = self.text[self.pos..].chars().next() {
            if self.pos > start
                && !scan::is_unexpected(c, self.text[self.pos..].starts_with("\r\n"))
            {
                break;
            }
            self.pos += c.len_utf8();
        }
        let text = &self.text[start..self.pos];
        self.error(
            Code::UnexpectedCharacter,
            start,
            self.pos,
            format!("unexpected `{}`", text.escape_debug()),
        );
        self.push(SyntaxKind::Error, start);
    }

    fn push(&mut self, kind: SyntaxKind, start: usize) {
        let normal = self.modes.is_empty();
        if !kind.is_trivia() {
            if !matches!(
                kind,
                SyntaxKind::Newline | SyntaxKind::Indent | SyntaxKind::Dedent
            ) {
                if normal && self.clause_pending {
                    self.clause_pending = false;
                    let prefix = &self.text[self.line_start..start];
                    self.clause = Some(Clause {
                        column: prefix.chars().count() as u32,
                        display: prefix.chars().map(|c| c.width().unwrap_or(1) as u32).sum(),
                    });
                }
                self.line_has_token = true;
            }
            match kind {
                SyntaxKind::Pipe if normal && self.depth == 0 && self.clause.is_none() => {
                    self.clause_pending = true;
                }
                SyntaxKind::Colon if normal && self.depth == 0 => {
                    self.clause = None;
                    self.clause_pending = false;
                }
                SyntaxKind::LParen | SyntaxKind::LBracket => self.depth += 1,
                SyntaxKind::RParen | SyntaxKind::RBracket => {
                    self.depth = self.depth.saturating_sub(1);
                }
                _ => {}
            }
        }
        self.tokens.push(Token {
            kind,
            range: start as u32..self.pos as u32,
        });
    }

    fn top(&self) -> u32 {
        self.stack.last().copied().unwrap_or(0)
    }

    fn char_len(&self, at: usize) -> usize {
        self.text[at..].chars().next().map_or(0, char::len_utf8)
    }

    fn span(&self, start: usize, end: usize) -> Span {
        Span {
            file: self.source.path.clone(),
            range: start as u32..end as u32,
        }
    }

    fn error(&mut self, code: Code, start: usize, end: usize, message: impl Into<String>) {
        let span = self.span(start, end);
        self.diagnostics.push(Diagnostic::new(code, span, message));
    }
}
