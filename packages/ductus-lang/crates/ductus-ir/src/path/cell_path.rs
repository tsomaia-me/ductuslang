//! The `CellPath` struct: a cell's identity as an ordered segment list.

use super::segment::Segment;

/// A cell's identity: its segments ordered from the root scope down to the cell.
///
/// The inner `Vec` is private on purpose. Immutability is enforced by the API —
/// build once, read segments, no mutating methods — never by the element type
/// (R6: plain `String`, no `Box<str>`, no interning in M0).
///
/// There is deliberately no `FromStr` and no parsing here (R29). Turning
/// `"app.mixer.gain"` back into a `CellPath` is the text layer's job.
///
/// # Invariants (R33)
///
/// A path has at least one segment, and a `Name` is non-empty and contains no
/// `.`. [`CellPath::new`] `debug_assert`s both. Real enforcement belongs to the
/// producers — the parser and the loader (F4) — because they are the ones
/// holding a span to blame; the constructor stays infallible.
///
/// # Equality and ordering
///
/// Equality, hashing and ordering are derived, so they are *structural*:
/// segment by segment, each compared as its own string. That is not the same as
/// comparing rendered text, and the structural form is the authority. Ordering
/// puts a prefix before its extensions (`app` < `app.gain`).
///
/// Name comparison is byte-wise, which makes it deterministic but not
/// alphabetical: `'Z'` (0x5A) sorts before `'a'` (0x61), so `app.Zone` precedes
/// `app.alpha`. Nothing here is locale-aware and nothing should become so —
/// determinism is the requirement, human sort order is not.
///
/// Derived `Ord` on [`Segment`] also follows *variant declaration order*. A
/// future variant declared before `Name` would silently reorder every existing
/// path. New variants go after `Name`, or `Ord` gets hand-written.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CellPath(Vec<Segment>);

impl CellPath {
    /// Builds a path from its segments, root first.
    ///
    /// Debug builds assert the invariants documented above; release builds
    /// trust the producer.
    pub fn new(segments: Vec<Segment>) -> CellPath {
        debug_assert!(!segments.is_empty(), "a cell path has at least one segment");

        for segment in &segments {
            match segment {
                Segment::Name(name) => {
                    debug_assert!(!name.is_empty(), "a path segment name is never empty");
                    debug_assert!(
                        !name.contains('.'),
                        "a path segment name never contains `.`: {name}"
                    );
                }
            }
        }

        CellPath(segments)
    }

    /// The path's segments, root first.
    pub fn segments(&self) -> &[Segment] {
        &self.0
    }

    /// The canonical rendering: segment names joined with `.`, e.g.
    /// `app.mixer.gain` (§15.4.1.1). Derived on every call — no rendered form
    /// is stored beside the structure, so the two cannot drift out of
    /// agreement.
    #[must_use]
    pub fn render(&self) -> String {
        let mut rendered = String::new();

        for (index, segment) in self.0.iter().enumerate() {
            if index > 0 {
                rendered.push('.');
            }

            // `match`, not `if let`: when the keyed and ordinal segment forms
            // land, their rendering must be decided here, and the compiler
            // should say so.
            match segment {
                Segment::Name(name) => rendered.push_str(name),
            }
        }

        rendered
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn path(names: &[&str]) -> CellPath {
        CellPath::new(
            names
                .iter()
                .map(|name| Segment::Name((*name).to_string()))
                .collect(),
        )
    }

    #[test]
    fn renders_segments_joined_by_dots() {
        assert_eq!(path(&["app", "mixer", "gain"]).render(), "app.mixer.gain");
    }

    #[test]
    fn renders_a_single_segment_path() {
        assert_eq!(path(&["app"]).render(), "app");
    }

    #[test]
    fn exposes_segments_root_first() {
        let cell = path(&["app", "gain"]);
        let expected = vec![
            Segment::Name("app".to_string()),
            Segment::Name("gain".to_string()),
        ];
        assert_eq!(cell.segments(), expected.as_slice());
    }

    #[test]
    fn orders_lexicographically_by_segment() {
        let mut paths = [
            path(&["app", "mixer"]),
            path(&["zone"]),
            path(&["app"]),
            path(&["app", "gain"]),
        ];
        paths.sort();
        let rendered: Vec<String> = paths.iter().map(CellPath::render).collect();
        assert_eq!(rendered, ["app", "app.gain", "app.mixer", "zone"]);
    }

    // Executable form of the ordering note: byte-wise, so uppercase sorts
    // before lowercase. Deterministic, not alphabetical.
    #[test]
    fn orders_names_byte_wise_not_alphabetically() {
        let mut paths = [path(&["app", "alpha"]), path(&["app", "Zone"])];
        paths.sort();
        let rendered: Vec<String> = paths.iter().map(CellPath::render).collect();
        assert_eq!(rendered, ["app.Zone", "app.alpha"]);
    }

    #[test]
    fn works_as_a_hash_map_key() {
        let mut cells = HashMap::new();
        cells.insert(path(&["app", "gain"]), 1);
        cells.insert(path(&["app", "mute"]), 2);

        // Equality is structural, so an independently built equal path finds
        // the same entry.
        assert_eq!(cells.get(&path(&["app", "gain"])), Some(&1));
        assert_eq!(cells.get(&path(&["app", "missing"])), None);
        assert_eq!(cells.len(), 2);
    }

    // The three invariant tests below exercise `debug_assert`, which compiles
    // out in release builds; `cargo test` builds in debug, and the cfg_attr
    // keeps a `--release` run honest instead of failing.
    #[test]
    #[should_panic(expected = "a cell path has at least one segment")]
    #[cfg_attr(not(debug_assertions), ignore = "debug_assert compiles out")]
    fn new_rejects_an_empty_path() {
        CellPath::new(Vec::new());
    }

    #[test]
    #[should_panic(expected = "a path segment name is never empty")]
    #[cfg_attr(not(debug_assertions), ignore = "debug_assert compiles out")]
    fn new_rejects_an_empty_name() {
        CellPath::new(vec![Segment::Name(String::new())]);
    }

    #[test]
    #[should_panic(expected = "a path segment name never contains")]
    #[cfg_attr(not(debug_assertions), ignore = "debug_assert compiles out")]
    fn new_rejects_a_dotted_name() {
        CellPath::new(vec![Segment::Name("app.gain".to_string())]);
    }
}
