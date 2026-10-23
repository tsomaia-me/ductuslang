//! The `path` function: a plain dotted `CellPath` from its segment names.

use ductus_ir::path::{CellPath, Segment};

/// A plain dotted path — `path(&["App", "x"])` is `App.x`.
///
/// The only path shape the implemented slice admits (D7): every segment is a
/// bare name, none carries an ordinal, and no key ever appears. A witness that
/// needed a richer path would be asserting something about paths rather than
/// about the commit cycle, and would build it itself.
#[must_use]
pub fn path(names: &[&str]) -> CellPath {
    CellPath::new(
        names
            .iter()
            .map(|text| Segment::Name {
                text: (*text).to_string(),
                ordinal: None,
            })
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_the_dotted_spelling_its_names_give() {
        assert_eq!(path(&["App", "x"]).render(), "App.x");
    }

    #[test]
    fn carries_no_ordinal_on_any_segment() {
        let built = path(&["App", "inner", "x"]);

        assert!(
            built
                .segments()
                .iter()
                .all(|segment| matches!(segment, Segment::Name { ordinal: None, .. }))
        );
    }
}
