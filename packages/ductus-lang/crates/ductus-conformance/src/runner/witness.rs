//! The `Witness` struct: a witness's two corpus files, loaded.

use std::fs;

/// A loaded witness — the `.duir` module text and its `.trace` golden
/// (corpus/README.md's pair convention, R9).
///
/// The two files are the shared corpus artifacts; only the driver is Rust. A
/// witness test loads the pair, asserts the source round-trips to its own
/// bytes, drives the module, and byte-compares the emitted trace against
/// `golden` through
/// [`assert_trace()`](crate::trace_compare::assert_trace()).
///
/// # Why `golden` is not an `Option`
///
/// Exactly one M0 witness has no golden: the trap witness, whose commit ends
/// in a panic (D8, §13.13.1) — past an abort there is no trace to observe, so
/// the missing golden is a LAW about that witness, not a loading mode. Making
/// the field optional would push an `expect` into every ordinary witness to
/// pay for the one exception. Instead the type models the pair, and the trap
/// witness never builds a `Witness` at all: it reads its source through
/// [`Witness::source_only`] and asserts the panic.
#[derive(Debug, Clone)]
pub struct Witness {
    /// The `.duir` module text, verbatim.
    pub source: String,
    /// The `.trace` golden, verbatim — hand-derived from the semantics
    /// contract (M0.md §4), never captured from a run.
    pub golden: String,
}

impl Witness {
    /// Loads `corpus/duir/m0/<name>.duir` and `corpus/duir/m0/<name>.trace`.
    ///
    /// Paths resolve against the CRATE MANIFEST, never the process cwd
    /// (corpus/README.md) — `cargo test` from any directory finds the corpus.
    ///
    /// # Panics
    ///
    /// If either file is missing or unreadable. A witness naming a corpus file
    /// that is not there is a broken test, not a runtime condition, so it
    /// fails loudly with the resolved path in the message.
    #[must_use]
    pub fn load(name: &str) -> Witness {
        Witness {
            source: read(name, "duir"),
            golden: read(name, "trace"),
        }
    }

    /// The `.duir` source of a witness that has NO golden — the trap witness
    /// alone (see the type's docs).
    ///
    /// # Panics
    ///
    /// If the file is missing or unreadable.
    #[must_use]
    pub fn source_only(name: &str) -> String {
        read(name, "duir")
    }
}

/// Reads one corpus file of a witness pair, anchored at the crate manifest.
fn read(name: &str, extension: &str) -> String {
    let path = format!(
        "{}/../../corpus/duir/m0/{name}.{extension}",
        env!("CARGO_MANIFEST_DIR")
    );

    fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("witness `{name}`: cannot read `{path}`: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    // The pair convention, exercised against a corpus file that exists since
    // the graph increment — the claim is the MANIFEST anchoring, which is what
    // breaks when a test runner picks a different cwd.
    #[test]
    fn loads_a_source_relative_to_the_crate_manifest() {
        let source = Witness::source_only("scopes");

        assert!(source.starts_with("module "));
    }

    #[test]
    #[should_panic(expected = "cannot read")]
    fn panics_with_the_resolved_path_when_a_file_is_missing() {
        let _ = Witness::source_only("no_such_witness_exists");
    }
}
