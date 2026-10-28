use std::collections::BTreeSet;
use std::path::Path;

use crate::spec_example::SpecExample;
use crate::spec_examples::spec_examples;

pub const PENDING_EXAMPLES: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../docs/pending-examples.txt"
);

pub fn pending_key(document: &str, example: &SpecExample) -> String {
    format!(
        "{document} §{} {} {}",
        example.section, example.ordinal, example.tag
    )
}

pub fn pending_lines(list: &Path) -> BTreeSet<String> {
    std::fs::read_to_string(list)
        .unwrap_or_else(|error| panic!("{} reads: {error}", list.display()))
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(String::from)
        .collect()
}

pub fn pending_spec_examples() -> BTreeSet<String> {
    let lines = pending_lines(Path::new(PENDING_EXAMPLES));
    spec_examples()
        .into_iter()
        .filter(|example| lines.contains(&pending_key("SPEC.md", example)))
        .map(|example| example.name())
        .collect()
}
