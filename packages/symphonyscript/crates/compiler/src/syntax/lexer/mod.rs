mod clause;
mod line_start;
mod mode;
mod raw_diagnostic;
mod region;
mod scan;
mod suspension;
#[cfg(test)]
mod tests;

use symphonyscript_core::{RESERVED_WORDS, Vocabulary};
use unicode_width::UnicodeWidthChar;

use std::ops::Range;

use self::clause::Clause;
pub(crate) use self::line_start::{LineStart, LineToken, LineTokens};
use self::mode::Mode;
use self::raw_diagnostic::RawDiagnostic;
use self::region::Region;
pub(crate) use self::scan::is_ident_continue;
use self::scan::is_ident_start;
use self::suspension::Suspension;
use super::lexed::Lexed;
use super::nfc::nfc;
use super::syntax_kind::SyntaxKind;
use super::token_columns::TokenColumns;
use crate::diagnostics::{Code, Diagnostic, stray_closer_help};
use crate::semantics::counted_walk::CountedWalk;
use crate::source::Source;

pub(crate) struct Lexer<'a> {
    source: &'a Source,
    text: &'a str,
    vocabulary: &'a Vocabulary,
    pos: usize,
    columns: TokenColumns,
    diagnostics: Vec<RawDiagnostic>,
    stack: Vec<u32>,
    depth: u32,
    modes: Vec<Mode>,
    line_start: usize,
    line_has_token: bool,
    pending_newline: Option<usize>,
    clause: Option<Clause>,
    clause_pending: bool,
    depths: Vec<u32>,
    suspended: Vec<Suspension>,
    regions: Vec<Region>,
    stacks: Vec<Vec<u32>>,
    stored_entries: usize,
    peak_stored_entries: usize,
    line_stack: Option<usize>,
    faults: Vec<Range<u32>>,
    reach: Vec<u32>,
    cut_constructs: Vec<Range<u32>>,
    resume_line: Option<usize>,
    finished: bool,
    json_next: bool,
    work: usize,
}

impl<'a> Lexer<'a> {
    pub(crate) fn new(source: &'a Source, vocabulary: &'a Vocabulary) -> Self {
        Self {
            source,
            text: &source.text,
            vocabulary,
            pos: 0,
            columns: TokenColumns::reserved(source.text.len()),
            diagnostics: Vec::new(),
            stack: vec![0],
            depth: 0,
            modes: Vec::new(),
            line_start: 0,
            line_has_token: false,
            pending_newline: None,
            clause: None,
            clause_pending: false,
            depths: Vec::with_capacity(source.text.len() / 2),
            suspended: Vec::new(),
            regions: Vec::new(),
            stacks: Vec::new(),
            stored_entries: 0,
            peak_stored_entries: 0,
            line_stack: None,
            faults: Vec::new(),
            reach: Vec::new(),
            cut_constructs: Vec::new(),
            resume_line: None,
            finished: false,
            json_next: false,
            work: 0,
        }
    }

    pub(crate) fn run(mut self) -> Lexed {
        while self.advance() {}
        let (columns, diagnostics) = self.finish();
        Lexed {
            tokens: columns.tokens(),
            diagnostics,
        }
    }

    pub(crate) fn finish(mut self) -> (TokenColumns, Vec<Diagnostic>) {
        self.columns.starts.push(self.pos as u32);
        let file = self.source.path.as_str();
        let diagnostics = self
            .diagnostics
            .into_iter()
            .map(|diagnostic| diagnostic.into_diagnostic(file))
            .collect();
        (self.columns, diagnostics)
    }

    pub(crate) fn advance(&mut self) -> bool {
        if let Some(line_start) = self.resume_line.take() {
            self.line_at(line_start);
            return true;
        }
        if self.pos < self.text.len() {
            self.line_at(self.pos);
            return true;
        }
        if self.finished {
            return false;
        }
        self.finished = true;
        if self.line_has_token {
            self.push(SyntaxKind::Newline, self.pos);
        }
        while self.stack.len() > 1 {
            self.stack.pop();
            self.push(SyntaxKind::Dedent, self.pos);
        }
        true
    }

    pub(crate) fn settled(&self) -> usize {
        match self.pending_newline {
            Some(index) if !self.finished => index,
            _ => self.columns.len(),
        }
    }

    pub(crate) fn kind_of(&self, index: usize) -> Option<SyntaxKind> {
        self.columns.kind(index)
    }

    pub(crate) fn range_of(&self, index: usize) -> Option<Range<u32>> {
        let start = self.columns.start(index)?;
        Some(start..self.end_of(index))
    }

    pub(crate) fn significant_at(&self, at: usize) -> Option<usize> {
        self.columns
            .sig
            .get(at)
            .map(|index| *index as usize)
            .filter(|index| *index < self.settled())
    }

    fn end_of(&self, index: usize) -> u32 {
        self.columns.start(index + 1).unwrap_or(self.pos as u32)
    }

    pub(crate) fn work(&self) -> usize {
        self.work
    }

    pub(crate) fn peak_stored_entries(&self) -> usize {
        self.peak_stored_entries
    }

    pub(crate) fn faulted_at(&mut self, offset: u32) -> bool {
        let work = &mut self.work;
        let before = self.faults.partition_point(|fault| {
            *work += 1;
            fault.start <= offset
        });
        before
            .checked_sub(1)
            .and_then(|index| self.reach.get(index))
            .is_some_and(|reach| *reach > offset)
    }

    pub(crate) fn in_cut_construct(&mut self, offset: u32) -> bool {
        let work = &mut self.work;
        let before = self.cut_constructs.partition_point(|construct| {
            *work += 1;
            construct.start <= offset
        });
        before
            .checked_sub(1)
            .and_then(|index| self.cut_constructs.get(index))
            .is_some_and(|construct| construct.end > offset)
    }

    pub(crate) fn token_before(&mut self, offset: u32) -> Option<usize> {
        let work = &mut self.work;
        let starts = &self.columns.starts;
        let mut at = self.columns.sig.partition_point(|token| {
            *work += 1;
            starts
                .get(*token as usize)
                .is_some_and(|start| *start < offset)
        });
        while at > 0 {
            at -= 1;
            self.work += 1;
            let index = *self.columns.sig.get(at)? as usize;
            if !self.range_of(index)?.is_empty() {
                return Some(index);
            }
        }
        None
    }

    pub(crate) fn resync_line(
        &mut self,
        token: usize,
        starts_line: impl Fn(&LineStart<'_>) -> bool,
    ) -> Option<usize> {
        let start = self.columns.start(token)? as usize;
        let region = self
            .regions
            .partition_point(|region| region.open <= start)
            .checked_sub(1)?;
        let inside = self.depths.get(token).copied().unwrap_or(0) > 0;
        let opens = self
            .regions
            .get(region)
            .is_some_and(|region| region.open == start);
        if !inside && !opens {
            return None;
        }
        let top = self
            .regions
            .get(region)
            .and_then(|region| self.stacks.get(region.stack))?
            .last()
            .copied()
            .unwrap_or(0);
        let mut next = self
            .suspended
            .partition_point(|line| line.first_token < token);
        loop {
            match self.suspended.get(next) {
                Some(line) if line.region != region => return None,
                Some(line) => {
                    if line.column <= top
                        && self
                            .line_start_of(line)
                            .is_some_and(|start| starts_line(&start))
                    {
                        return Some(next);
                    }
                    next += 1;
                }
                None => {
                    let open = self.depth > 0 && self.regions.len() == region + 1;
                    if !open || !self.advance() {
                        return None;
                    }
                }
            }
        }
    }

    pub(crate) fn depth_at(&self, index: usize) -> Option<u32> {
        self.depths.get(index).copied()
    }

    pub(crate) fn suspended_line_at(&self, token: usize) -> Option<(usize, LineStart<'_>)> {
        if self.depths.get(token).copied().unwrap_or(0) == 0 {
            return None;
        }
        let index = self
            .suspended
            .binary_search_by_key(&token, |line| line.first_token)
            .ok()?;
        let line = self.suspended.get(index)?;
        let top = self
            .regions
            .get(line.region)
            .and_then(|region| self.stacks.get(region.stack))
            .and_then(|stack| stack.last().copied())
            .unwrap_or(0);
        if line.column > top {
            return None;
        }
        Some((index, self.line_start_of(line)?))
    }

    fn line_start_of(&self, line: &Suspension) -> Option<LineStart<'_>> {
        let first = LineToken {
            kind: self.kind_of(line.first_token)?,
            start: self.columns.start(line.first_token)?,
            end: self.end_of(line.first_token),
        };
        let line_end =
            self.text
                .get(first.start as usize..)
                .and_then(|rest| rest.find('\n'))
                .map_or(self.text.len(), |offset| first.start as usize + offset) as u32;
        CountedWalk::LexByte.step_by((line_end - first.start) as usize);
        let after = self
            .columns
            .starts
            .get(line.first_token + 1..)
            .unwrap_or_default();
        let length = after.partition_point(|start| *start < line_end);
        let rest = LineTokens {
            kinds: self
                .columns
                .kinds
                .get(line.first_token + 1..)
                .and_then(|kinds| kinds.get(..length))
                .unwrap_or_default(),
            starts: after.get(..length).unwrap_or_default(),
            end: self.end_of(line.first_token + length),
        };
        let rest_depths = self
            .depths
            .get(line.first_token + 1..)
            .and_then(|depths| depths.get(..length))
            .unwrap_or_default();
        let closes_region = rest.iter().zip(rest_depths).any(|(token, depth)| {
            *depth == 1
                && matches!(
                    token.kind,
                    SyntaxKind::RParen | SyntaxKind::RBracket | SyntaxKind::RBrace
                )
        });
        Some(LineStart {
            first,
            first_depth: self.depths.get(line.first_token).copied().unwrap_or(0),
            rest,
            rest_depths,
            top_level: line.column == 0,
            closes_region,
        })
    }

    pub(crate) fn resynchronize(&mut self, line: usize, consumed: usize) {
        let Some(suspension) = self.suspended.get(line).cloned() else {
            return;
        };
        let keep = self
            .columns
            .starts
            .partition_point(|start| (*start as usize) < suspension.offset)
            .max(consumed);
        let from = keep
            .checked_sub(1)
            .filter(|index| *index < self.columns.len())
            .map_or(suspension.offset, |index| {
                (self.end_of(index) as usize).max(suspension.offset)
            });
        let snapshot = self
            .regions
            .get(suspension.region)
            .map_or(0, |region| region.stack);
        let stack = self
            .stacks
            .get(snapshot)
            .cloned()
            .unwrap_or_else(|| vec![0]);
        self.columns.kinds.truncate(keep);
        self.columns.starts.truncate(keep);
        let significant = self
            .columns
            .sig
            .partition_point(|token| (*token as usize) < keep);
        self.columns.sig.truncate(significant);
        self.depths.truncate(keep);
        let offset = suspension.offset as u32;
        let kept = self
            .diagnostics
            .partition_point(|diagnostic| diagnostic.range.start < offset);
        self.diagnostics.truncate(kept);
        let kept = self.faults.partition_point(|fault| fault.start < offset);
        self.faults.truncate(kept);
        self.reach.truncate(kept);
        let kept = self
            .cut_constructs
            .partition_point(|construct| construct.start < offset);
        self.cut_constructs.truncate(kept);
        self.suspended.truncate(line);
        self.regions.truncate(suspension.region + 1);
        let dropped: usize = self
            .stacks
            .get(snapshot + 1..)
            .unwrap_or_default()
            .iter()
            .map(Vec::len)
            .sum();
        self.stored_entries = self.stored_entries.saturating_sub(dropped);
        self.stacks.truncate(snapshot + 1);
        self.line_stack = None;
        self.pos = from;
        self.line_start = suspension.offset;
        self.stack = stack;
        self.depth = 0;
        self.modes.clear();
        self.line_has_token = false;
        self.pending_newline = None;
        self.clause = None;
        self.clause_pending = false;
        self.finished = false;
        self.json_next = false;
        self.push(SyntaxKind::Newline, from);
        self.resume_line = Some(suspension.offset);
    }

    fn line_at(&mut self, line_start: usize) {
        let begin = self.pos;
        let first_diagnostic = self.diagnostics.len();
        self.line_start = line_start;
        self.line_stack = None;
        let rest = &self.text[self.pos..];
        let indent = rest.len() - rest.trim_start_matches([' ', '\t']).len();
        let after = &rest[indent..];
        let blank = after.is_empty()
            || after.starts_with('\n')
            || after.starts_with("\r\n")
            || after.starts_with("//");
        if indent > 0 {
            let start = self.pos;
            self.pos += indent;
            self.push(SyntaxKind::Whitespace, start);
        }
        if !blank {
            self.layout();
        }
        self.rest_of_line();
        self.line_break();
        self.record_faults(first_diagnostic);
        self.work += self.pos - begin;
        CountedWalk::LexByte.step_by(self.pos - begin);
    }

    fn record_faults(&mut self, first: usize) {
        let mut ranges: Vec<Range<u32>> = self
            .diagnostics
            .get(first..)
            .unwrap_or_default()
            .iter()
            .map(|diagnostic| diagnostic.range.clone())
            .collect();
        ranges.sort_by_key(|range| range.start);
        for range in ranges {
            let end = range.end.max(range.start.saturating_add(1));
            let reach = self.reach.last().copied().unwrap_or(0).max(end);
            self.faults.push(range);
            self.reach.push(reach);
        }
    }

    fn layout(&mut self) {
        let leading = &self.text[self.line_start..self.pos];
        if self.depth > 0 && self.ends_region_here(leading.chars().count() as u32) {
            self.depth = 0;
            self.modes.clear();
            self.clause = None;
            self.clause_pending = false;
            self.pending_newline = None;
            self.push(SyntaxKind::Newline, self.pos);
        }
        let leading = &self.text[self.line_start..self.pos];
        if self.depth > 0 {
            self.suspended.push(Suspension {
                offset: self.line_start,
                first_token: self.columns.len(),
                column: leading.chars().count() as u32,
                region: self.regions.len().saturating_sub(1),
            });
            return;
        }
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
            let aligned = column == clause.column && clause.column > top;
            let misaligned = column == clause.display && column != clause.column && column > top;
            if misaligned {
                let end = self.pos + self.char_len(self.pos);
                let summary = format!(
                    "this line lines up with the attribute clause only on screen; its first entry is at column {}",
                    clause.column + 1
                );
                let help = format!(
                    "wide characters before the clause make column {} show at display column {}",
                    clause.column + 1,
                    clause.display + 1
                );
                self.raw(Code::ClauseColumn, self.pos, end, summary, Some(help));
            }
            if aligned || misaligned {
                if let Some(index) = self.pending_newline.take() {
                    self.trivial(index);
                }
                self.line_has_token = true;
                self.clause = Some(clause);
                return;
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
                self.cut_constructs.push(self.pos as u32..end as u32);
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
                    self.cut_constructs.push(start as u32..self.pos as u32);
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
                Some(Mode::Json { .. }) => self.json_token(),
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
            self.json_next = false;
            self.pending_newline = Some(self.columns.len() - 1);
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
            Mode::Interp { .. } | Mode::Brace { .. } | Mode::Json { .. } | Mode::Code { .. } => {
                None
            }
        })
    }

    fn close_strings(&mut self) {
        let Some(outermost) = self
            .modes
            .iter()
            .position(|mode| matches!(mode, Mode::String { .. }))
        else {
            return;
        };
        if let Some(Mode::String { depth, .. }) = self.modes.get(outermost) {
            self.depth = *depth;
        }
        self.modes.truncate(outermost);
    }

    fn token(&mut self) {
        let start = self.pos;
        let rest = &self.text[start..];
        let mut chars = rest.chars();
        let Some(c) = chars.next() else {
            return;
        };
        let next = chars.next();
        if std::mem::take(&mut self.json_next) && self.json_value(c, next) {
            return;
        }
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
            '{' => {
                self.pos += 1;
                let depth = self.depth;
                self.push(SyntaxKind::LBrace, start);
                self.modes.push(Mode::Brace { depth });
            }
            '}' if matches!(self.modes.last(), Some(Mode::Brace { .. })) => {
                self.pos += 1;
                if let Some(Mode::Brace { depth }) = self.modes.pop() {
                    self.depth = depth.saturating_add(1);
                }
                self.push(SyntaxKind::RBrace, start);
            }
            '}' if self.depth > 0 => {
                self.pos += 1;
                self.push(SyntaxKind::RBrace, start);
            }
            '0'..='9' => self.number(),
            '.' if next.is_some_and(|n| n.is_ascii_digit()) => self.leading_point(),
            c if is_ident_start(c) => self.ident(),
            _ => self.punct(c, next),
        }
    }

    fn json_value(&mut self, c: char, next: Option<char>) -> bool {
        let start = self.pos;
        let rest = &self.text[start..];
        match c {
            ' ' | '\t' | '-' => {
                self.json_next = true;
                false
            }
            '/' if next == Some('/') => {
                self.json_next = true;
                false
            }
            '[' if rest[1..].trim_start_matches([' ', '\t']).starts_with("..") => false,
            '{' | '[' => {
                self.modes.push(Mode::Json { depth: self.depth });
                self.pos += 1;
                let kind = if c == '{' {
                    SyntaxKind::LBrace
                } else {
                    SyntaxKind::LBracket
                };
                self.push(kind, start);
                true
            }
            '"' => {
                self.json_string();
                true
            }
            '0'..='9' => {
                self.json_number();
                true
            }
            _ if word(rest) == "null" => {
                self.pos += 4;
                self.push(SyntaxKind::JsonNull, start);
                true
            }
            _ => false,
        }
    }

    fn json_token(&mut self) {
        let start = self.pos;
        let rest = &self.text[start..];
        let Some(c) = rest.chars().next() else {
            return;
        };
        match c {
            '"' => self.json_string(),
            '0'..='9' => self.json_number(),
            '(' => {
                self.modes.push(Mode::Code { depth: self.depth });
                self.pos += 1;
                self.push(SyntaxKind::LParen, start);
            }
            '{' => {
                self.pos += 1;
                self.push(SyntaxKind::LBrace, start);
            }
            '}' => {
                self.pos += 1;
                self.push(SyntaxKind::RBrace, start);
            }
            _ if word(rest) == "null" => {
                self.pos += 4;
                self.push(SyntaxKind::JsonNull, start);
            }
            _ => self.token(),
        }
    }

    fn json_string(&mut self) {
        let start = self.pos;
        let rest = &self.text[start..];
        let mut chars = rest.char_indices().skip(1).peekable();
        let mut end = None;
        let mut line_end = rest.len();
        while let Some((offset, c)) = chars.next() {
            match c {
                '"' => {
                    end = Some(offset + 1);
                    break;
                }
                '\n' => {
                    line_end = offset;
                    break;
                }
                '\r' if rest[offset..].starts_with("\r\n") => {
                    line_end = offset;
                    break;
                }
                '\\' => match chars.peek().copied() {
                    Some((_, '"' | '\\' | '/' | 'b' | 'f' | 'n' | 'r' | 't')) => {
                        chars.next();
                    }
                    Some((at, 'u')) => {
                        chars.next();
                        let mut digits = 0;
                        let mut last = at + 1;
                        while digits < 4
                            && let Some((offset, digit)) = chars.peek().copied()
                            && digit.is_ascii_hexdigit()
                        {
                            chars.next();
                            digits += 1;
                            last = offset + 1;
                        }
                        if digits < 4 {
                            self.error(
                                Code::JsonSyntax,
                                start + offset,
                                start + last,
                                "`\\u` in JSON text is followed by four hexadecimal digits",
                            );
                        }
                    }
                    Some((at, escaped)) if escaped != '\n' && escaped != '\r' => {
                        chars.next();
                        self.error(
                            Code::JsonSyntax,
                            start + offset,
                            start + at + escaped.len_utf8(),
                            format!("`\\{escaped}` is not an escape of JSON text"),
                        );
                    }
                    _ => self.error(
                        Code::JsonSyntax,
                        start + offset,
                        start + offset + 1,
                        "a backslash at the end of a line escapes nothing",
                    ),
                },
                c if u32::from(c) < 0x20 => self.error(
                    Code::JsonSyntax,
                    start + offset,
                    start + offset + c.len_utf8(),
                    "a control character stands in a JSON string only as an escape",
                ),
                _ => {}
            }
        }
        match end {
            Some(end) => self.pos += end,
            None => {
                self.pos += line_end;
                self.cut_constructs.push(start as u32..self.pos as u32);
                self.error(
                    Code::JsonSyntax,
                    start,
                    self.pos,
                    "this JSON string does not end on its line",
                );
            }
        }
        self.push(SyntaxKind::JsonString, start);
    }

    fn json_number(&mut self) {
        let start = self.pos;
        let bytes = self.text.as_bytes();
        let digits = |mut at: usize| {
            while bytes.get(at).is_some_and(u8::is_ascii_digit) {
                at += 1;
            }
            at
        };
        let leading_zero =
            bytes.get(start) == Some(&b'0') && bytes.get(start + 1).is_some_and(u8::is_ascii_digit);
        let mut end = digits(start);
        if bytes.get(end) == Some(&b'.') && bytes.get(end + 1).is_some_and(u8::is_ascii_digit) {
            end = digits(end + 1);
        }
        if matches!(bytes.get(end), Some(b'e' | b'E')) {
            let digit_at = match bytes.get(end + 1) {
                Some(b'+' | b'-') => end + 2,
                _ => end + 1,
            };
            if bytes.get(digit_at).is_some_and(u8::is_ascii_digit) {
                end = digits(digit_at);
            }
        }
        let tail = self.text[end..]
            .char_indices()
            .find(|(_, c)| !is_ident_continue(*c))
            .map_or(self.text.len() - end, |(offset, _)| offset);
        self.pos = end + tail;
        if leading_zero {
            self.error(
                Code::JsonSyntax,
                start,
                end,
                "a number in JSON text has no leading zero",
            );
        } else if tail > 0 {
            self.error(
                Code::JsonSyntax,
                end,
                self.pos,
                format!(
                    "`{}` directly follows a number in JSON text, which has no prefix, separator or suffix",
                    &self.text[end..self.pos]
                ),
            );
        }
        self.push(SyntaxKind::JsonNumber, start);
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
        if kind == SyntaxKind::JsonKw {
            self.json_next = true;
        }
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
            "u8" | "i32" | "i64" if float => self.error(
                Code::FloatTypeSuffix,
                start,
                self.pos,
                format!("`{name}` is an integer type; a float literal cannot take it"),
            ),
            "u8" | "i32" | "i64" | "f32" | "f64" => {}
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
                self.raw(
                    Code::ReservedToken,
                    start,
                    self.pos,
                    "`==` is reserved",
                    Some("compare with `is`".into()),
                );
                self.push(SyntaxKind::Error, start);
                return;
            }
            ('!', Some('=')) => {
                self.pos += 2;
                self.raw(
                    Code::ReservedToken,
                    start,
                    self.pos,
                    "`!=` is reserved",
                    Some("compare with `is not`".into()),
                );
                self.push(SyntaxKind::Error, start);
                return;
            }
            ('^', _) => (SyntaxKind::Caret, 1),
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
            ('=', Some('>')) => (SyntaxKind::FatArrow, 2),
            ('=', _) => (SyntaxKind::Eq, 1),
            ('|', _) => (SyntaxKind::Pipe, 1),
            ('/', _) => (SyntaxKind::Slash, 1),
            ('.', Some('.')) => (SyntaxKind::Dot2, 2),
            ('.', _) => (SyntaxKind::Dot, 1),
            ('+', _) => (SyntaxKind::Plus, 1),
            ('-', Some('>')) => (SyntaxKind::Arrow, 2),
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
            ('?', Some('?')) => (SyntaxKind::Question2, 2),
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
        let summary = format!("unexpected `{}`", text.escape_debug());
        let mut chars = text.chars();
        let help = match (chars.next(), chars.next()) {
            (Some(closer), None) => stray_closer_help(closer),
            _ => None,
        };
        self.raw(Code::UnexpectedCharacter, start, self.pos, summary, help);
        self.push(SyntaxKind::Error, start);
    }

    fn push(&mut self, kind: SyntaxKind, start: usize) {
        CountedWalk::LexToken.step();
        let normal = self.modes.is_empty();
        self.depths.push(self.depth);
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
                SyntaxKind::LParen | SyntaxKind::LBracket | SyntaxKind::LBrace => {
                    if self.depth == 0 {
                        let stack = self.line_stack_snapshot();
                        self.regions.push(Region { open: start, stack });
                    }
                    self.depth += 1;
                }
                SyntaxKind::RParen | SyntaxKind::RBracket | SyntaxKind::RBrace => {
                    self.depth = self.depth.saturating_sub(1);
                    while let Some(
                        Mode::Brace { depth } | Mode::Json { depth } | Mode::Code { depth },
                    ) = self.modes.last()
                        && *depth >= self.depth
                    {
                        self.modes.pop();
                    }
                }
                _ => {}
            }
        }
        if !kind.is_trivia() {
            self.columns.sig.push(self.columns.len() as u32);
        }
        self.columns.kinds.push(kind);
        self.columns.starts.push(start as u32);
    }

    fn ends_region_here(&self, column: u32) -> bool {
        let top = self
            .regions
            .last()
            .and_then(|region| self.stacks.get(region.stack))
            .and_then(|stack| stack.last().copied())
            .unwrap_or(0);
        column <= top && self.starts_with_item_keyword()
    }

    fn starts_with_item_keyword(&self) -> bool {
        let rest = self.text.get(self.pos..).unwrap_or_default();
        matches!(
            SyntaxKind::keyword(word(rest)),
            Some(
                SyntaxKind::LetKw
                    | SyntaxKind::StateKw
                    | SyntaxKind::StreamKw
                    | SyntaxKind::ChannelKw
                    | SyntaxKind::FnKw
                    | SyntaxKind::PatternKw
            )
        )
    }

    fn line_stack_snapshot(&mut self) -> usize {
        if let Some(snapshot) = self.line_stack {
            return snapshot;
        }
        let snapshot = self.stacks.len();
        self.stacks.push(self.stack.clone());
        self.stored_entries += self.stack.len();
        self.peak_stored_entries = self.peak_stored_entries.max(self.stored_entries);
        self.line_stack = Some(snapshot);
        snapshot
    }

    fn top(&self) -> u32 {
        self.stack.last().copied().unwrap_or(0)
    }

    fn char_len(&self, at: usize) -> usize {
        self.text[at..].chars().next().map_or(0, char::len_utf8)
    }

    fn error(&mut self, code: Code, start: usize, end: usize, message: impl Into<String>) {
        self.raw(code, start, end, message, None);
    }

    fn raw(
        &mut self,
        code: Code,
        start: usize,
        end: usize,
        summary: impl Into<String>,
        help: Option<String>,
    ) {
        self.diagnostics.push(RawDiagnostic {
            code,
            range: start as u32..end as u32,
            summary: summary.into(),
            help,
        });
    }

    fn trivial(&mut self, index: usize) {
        if let Some(kind) = self.columns.kinds.get_mut(index) {
            *kind = SyntaxKind::Whitespace;
        }
        if let Ok(at) = self.columns.sig.binary_search(&(index as u32)) {
            self.columns.sig.remove(at);
        }
    }
}

fn word(text: &str) -> &str {
    let mut chars = text.char_indices();
    match chars.next() {
        Some((_, c)) if is_ident_start(c) => {}
        _ => return "",
    }
    let end = chars
        .find(|(_, c)| !is_ident_continue(*c))
        .map_or(text.len(), |(offset, _)| offset);
    CountedWalk::LexByte.step_by(end);
    text.get(..end).unwrap_or_default()
}
