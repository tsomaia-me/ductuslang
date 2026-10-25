use std::time::Duration;

use crate::checker::DEFAULT_BUDGET;

#[derive(Clone, PartialEq, Debug)]
pub struct Arguments {
    pub budget: Duration,
    pub positional: Vec<String>,
}

pub fn arguments(raw: impl IntoIterator<Item = String>) -> Result<Arguments, String> {
    let mut budget = DEFAULT_BUDGET;
    let mut positional = Vec::new();
    let mut raw = raw.into_iter();
    while let Some(argument) = raw.next() {
        if argument != "--budget" {
            positional.push(argument);
            continue;
        }
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
    Ok(Arguments { budget, positional })
}
