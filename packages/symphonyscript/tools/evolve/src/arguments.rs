use std::time::Duration;

use crate::checker::DEFAULT_BUDGET;
use crate::named_vocabulary::VOCABULARY_NAMES;
use crate::over_memory_bound::DEFAULT_MEMORY_BOUND;
use crate::restarted::RESUME_AT;

#[derive(Clone, PartialEq, Debug)]
pub struct Arguments {
    pub budget: Duration,
    pub memory_bound: u64,
    pub vocabulary: String,
    pub resume_at: usize,
    pub positional: Vec<String>,
}

pub fn arguments(raw: impl IntoIterator<Item = String>) -> Result<Arguments, String> {
    let mut budget = DEFAULT_BUDGET;
    let mut memory_bound = DEFAULT_MEMORY_BOUND;
    let mut vocabulary = VOCABULARY_NAMES[0].to_string();
    let mut resume_at = 0;
    let mut positional = Vec::new();
    let mut raw = raw.into_iter();
    while let Some(argument) = raw.next() {
        match argument.as_str() {
            "--budget" => {
                let value = raw
                    .next()
                    .ok_or_else(|| "--budget takes a number of seconds".to_string())?;
                let seconds: f64 = value
                    .parse()
                    .map_err(|_| format!("--budget takes a number of seconds, not `{value}`"))?;
                budget = Duration::try_from_secs_f64(seconds).map_err(|_| {
                    format!("--budget takes a non-negative number of seconds, not `{value}`")
                })?;
            }
            "--memory-bound" => {
                let value = raw.next().ok_or_else(|| {
                    "--memory-bound takes a number of bytes per source byte".to_string()
                })?;
                memory_bound = value.parse().map_err(|_| {
                    format!("--memory-bound takes a number of bytes per source byte, not `{value}`")
                })?;
            }
            "--vocabulary" => {
                vocabulary = raw
                    .next()
                    .ok_or_else(|| "--vocabulary takes a vocabulary's name".to_string())?;
            }
            RESUME_AT => {
                let value = raw
                    .next()
                    .ok_or_else(|| format!("{RESUME_AT} takes a line index"))?;
                resume_at = value
                    .parse()
                    .map_err(|_| format!("{RESUME_AT} takes a line index, not `{value}`"))?;
            }
            _ => positional.push(argument),
        }
    }
    Ok(Arguments {
        budget,
        memory_bound,
        vocabulary,
        resume_at,
        positional,
    })
}
