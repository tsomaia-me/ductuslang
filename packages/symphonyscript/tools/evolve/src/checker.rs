use std::sync::Arc;
use std::time::Duration;

use symphonyscript::syntax::parse;
use symphonyscript::{Code, Compiler, Diagnostic, Failed};
use symphonyscript_core::Vocabulary;

use crate::differing_codes::differing_codes;
use crate::finding::Finding;
use crate::guarded::{Guarded, guarded};
use crate::measured::Measured;
use crate::observation::{Observation, observed};
use crate::over_memory_bound::{DEFAULT_MEMORY_BOUND, over_memory_bound};
use crate::program::Program;
use crate::renamed::{renamed, restored};
use crate::scaled::scaled;
use crate::signal::Signal;
use crate::superlinear::{SCALING_FLOOR, superlinear};
use crate::variants::{reordered, with_noise};
use crate::verdict::{DiagnosticRecord, Verdict};
use crate::work::Work;

pub const DEFAULT_BUDGET: Duration = Duration::from_secs(5);
const DOUBLED: usize = 2;
const QUADRUPLED: usize = 4;

pub struct Checker {
    compiler: Arc<Compiler>,
    vocabulary: Vocabulary,
    budget: Duration,
    memory_bound: u64,
}

impl Checker {
    pub fn new(budget: Duration) -> Result<Checker, Failed> {
        Checker::with_vocabulary(budget, symphonyscript_fixtures::vocabulary())
    }

    pub fn with_vocabulary(budget: Duration, vocabulary: Vocabulary) -> Result<Checker, Failed> {
        let compiler = Compiler::new(&vocabulary)?;
        Ok(Checker {
            compiler: Arc::new(compiler),
            vocabulary,
            budget,
            memory_bound: DEFAULT_MEMORY_BOUND,
        })
    }

    pub fn with_memory_bound(self, bytes_per_source_byte: u64) -> Checker {
        Checker {
            memory_bound: bytes_per_source_byte,
            ..self
        }
    }

    pub fn vocabulary(&self) -> &Vocabulary {
        &self.vocabulary
    }

    pub fn compile(&self, program: &Program) -> Guarded<Vec<Diagnostic>> {
        let compiler = Arc::clone(&self.compiler);
        let sources = program.compiler_sources();
        guarded(self.budget, move || compiler.check(&sources))
    }

    pub fn parses(&self, program: &Program) -> bool {
        let vocabulary = self.vocabulary.clone();
        let sources = program.compiler_sources();
        matches!(
            guarded(self.budget, move || {
                sources
                    .iter()
                    .all(|source| parse(source, &vocabulary).diagnostics.is_empty())
            }),
            Guarded::Finished { value: true, .. }
        )
    }

    pub fn verdict(&self, program: &Program) -> Verdict {
        let compiled = self.compile(program);
        let mut verdict = Verdict {
            id: program.id.clone(),
            parses: self.parses(program),
            diagnostics: Vec::new(),
            signals: Vec::new(),
            findings: Vec::new(),
            panic: None,
            elapsed_ms: None,
            allocations: None,
            peak_bytes: None,
        };
        match &compiled {
            Guarded::Finished {
                value,
                elapsed,
                work,
            } => {
                verdict.elapsed_ms = Some(elapsed.as_secs_f64() * 1000.0);
                verdict.allocations = Some(work.allocations);
                verdict.peak_bytes = Some(work.peak_bytes);
                verdict.diagnostics = value.iter().map(DiagnosticRecord::from).collect();
            }
            Guarded::Panicked { message } => verdict.panic = Some(message.clone()),
            Guarded::TimedOut => {}
        }
        for signal in Signal::ALL {
            if let Some(codes) = self.causes(program, &compiled, signal).into_iter().next() {
                verdict.signals.push(signal);
                verdict.findings.push(Finding { signal, codes });
            }
        }
        verdict
    }

    pub fn finding(&self, program: &Program, signal: Signal) -> Option<Finding> {
        let compiled = self.compile(program);
        self.causes(program, &compiled, signal)
            .into_iter()
            .next()
            .map(|codes| Finding { signal, codes })
    }

    pub fn reproduces(&self, program: &Program, finding: &Finding) -> bool {
        let compiled = self.compile(program);
        self.causes(program, &compiled, finding.signal)
            .contains(&finding.codes)
    }

    fn causes(
        &self,
        program: &Program,
        compiled: &Guarded<Vec<Diagnostic>>,
        signal: Signal,
    ) -> Vec<Vec<String>> {
        let (diagnostics, work) = match (signal, compiled) {
            (Signal::Crash, Guarded::Panicked { message }) => return vec![vec![message.clone()]],
            (Signal::Slow, Guarded::TimedOut) => return vec![Vec::new()],
            (_, Guarded::Finished { value, work, .. }) => (value, *work),
            _ => return Vec::new(),
        };
        let codes = sorted_codes(diagnostics);
        let mut unique = codes.clone();
        unique.dedup();
        match signal {
            Signal::Crash => diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == Code::InternalError)
                .then(|| vec![Code::InternalError.as_str().to_string()])
                .into_iter()
                .collect(),
            Signal::Slow => Vec::new(),
            Signal::Order => reordered(program, &self.vocabulary)
                .iter()
                .filter_map(|variant| {
                    let variant_codes = match self.compile(variant) {
                        Guarded::Finished { value, .. } => sorted_codes(&value),
                        Guarded::Panicked { message } => vec![message],
                        Guarded::TimedOut => return None,
                    };
                    let differing = differing_codes(&codes, &variant_codes);
                    (!differing.is_empty()).then_some(differing)
                })
                .collect(),
            Signal::Rename => {
                let Some(renamed) = renamed(program, &self.vocabulary) else {
                    return Vec::new();
                };
                let baseline = observed(diagnostics, program, str::to_string);
                let differs = self.differs(&baseline, &renamed.program, |text| {
                    restored(text, &renamed.originals)
                });
                differs.then_some(unique).into_iter().collect()
            }
            Signal::Noise => {
                let baseline = observed(diagnostics, program, str::to_string);
                let differs = self.differs(&baseline, &with_noise(program), str::to_string);
                differs.then_some(unique).into_iter().collect()
            }
            Signal::Scaling => self
                .scales_superlinearly(program, &unique, work)
                .then_some(unique)
                .into_iter()
                .collect(),
            Signal::Memory => over_memory_bound(work.peak_bytes, program.size(), self.memory_bound)
                .then_some(unique)
                .into_iter()
                .collect(),
        }
    }

    fn differs(
        &self,
        baseline: &Observation,
        variant: &Program,
        restore: impl Fn(&str) -> String,
    ) -> bool {
        let observation = match self.compile(variant) {
            Guarded::Finished { value, .. } => observed(&value, variant, restore),
            Guarded::Panicked { message } => Observation::Panicked(message),
            Guarded::TimedOut => return false,
        };
        observation != *baseline
    }

    fn scales_superlinearly(&self, program: &Program, unique: &[String], work: Work) -> bool {
        if work.allocations < SCALING_FLOOR {
            return false;
        }
        let once = Measured {
            distinct_codes: unique.to_vec(),
            work,
        };
        let (Some(doubled), Some(quadrupled)) = (
            self.measured_scaled(program, DOUBLED),
            self.measured_scaled(program, QUADRUPLED),
        ) else {
            return false;
        };
        superlinear(&once, &doubled, &quadrupled)
    }

    fn measured_scaled(&self, program: &Program, factor: usize) -> Option<Measured> {
        let Guarded::Finished { value, work, .. } =
            self.compile(&scaled(program, &self.vocabulary, factor))
        else {
            return None;
        };
        let mut distinct_codes = sorted_codes(&value);
        distinct_codes.dedup();
        Some(Measured {
            distinct_codes,
            work,
        })
    }
}

fn sorted_codes(diagnostics: &[Diagnostic]) -> Vec<String> {
    let mut codes: Vec<String> = diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code.as_str().to_string())
        .collect();
    codes.sort();
    codes
}
