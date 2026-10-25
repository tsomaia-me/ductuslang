use std::sync::Arc;
use std::time::Duration;

use symphonyscript::{Code, Compiler, Diagnostic, Failed};
use symphonyscript_core::Vocabulary;

use crate::guarded::{Guarded, guarded};
use crate::observation::{Observation, observed};
use crate::program::Program;
use crate::renamed::{renamed, restored};
use crate::signal::Signal;
use crate::variants::{reordered, with_noise};
use crate::verdict::{DiagnosticRecord, Verdict};

pub const DEFAULT_BUDGET: Duration = Duration::from_secs(5);

pub struct Checker {
    compiler: Arc<Compiler>,
    vocabulary: Vocabulary,
    budget: Duration,
}

impl Checker {
    pub fn new(budget: Duration) -> Result<Checker, Failed> {
        let vocabulary = symphonyscript_fixtures::vocabulary();
        let compiler = Compiler::new(&vocabulary)?;
        Ok(Checker {
            compiler: Arc::new(compiler),
            vocabulary,
            budget,
        })
    }

    pub fn vocabulary(&self) -> &Vocabulary {
        &self.vocabulary
    }

    pub fn compile(&self, program: &Program) -> Guarded<Vec<Diagnostic>> {
        let compiler = Arc::clone(&self.compiler);
        let sources = program.compiler_sources();
        guarded(self.budget, move || compiler.check(&sources))
    }

    pub fn verdict(&self, program: &Program) -> Verdict {
        let compiled = self.compile(program);
        let mut verdict = Verdict {
            id: program.id.clone(),
            diagnostics: Vec::new(),
            signals: Vec::new(),
            panic: None,
            elapsed_ms: None,
        };
        let diagnostics = match compiled {
            Guarded::Finished { value, elapsed } => {
                verdict.elapsed_ms = Some(elapsed.as_secs_f64() * 1000.0);
                value
            }
            Guarded::Panicked { message } => {
                verdict.signals.push(Signal::Crash);
                verdict.panic = Some(message);
                return verdict;
            }
            Guarded::TimedOut => {
                verdict.signals.push(Signal::Slow);
                return verdict;
            }
        };
        verdict.diagnostics = diagnostics.iter().map(DiagnosticRecord::from).collect();
        if has_internal_error(&diagnostics) {
            verdict.signals.push(Signal::Crash);
        }
        let baseline = observed(&diagnostics, program, str::to_string);
        for signal in [Signal::Order, Signal::Rename, Signal::Noise] {
            if self.varies(program, &baseline, signal) {
                verdict.signals.push(signal);
            }
        }
        verdict
    }

    pub fn holds(&self, program: &Program, signal: Signal) -> bool {
        let compiled = self.compile(program);
        match (signal, compiled) {
            (Signal::Crash, Guarded::Panicked { .. }) => true,
            (Signal::Crash, Guarded::Finished { value, .. }) => has_internal_error(&value),
            (Signal::Slow, Guarded::TimedOut) => true,
            (Signal::Order | Signal::Rename | Signal::Noise, Guarded::Finished { value, .. }) => {
                let baseline = observed(&value, program, str::to_string);
                self.varies(program, &baseline, signal)
            }
            _ => false,
        }
    }

    pub fn codes(&self, program: &Program) -> Vec<String> {
        let mut codes: Vec<String> = match self.compile(program) {
            Guarded::Finished { value, .. } => value
                .iter()
                .map(|diagnostic| diagnostic.code.as_str().to_string())
                .collect(),
            Guarded::Panicked { .. } | Guarded::TimedOut => Vec::new(),
        };
        codes.sort();
        codes.dedup();
        codes
    }

    fn varies(&self, program: &Program, baseline: &Observation, signal: Signal) -> bool {
        match signal {
            Signal::Order => reordered(program, &self.vocabulary)
                .iter()
                .any(|variant| self.differs(baseline, variant, str::to_string)),
            Signal::Rename => {
                let renamed = renamed(program, &self.vocabulary);
                self.differs(baseline, &renamed.program, |text| {
                    restored(text, &renamed.originals)
                })
            }
            Signal::Noise => self.differs(baseline, &with_noise(program), str::to_string),
            Signal::Crash | Signal::Slow => false,
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
}

fn has_internal_error(diagnostics: &[Diagnostic]) -> bool {
    diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == Code::InternalError)
}
