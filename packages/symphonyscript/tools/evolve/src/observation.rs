use std::collections::BTreeMap;

use symphonyscript::Diagnostic;

use crate::program::Program;

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Observation {
    Diagnostics(Vec<(String, String)>),
    Panicked(String),
}

pub fn observed(
    diagnostics: &[Diagnostic],
    program: &Program,
    restore: impl Fn(&str) -> String,
) -> Observation {
    let texts: BTreeMap<&str, &str> = program
        .sources
        .iter()
        .map(|source| (source.path.as_str(), source.text.as_str()))
        .collect();
    let mut seen: Vec<(String, String)> = diagnostics
        .iter()
        .map(|diagnostic| {
            let span = &diagnostic.span;
            let covered = texts
                .get(span.file.as_str())
                .and_then(|text| {
                    let start = usize::try_from(span.range.start).ok()?;
                    let end = usize::try_from(span.range.end).ok()?;
                    text.get(start..end)
                })
                .unwrap_or_default();
            (diagnostic.code.as_str().to_string(), restore(covered))
        })
        .collect();
    seen.sort();
    Observation::Diagnostics(seen)
}
