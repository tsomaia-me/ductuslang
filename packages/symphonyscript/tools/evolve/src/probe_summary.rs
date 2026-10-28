use std::fmt;

use crate::probe_block::drawn_codes;
use crate::probe_program::ProbeProgram;
use crate::probed::Probed;

#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub struct ProbeSummary {
    pub programs: usize,
    pub clean: usize,
    pub errors: usize,
    pub warnings: usize,
    pub panics: usize,
    pub timeouts: usize,
    pub ok: usize,
    pub differs: usize,
}

impl ProbeSummary {
    pub fn add(&mut self, program: &ProbeProgram, probed: &Probed, values: bool) {
        self.programs += 1;
        match probed {
            Probed::Finished { diagnostics, .. } => {
                if diagnostics.is_empty() {
                    self.clean += 1;
                }
                if diagnostics.iter().any(|diagnostic| diagnostic.error) {
                    self.errors += 1;
                }
                if diagnostics.iter().any(|diagnostic| !diagnostic.error) {
                    self.warnings += 1;
                }
            }
            Probed::Panicked { .. } => self.panics += 1,
            Probed::TimedOut => self.timeouts += 1,
        }
        if values && let Some(expected) = &program.expect {
            if *expected == drawn_codes(probed) {
                self.ok += 1;
            } else {
                self.differs += 1;
            }
        }
    }
}

impl fmt::Display for ProbeSummary {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "programs {}, clean {}, errors {}, warnings {}, panics {}, timeouts {}",
            self.programs, self.clean, self.errors, self.warnings, self.panics, self.timeouts
        )?;
        if self.ok + self.differs > 0 {
            write!(formatter, ", ok {}, differs {}", self.ok, self.differs)?;
        }
        Ok(())
    }
}
