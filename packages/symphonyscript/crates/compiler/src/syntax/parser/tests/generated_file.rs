use std::ops::Range;

use symphonyscript_core::Vocabulary;

use super::fault_catalogue::{BracketKind, FaultItem, Open, unmark};
use super::line_shape::LineShape;
use super::valid_items::{HOST, ValidItems};
use crate::diagnostics::Code;

const DEEPER: usize = 4;

const FAULTY_HOSTS: &[(Code, &str, usize)] = &[
    (Code::PlacementSyntax, "Clip host ⟦2⟧:\n", 4),
    (Code::PlacementSyntax, "Clip host | gain=1 ⟦2⟧:\n", 4),
    (Code::PlacementSyntax, "Clip host when x ⟦y⟧:\n", 4),
    (Code::ReservedToken, "Clip host | gain=1 ⟦^⟧:\n", 4),
    (Code::PatternSyntax, "pattern P(a ⟦b⟧):\n", 4),
    (Code::ForSyntax, "Clip host:\n    for i in 0..3 ⟦4⟧:\n", 8),
    (Code::WhenSyntax, "Clip host:\n    when x ⟦y⟧:\n", 8),
];

const MATCH_HOSTS: &[(Code, &str, usize)] = &[
    (Code::WireSyntax, "    Plays ⟦2⟧: match x:\n", 0),
    (Code::LetSyntax, "let a ⟦2⟧ = match x:\n", 4),
];

const VALID_ARMS: &[&str] = &[
    "        0: verse\n",
    "        Sine: chorus\n",
    "        -1: intro\n",
];

const FAULTY_ARMS: &[(Code, &str)] = &[
    (Code::ExpressionSyntax, "        0: ⟦)⟧\n"),
    (Code::MatchSyntax, "        ⟦+⟧: verse\n"),
    (Code::MatchSyntax, "        0 ⟦verse⟧\n"),
    (Code::ExpressionSyntax, "        else: ⟦|⟧\n"),
];

const ELSE_ARM: &str = "        else: verse\n";

const MISSING_BLOCK_HOST: &str = "Clip lacking:\n";

fn match_hosts(vocabulary: &Vocabulary) -> Vec<Candidate> {
    MATCH_HOSTS
        .iter()
        .flat_map(|(head_code, head, shallower)| {
            let arm = move |code: Code, marked: &'static str| (code, &marked[*shallower..]);
            let faulty = FAULTY_ARMS.iter().map(move |(code, marked)| {
                let mut pieces = vec![
                    (*head_code, *head),
                    arm(*code, VALID_ARMS[0]),
                    arm(*code, marked),
                ];
                if !marked.trim_start().starts_with("else") {
                    pieces.push(arm(*code, ELSE_ARM));
                }
                (format!("match host {head:?} with {code}"), pieces)
            });
            let valid = VALID_ARMS
                .iter()
                .chain([&ELSE_ARM])
                .map(move |marked| arm(*head_code, marked));
            let all_valid = (
                format!("match host {head:?}"),
                std::iter::once((*head_code, *head)).chain(valid).collect(),
            );
            faulty.chain([all_valid])
        })
        .map(|(label, pieces): (String, Vec<(Code, &str)>)| {
            let mut text = String::new();
            let mut faults = Vec::new();
            for (code, marked) in pieces {
                let (piece, piece_faults) = unmark(code, marked);
                let offset = text.len() as u32;
                faults.extend(
                    piece_faults
                        .into_iter()
                        .map(|(code, range)| (code, offset + range.start..offset + range.end)),
                );
                text.push_str(&piece);
            }
            Candidate::new(label, &text, faults, Open::Closed, vocabulary)
        })
        .collect()
}

const REGION_LINES: &[&str] = &[
    "a/b c",
    "x/2,",
    "a /2",
    "Arp /48 | steps=3",
    "a/x::y::z",
    "a/to",
    "a/to.gain.x",
    "a/b.c::d",
    "a/2 b",
    "x/(1) y",
    "a/x::y",
    "a/(1)",
    "a/-3 | x",
    "1 + 2",
    "a.b",
    "a,",
    "- b",
    "a = 1",
    "\"s\",",
    "f(1),",
    "Note",
    "Note Note",
    "(Note)",
    "let x = 1",
    "when x: Note",
];

pub(super) struct Candidate {
    label: String,
    text: String,
    body: bool,
    in_pattern: bool,
    shape: LineShape,
    faults: Vec<(Code, Range<u32>)>,
    open: Open,
    deeper: Option<Box<Candidate>>,
}

impl Candidate {
    fn new(
        label: String,
        text: &str,
        faults: Vec<(Code, Range<u32>)>,
        open: Open,
        vocabulary: &Vocabulary,
    ) -> Candidate {
        let body = text.starts_with([' ', '\t']);
        let mut candidate = Candidate {
            label,
            text: text.to_string(),
            body,
            in_pattern: !text.trim_start().starts_with("let "),
            shape: LineShape::of(text, vocabulary),
            faults,
            open,
            deeper: None,
        };
        if body {
            candidate.deeper = Some(Box::new(candidate.deepened(vocabulary)));
        }
        candidate
    }

    fn deepened(&self, vocabulary: &Vocabulary) -> Candidate {
        let mut text = String::new();
        let mut starts = Vec::new();
        for line in self.text.split_inclusive('\n') {
            if !line.trim().is_empty() {
                starts.push(text.len() - starts.len() * DEEPER);
                text.push_str(&" ".repeat(DEEPER));
            }
            text.push_str(line);
        }
        let shift = |offset: u32, end: bool| {
            let before = starts.partition_point(|start| {
                let start = *start as u32;
                start < offset || (start == offset && !end)
            });
            offset + (before * DEEPER) as u32
        };
        Candidate {
            label: format!("{} (deeper)", self.label),
            shape: LineShape::of(&text, vocabulary),
            text,
            body: true,
            in_pattern: self.in_pattern,
            faults: self
                .faults
                .iter()
                .map(|(code, range)| (*code, shift(range.start, false)..shift(range.end, true)))
                .collect(),
            open: self.open,
            deeper: None,
        }
    }

    fn column(&self) -> usize {
        self.text.len() - self.text.trim_start_matches([' ', '\t']).len()
    }

    fn follows(&self, pending: Option<Open>) -> bool {
        match pending {
            None | Some(Open::Closed) => true,
            Some(Open::AfterFault) => self.shape.resynchronizes(!self.body),
            Some(Open::Unclosed { bracket, .. }) => {
                self.faults.is_empty()
                    && self.open == Open::Closed
                    && self.shape.ends_unclosed(bracket)
            }
        }
    }
}

struct Host {
    candidate: Candidate,
    column: usize,
}

impl Host {
    fn takes(&self, item: &Candidate) -> bool {
        item.in_pattern || !self.candidate.text.starts_with("pattern")
    }
}

pub(super) struct Pool {
    region_lines: Vec<(String, LineShape)>,
    faults: Vec<Candidate>,
    top: Vec<Candidate>,
    body: Vec<Candidate>,
    hosts: Vec<Host>,
    missing_block: Candidate,
}

impl Pool {
    pub(super) fn new(faults: &[FaultItem], valid: &ValidItems, vocabulary: &Vocabulary) -> Pool {
        let valid_candidate = |text: &String| {
            Candidate::new(
                String::from("valid"),
                text,
                Vec::new(),
                Open::Closed,
                vocabulary,
            )
        };
        let plain = Host {
            candidate: Candidate::new(
                String::from("host"),
                HOST,
                Vec::new(),
                Open::Closed,
                vocabulary,
            ),
            column: 4,
        };
        let faulty = FAULTY_HOSTS.iter().map(|(code, marked, column)| {
            let (text, faults) = unmark(*code, marked);
            Host {
                candidate: Candidate::new(
                    format!("host with {code}"),
                    &text,
                    faults,
                    Open::Closed,
                    vocabulary,
                ),
                column: *column,
            }
        });
        Pool {
            region_lines: REGION_LINES
                .iter()
                .map(|line| (format!("{line}\n"), LineShape::of(line, vocabulary)))
                .collect(),
            faults: faults
                .iter()
                .map(|item| {
                    let label = format!("{} in {:?}, {:?}", item.code, item.context, item.lines);
                    Candidate::new(
                        label,
                        &item.text,
                        item.faults.clone(),
                        item.open,
                        vocabulary,
                    )
                })
                .chain(match_hosts(vocabulary))
                .collect(),
            top: valid.top.iter().map(valid_candidate).collect(),
            body: valid.body.iter().map(valid_candidate).collect(),
            hosts: std::iter::once(plain).chain(faulty).collect(),
            missing_block: Candidate::new(
                String::from("missing block"),
                MISSING_BLOCK_HOST,
                Vec::new(),
                Open::Closed,
                vocabulary,
            ),
        }
    }
}

pub(super) struct Generated {
    pub(super) text: String,
    pub(super) expected: Vec<(Code, Range<u32>)>,
    pub(super) closers: Vec<(Range<u32>, String)>,
    pub(super) items: Vec<(u32, String)>,
}

struct Random(u64);

impl Random {
    fn below(&mut self, bound: usize) -> usize {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^= z >> 31;
        (z % bound as u64) as usize
    }
}

struct Writer<'p> {
    pool: &'p Pool,
    random: Random,
    out: Generated,
    host: Option<&'p Host>,
    pending: Option<Open>,
}

impl<'p> Writer<'p> {
    fn emit(&mut self, candidate: &Candidate) {
        let offset = self.out.text.len() as u32;
        if let Some(Open::Unclosed { closer, .. }) = self.pending {
            let first = &candidate.shape.first_token;
            let span = offset + first.start..offset + first.end;
            let token = candidate
                .text
                .get(first.start as usize..first.end as usize)
                .unwrap_or_default();
            self.out
                .closers
                .push((span.clone(), format!("found `{token}`")));
            self.out.expected.push((closer, span));
        }
        for (code, range) in &candidate.faults {
            self.out
                .expected
                .push((*code, offset + range.start..offset + range.end));
        }
        self.out.items.push((offset, candidate.label.clone()));
        self.out.text.push_str(&candidate.text);
        self.pending = Some(candidate.open);
    }

    fn in_host(&self, item: &'p Candidate) -> Option<&'p Candidate> {
        let host = self.host?;
        if !host.takes(item) {
            return None;
        }
        if host.column > 4 {
            item.deeper.as_deref()
        } else {
            Some(item)
        }
    }

    fn open_host(&mut self, item: &Candidate) -> bool {
        let pool = self.pool;
        let pending = self.pending;
        let fitting: Vec<&'p Host> = pool
            .hosts
            .iter()
            .filter(|host| host.takes(item) && host.candidate.follows(pending))
            .collect();
        if fitting.is_empty() {
            return false;
        }
        let host = fitting[self.random.below(fitting.len())];
        self.emit(&host.candidate);
        self.host = Some(host);
        true
    }

    fn place(&mut self, candidate: &'p Candidate) {
        loop {
            if !candidate.body {
                self.host = None;
            } else if self.in_host(candidate).is_none() {
                self.host = None;
                if !self.open_host(candidate) {
                    self.follower(false);
                    continue;
                }
            }
            let target = if candidate.body {
                self.in_host(candidate).unwrap_or(candidate)
            } else {
                candidate
            };
            if target.follows(self.pending) {
                self.emit(target);
                if target.open == Open::AfterFault {
                    self.region_lines(target.column());
                }
                return;
            }
            self.follower(candidate.body);
        }
    }

    fn after_missing_block(&mut self, fault: &'p Candidate) {
        let pool = self.pool;
        while !pool.missing_block.follows(self.pending) {
            self.follower(false);
        }
        self.host = None;
        self.emit(&pool.missing_block);
        let offset = self.out.text.len() as u32;
        let first = &fault.shape.first_token;
        self.out.expected.push((
            Code::PlacementSyntax,
            offset + first.start..offset + first.end,
        ));
        self.emit(fault);
        if fault.open == Open::AfterFault {
            self.region_lines(fault.column());
        }
    }

    fn region_lines(&mut self, column: usize) {
        let pool = self.pool;
        for _ in 0..self.random.below(3) {
            let deeper = self.random.below(2) == 0;
            let fitting: Vec<&String> = pool
                .region_lines
                .iter()
                .filter(|(_, shape)| {
                    deeper
                        || !(shape.resynchronizes(column == 0)
                            || shape.ends_unclosed(BracketKind::Expression)
                            || shape.ends_unclosed(BracketKind::Bundle))
                })
                .map(|(line, _)| line)
                .collect();
            let line = fitting[self.random.below(fitting.len())];
            let indentation = column + if deeper { 2 } else { 0 };
            self.out
                .items
                .push((self.out.text.len() as u32, String::from("region line")));
            self.out.text.push_str(&" ".repeat(indentation));
            self.out.text.push_str(line);
        }
    }

    fn follower(&mut self, body: bool) {
        let pool = self.pool;
        let pending = self.pending;
        let mut choices: Vec<&'p Candidate> = if body {
            pool.body
                .iter()
                .filter_map(|candidate| self.in_host(candidate))
                .filter(|candidate| candidate.follows(pending))
                .collect()
        } else {
            Vec::new()
        };
        if choices.is_empty() {
            self.host = None;
            choices = pool
                .top
                .iter()
                .filter(|candidate| candidate.follows(pending))
                .collect();
        }
        let choice = choices[self.random.below(choices.len())];
        self.emit(choice);
    }

    fn valid(&mut self) {
        let pool = self.pool;
        let list = if self.random.below(2) == 0 {
            &pool.top
        } else {
            &pool.body
        };
        let choice = &list[self.random.below(list.len())];
        self.place(choice);
    }
}

pub(super) fn generate(seed: u64, fault_count: usize, pool: &Pool) -> Generated {
    let mut writer = Writer {
        pool,
        random: Random(seed),
        out: Generated {
            text: String::new(),
            expected: Vec::new(),
            closers: Vec::new(),
            items: Vec::new(),
        },
        host: None,
        pending: None,
    };
    let mut order: Vec<usize> = Vec::new();
    for _ in 0..fault_count {
        if order.is_empty() {
            order = (0..pool.faults.len()).collect();
            for index in (1..order.len()).rev() {
                let other = writer.random.below(index + 1);
                order.swap(index, other);
            }
        }
        for _ in 0..writer.random.below(3) {
            writer.valid();
        }
        let fault = &pool.faults[order.pop().expect("the order is refilled when empty")];
        if !fault.body && writer.random.below(4) == 0 {
            writer.after_missing_block(fault);
        } else {
            writer.place(fault);
        }
    }
    if !matches!(writer.pending, None | Some(Open::Closed)) {
        writer.follower(false);
    }
    writer.out
}
