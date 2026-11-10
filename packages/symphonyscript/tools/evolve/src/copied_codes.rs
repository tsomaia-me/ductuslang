use symphonyscript::Diagnostic;

use crate::program::Program;

pub fn copied_codes(program: &Program, original: &[Diagnostic], factor: usize) -> Vec<String> {
    let first = program
        .ordered_sources()
        .first()
        .map(|source| source.path.clone());
    let mut codes: Vec<String> = original
        .iter()
        .flat_map(|diagnostic| {
            let whole = diagnostic.span.range.is_empty()
                && diagnostic.span.range.start == 0
                && first.as_ref() == Some(&diagnostic.span.file);
            let times = if whole { 1 } else { factor };
            std::iter::repeat_n(diagnostic.code.as_str().to_string(), times)
        })
        .collect();
    codes.sort();
    codes
}
