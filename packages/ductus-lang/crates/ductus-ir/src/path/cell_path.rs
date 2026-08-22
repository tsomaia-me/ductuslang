//! The `CellPath` struct: a cell's identity as an ordered segment list.

use std::fmt::Write;

use super::segment::Segment;
use crate::text::{is_name_shaped, is_reserved_word, quote};

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
/// A path has at least one segment, and a [`Segment::Name`]'s text is
/// NAME-shaped and not one of the five RESERVED spellings (002-39) — the first
/// subsumes the older "non-empty, no `.`" pair, since a name that is empty or
/// dotted is not NAME-shaped, and the second is what keeps a rendered path from
/// reading back as a keyword. [`CellPath::new`] `debug_assert`s all three. A
/// [`Segment::Key`] has NO invariant: any string is a legal key, the empty
/// string and `null` included, because quoting covers whatever bare spelling
/// cannot (see [`render`](CellPath::render)). Real enforcement belongs to the
/// producers — the parser and the loader (F4) — because they are the ones
/// holding a span to blame; the constructor stays infallible.
///
/// # Equality and ordering
///
/// Equality, hashing and ordering are derived, so they are *structural*:
/// segment by segment, each compared as its own variant and payload. That is
/// not the same as comparing rendered text, and the structural form is the
/// authority. Ordering puts a prefix before its extensions (`app` < `app.gain`).
///
/// Name comparison is byte-wise, which makes it deterministic but not
/// alphabetical: `'Z'` (0x5A) sorts before `'a'` (0x61), so `app.Zone` precedes
/// `app.alpha`. Nothing here is locale-aware and nothing should become so —
/// determinism is the requirement, human sort order is not.
///
/// Derived `Ord` on [`Segment`] also follows *variant declaration order*, so an
/// ordinal is compared after its name and every key sorts after every name.
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
            // A `match`, not an `if let`: a new segment form must decide here
            // whether it carries an invariant, and the compiler should say so.
            match segment {
                Segment::Name { text, .. } => {
                    debug_assert!(
                        is_name_shaped(text),
                        "a path segment name is NAME-shaped: {text}"
                    );
                    debug_assert!(
                        !is_reserved_word(text),
                        "a path segment name is not a reserved spelling: {text}"
                    );
                }
                // Any string is a legal key — quoting covers what bare cannot,
                // the five reserved spellings included.
                Segment::Key(_) => {}
            }
        }

        CellPath(segments)
    }

    /// The path's segments, root first.
    pub fn segments(&self) -> &[Segment] {
        &self.0
    }

    /// The path's segments, root first — the CONSUMING accessor.
    ///
    /// [`segments`](CellPath::segments) lends the list; this one hands it over.
    /// A caller that has to REBUILD a path from a small edit — the parser's
    /// `lookback_entry`, which takes an ordinal back off the last segment — is
    /// otherwise forced to clone the whole list to change one field.
    ///
    /// Immutability is not weakened by it: the path is consumed, so nothing
    /// observes a mutated `CellPath`, and whatever the caller builds next goes
    /// back through [`new`](CellPath::new) and its invariant checks.
    pub fn into_segments(self) -> Vec<Segment> {
        self.0
    }

    /// The canonical rendering: segments joined with `.` (§15.4.1.1). Derived on
    /// every call — no rendered form is stored beside the structure, so the two
    /// cannot drift out of agreement.
    ///
    /// Each segment renders per the canonical law:
    ///
    /// - a name is its text, followed by `:N` when it carries an ordinal —
    ///   `app.mixer.gain`, `App.print:0.text`;
    /// - a key is BARE iff it is NAME-shaped AND not a reserved spelling, and
    ///   single-quoted otherwise, in the one canonical payload spelling — raw
    ///   wherever raw is legal, with the delimiting quote, the backslash, a
    ///   carriage return, and every control scalar other than newline and tab
    ///   escaped — `App.rows.row_1.render`, `App.rows.'42'.render`,
    ///   `App.rows.'null'.render`.
    ///
    /// Quoting is canonical rather than optional (R41): bare is written
    /// wherever bare is available, so a key has exactly one spelling here even
    /// though the grammar would parse both.
    #[must_use]
    pub fn render(&self) -> String {
        let mut rendered = String::new();

        self.render_into(&mut rendered);

        rendered
    }

    /// The same canonical rendering, APPENDED to a caller's sink.
    ///
    /// [`render`](CellPath::render) is R29's surface and stays exactly what it
    /// was; this is the same text written where it belongs. A path is almost
    /// never the whole of a line — it sits inside an entry the emitter is
    /// already building — so a caller with a sink of its own pays no `String`
    /// per path.
    pub fn render_into(&self, out: &mut String) {
        for (index, segment) in self.0.iter().enumerate() {
            if index > 0 {
                out.push('.');
            }

            match segment {
                Segment::Name { text, ordinal } => {
                    out.push_str(text);

                    if let Some(ordinal) = ordinal {
                        let _ = write!(out, ":{ordinal}");
                    }
                }
                Segment::Key(key) => render_key(out, key),
            }
        }
    }
}

// A key's canonical spelling (R41, §15.4.1.1): bare iff NAME-shaped AND not one
// of the five RESERVED spellings, otherwise single-quoted by the one canonical
// payload emitter (033-243, 033-249).
//
// The reservation clause is what makes canonical uniqueness MEANING-safe.
// Without it, `Key("null")` would render bare and the next read would take the
// bare word for the keyword — a fixpoint-stable canonicalization that changes
// what the text says. Quoted, the key reads back as the key it is.
//
// The empty key is not NAME-shaped, so it renders as `''`; a key carrying a
// newline, a carriage return or a control scalar is spellable for the first
// time, because the escape set is the string form's (R63).
fn render_key(out: &mut String, key: &str) {
    if is_name_shaped(key) && !is_reserved_word(key) {
        out.push_str(key);

        return;
    }

    quote(out, '\'', key);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn name(text: &str) -> Segment {
        Segment::Name {
            text: text.to_string(),
            ordinal: None,
        }
    }

    fn path(names: &[&str]) -> CellPath {
        CellPath::new(names.iter().map(|text| name(text)).collect())
    }

    #[test]
    fn renders_segments_joined_by_dots() {
        assert_eq!(path(&["app", "mixer", "gain"]).render(), "app.mixer.gain");
    }

    #[test]
    fn renders_a_single_segment_path() {
        assert_eq!(path(&["app"]).render(), "app");
    }

    // The ordinal attaches to ANY segment, not just the tail — the corpus's own
    // `App.print:0.text` puts it on the middle component.
    #[test]
    fn renders_an_ordinal_suffix_on_any_segment() {
        let cell = CellPath::new(vec![
            name("App"),
            Segment::Name {
                text: "print".to_string(),
                ordinal: Some(0),
            },
            name("text"),
        ]);

        assert_eq!(cell.render(), "App.print:0.text");

        let tail = CellPath::new(vec![
            name("App"),
            Segment::Name {
                text: "print".to_string(),
                ordinal: Some(12),
            },
        ]);

        assert_eq!(tail.render(), "App.print:12");
    }

    // Canonical quoting (R41): bare iff the key is NAME-shaped — the charset
    // exactly, nothing more and nothing less — so a purely numeric key quotes.
    #[test]
    fn renders_a_key_bare_only_when_it_is_name_shaped() {
        let bare = CellPath::new(vec![
            name("App"),
            name("rows"),
            Segment::Key("row_1".to_string()),
            name("render_row"),
        ]);

        assert_eq!(bare.render(), "App.rows.row_1.render_row");

        let quoted = CellPath::new(vec![
            name("App"),
            name("rows"),
            Segment::Key("42".to_string()),
            name("render_row"),
        ]);

        assert_eq!(quoted.render(), "App.rows.'42'.render_row");

        let dotted = CellPath::new(vec![
            name("App"),
            name("rows"),
            Segment::Key("us-east.1".to_string()),
            name("render_row"),
        ]);

        assert_eq!(dotted.render(), "App.rows.'us-east.1'.render_row");
    }

    // The second half of the bare-iff test (002-39): a key spelled by one of
    // the five reserved words IS NAME-shaped, and still quotes — a bare
    // rendering would read back as the keyword rather than as the key.
    #[test]
    fn renders_a_key_spelled_by_a_reserved_word_quoted() {
        for word in ["null", "true", "false", "nan", "inf"] {
            let keyed = CellPath::new(vec![name("App"), Segment::Key(word.to_string())]);

            assert_eq!(keyed.render(), format!("App.'{word}'"));
        }

        // Not a prefix test: `nullable` is an ordinary key and renders bare.
        let ordinary = CellPath::new(vec![name("App"), Segment::Key("nullable".to_string())]);

        assert_eq!(ordinary.render(), "App.nullable");
    }

    // The key's escape set is the string form's (R63), so the quoted spelling
    // is `quote`'s: the delimiter, the backslash and a carriage return escape,
    // a newline and a tab stay raw, and any other control scalar becomes
    // `\u{…}`. The empty key is a legal key with no bare spelling, so quoting
    // is what makes it expressible at all.
    #[test]
    fn spells_a_quoted_key_by_the_canonical_payload_rule() {
        let quote = CellPath::new(vec![name("App"), Segment::Key("it's".to_string())]);

        assert_eq!(quote.render(), "App.'it\\'s'");

        let backslash = CellPath::new(vec![name("App"), Segment::Key("a\\b".to_string())]);

        assert_eq!(backslash.render(), "App.'a\\\\b'");

        // The two spellings that had NO key form before: a carriage return, and
        // a control scalar. `Key("a\r\nb")` used to come back as `"a\nb"`.
        let carriage = CellPath::new(vec![name("App"), Segment::Key("a\rb".to_string())]);

        assert_eq!(carriage.render(), "App.'a\\rb'");

        let pair = CellPath::new(vec![name("App"), Segment::Key("a\r\nb".to_string())]);

        assert_eq!(pair.render(), "App.'a\\r\nb'");

        let control = CellPath::new(vec![name("App"), Segment::Key("a\u{0}b".to_string())]);

        assert_eq!(control.render(), "App.'a\\u{0}b'");

        // Raw, on purpose — the same owner-ruled half of the STRING design.
        let raw = CellPath::new(vec![name("App"), Segment::Key("a\nb\tc".to_string())]);

        assert_eq!(raw.render(), "App.'a\nb\tc'");

        let empty = CellPath::new(vec![name("App"), Segment::Key(String::new())]);

        assert_eq!(empty.render(), "App.''");
    }

    // `render_into` is the rendering and `render` is a wrapper around it, so
    // the two agree by construction — and the sink is APPENDED to, never
    // cleared, which is the whole reason a caller uses it.
    #[test]
    fn renders_into_a_callers_sink_by_appending() {
        let cell = CellPath::new(vec![
            name("App"),
            Segment::Name {
                text: "print".to_string(),
                ordinal: Some(0),
            },
            Segment::Key("42".to_string()),
        ]);

        let mut out = "inputs [".to_string();
        cell.render_into(&mut out);
        out.push(']');

        assert_eq!(out, "inputs [App.print:0.'42']");
        assert_eq!(out, format!("inputs [{}]", cell.render()));
    }

    // The consuming accessor hands the list over so a caller editing one
    // segment need not clone the whole path.
    #[test]
    fn hands_its_segments_over_when_consumed() {
        let cell = CellPath::new(vec![
            name("App"),
            Segment::Name {
                text: "a".to_string(),
                ordinal: Some(5),
            },
        ]);
        let mut segments = cell.into_segments();

        let Some(Segment::Name { ordinal, .. }) = segments.last_mut() else {
            panic!("the last segment is a name");
        };

        assert_eq!(ordinal.take(), Some(5));
        assert_eq!(CellPath::new(segments).render(), "App.a");
    }

    #[test]
    fn exposes_segments_root_first() {
        let cell = path(&["app", "gain"]);
        let expected = vec![name("app"), name("gain")];

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

    // Ordering is structural, so an ordinal is compared after its name and a
    // key sorts after every name — variant declaration order, nothing more.
    #[test]
    fn orders_ordinals_after_their_name_and_keys_after_names() {
        let mut paths = [
            CellPath::new(vec![Segment::Key("z".to_string())]),
            CellPath::new(vec![Segment::Name {
                text: "print".to_string(),
                ordinal: Some(2),
            }]),
            CellPath::new(vec![Segment::Name {
                text: "print".to_string(),
                ordinal: None,
            }]),
            CellPath::new(vec![Segment::Name {
                text: "print".to_string(),
                ordinal: Some(10),
            }]),
        ];
        paths.sort();
        let rendered: Vec<String> = paths.iter().map(CellPath::render).collect();

        // `None` before `Some` is `Option`'s own order; 2 before 10 is numeric,
        // which a text ordinal would have got backwards.
        assert_eq!(rendered, ["print", "print:2", "print:10", "z"]);
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

    // An ordinal is part of the identity: `App.print` and `App.print:0` are two
    // cells, and nothing here folds them together.
    #[test]
    fn an_ordinal_distinguishes_two_paths() {
        let bare = CellPath::new(vec![name("App"), name("print")]);
        let ordinal = CellPath::new(vec![
            name("App"),
            Segment::Name {
                text: "print".to_string(),
                ordinal: Some(0),
            },
        ]);

        assert_ne!(bare, ordinal);
    }

    // The invariant tests below exercise `debug_assert`, which compiles out in
    // release builds; `cargo test` builds in debug, and the cfg_attr keeps a
    // `--release` run honest instead of failing.
    #[test]
    #[should_panic(expected = "a cell path has at least one segment")]
    #[cfg_attr(not(debug_assertions), ignore = "debug_assert compiles out")]
    fn new_rejects_an_empty_path() {
        CellPath::new(Vec::new());
    }

    #[test]
    #[should_panic(expected = "a path segment name is NAME-shaped")]
    #[cfg_attr(not(debug_assertions), ignore = "debug_assert compiles out")]
    fn new_rejects_an_empty_name() {
        CellPath::new(vec![name("")]);
    }

    #[test]
    #[should_panic(expected = "a path segment name is NAME-shaped")]
    #[cfg_attr(not(debug_assertions), ignore = "debug_assert compiles out")]
    fn new_rejects_a_dotted_name() {
        CellPath::new(vec![name("app.gain")]);
    }

    // The reservation as a path invariant (002-39). `null` IS NAME-shaped, so
    // the shape assert passes and this one is what catches it — and catching it
    // HERE is why no emitter position needs a guard of its own: a path spelled
    // by a reserved word cannot be built, so it can never be rendered.
    #[test]
    #[should_panic(expected = "a path segment name is not a reserved spelling")]
    #[cfg_attr(not(debug_assertions), ignore = "debug_assert compiles out")]
    fn new_rejects_a_reserved_name() {
        CellPath::new(vec![name("App"), name("null")]);
    }

    // A key is exempt: the string that would be an illegal NAME is a perfectly
    // good key, and quoting is what makes it spellable. A reserved spelling is
    // exempt for the same reason — as a KEY it is data, not a keyword.
    #[test]
    fn new_accepts_any_key() {
        let cell = CellPath::new(vec![Segment::Key("app.gain".to_string())]);

        assert_eq!(cell.render(), "'app.gain'");

        let reserved = CellPath::new(vec![Segment::Key("null".to_string())]);

        assert_eq!(reserved.render(), "'null'");
    }
}
