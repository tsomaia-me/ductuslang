use std::time::Duration;

use symphonyscript_core::LimitOverrides;

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
    pub limits: LimitOverrides,
    pub positional: Vec<String>,
}

pub fn arguments(raw: impl IntoIterator<Item = String>) -> Result<Arguments, String> {
    let mut budget = DEFAULT_BUDGET;
    let mut memory_bound = DEFAULT_MEMORY_BOUND;
    let mut vocabulary = VOCABULARY_NAMES[0].to_string();
    let mut resume_at = 0;
    let mut limits = LimitOverrides::default();
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
            "--limit" => {
                let value = raw
                    .next()
                    .ok_or_else(|| "--limit takes <name>=<value>".to_string())?;
                limits = overridden(limits, &value)?;
            }
            _ => positional.push(argument),
        }
    }
    Ok(Arguments {
        budget,
        memory_bound,
        vocabulary,
        resume_at,
        limits,
        positional,
    })
}

fn overridden(limits: LimitOverrides, setting: &str) -> Result<LimitOverrides, String> {
    let refused = || format!("--limit takes <name>=<value>, not `{setting}`");
    let (name, value) = setting.split_once('=').ok_or_else(refused)?;
    let value = Some(value.parse::<u64>().map_err(|_| refused())?);
    let mut limits = limits;
    let field = match name {
        "recursive_calls" => &mut limits.recursive_calls,
        "recursive_calls_warning" => &mut limits.recursive_calls_warning,
        "nesting_depth" => &mut limits.nesting_depth,
        "nesting_depth_warning" => &mut limits.nesting_depth_warning,
        "inlined_calls" => &mut limits.inlined_calls,
        "inlined_calls_warning" => &mut limits.inlined_calls_warning,
        "expansion_steps" => &mut limits.expansion_steps,
        "expansion_steps_warning" => &mut limits.expansion_steps_warning,
        "expanded_items" => &mut limits.expanded_items,
        "expanded_items_warning" => &mut limits.expanded_items_warning,
        "value_elements" => &mut limits.value_elements,
        "value_elements_warning" => &mut limits.value_elements_warning,
        "memory_bytes" => &mut limits.memory_bytes,
        "memory_bytes_warning" => &mut limits.memory_bytes_warning,
        "reload_edit_distance" => &mut limits.reload_edit_distance,
        "shared_body_threshold" => &mut limits.shared_body_threshold,
        _ => return Err(format!("--limit names no limit `{name}`")),
    };
    *field = value;
    Ok(limits)
}
