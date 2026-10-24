use std::ops::Range;

use symphonyscript_core::Vocabulary;

use super::fault_catalogue::{FaultItem, Open};
use super::line_shape::LineShape;
use super::valid_items::{HOST, ValidItems};
use crate::diagnostics::Code;

pub(super) struct Candidate {
    label: String,
    text: String,
    body: bool,
    shape: LineShape,
    faults: Vec<(Code, Range<u32>)>,
    open: Open,
}

impl Candidate {
    fn new(
        label: String,
        text: &str,
        faults: Vec<(Code, Range<u32>)>,
        open: Open,
        vocabulary: &Vocabulary,
    ) -> Candidate {
        Candidate {
            label,
            text: text.to_string(),
            body: text.starts_with([' ', '\t']),
            shape: LineShape::of(text, vocabulary),
            faults,
            open,
        }
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

pub(super) struct Pool {
    faults: Vec<Candidate>,
    top: Vec<Candidate>,
    body: Vec<Candidate>,
    host: Candidate,
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
        Pool {
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
                .collect(),
            top: valid.top.iter().map(valid_candidate).collect(),
            body: valid.body.iter().map(valid_candidate).collect(),
            host: Candidate::new(
                String::from("host"),
                HOST,
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
    in_host: bool,
    pending: Option<Open>,
}

impl<'p> Writer<'p> {
    fn emit(&mut self, candidate: &Candidate) {
        let offset = self.out.text.len() as u32;
        if let Some(Open::Unclosed { closer, .. }) = self.pending {
            let first = &candidate.shape.first_token;
            self.out
                .expected
                .push((closer, offset + first.start..offset + first.end));
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

    fn place(&mut self, candidate: &Candidate) {
        loop {
            if candidate.body && !self.in_host {
                if !self.pool.host.follows(self.pending) {
                    self.follower(false);
                    continue;
                }
                self.emit(&self.pool.host);
                self.in_host = true;
            }
            if !candidate.body {
                self.in_host = false;
            }
            if candidate.follows(self.pending) {
                self.emit(candidate);
                return;
            }
            self.follower(candidate.body);
        }
    }

    fn follower(&mut self, body: bool) {
        let pool = self.pool;
        let pending = self.pending;
        let fitting = |list: &'p [Candidate]| -> Vec<&'p Candidate> {
            list.iter()
                .filter(|candidate| candidate.follows(pending))
                .collect()
        };
        let mut choices = if body && self.in_host {
            fitting(&pool.body)
        } else {
            Vec::new()
        };
        if choices.is_empty() {
            self.in_host = false;
            choices = fitting(&pool.top);
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
            items: Vec::new(),
        },
        in_host: false,
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
        let fault = order.pop().expect("the order is refilled when empty");
        writer.place(&pool.faults[fault]);
    }
    if !matches!(writer.pending, None | Some(Open::Closed)) {
        writer.follower(false);
    }
    writer.out
}
