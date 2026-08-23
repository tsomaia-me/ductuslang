//! The `Transaction` struct: §13.10.4's atomic grouping of staged writes.

use ductus_ir::diag::Diagnostic;
use ductus_ir::path::CellPath;

use super::write_check::write_check;
use crate::loader::{CellIndex, LoadedGraph};
use crate::value::Value;

/// The handle [`Engine::transaction`](super::Engine::transaction) hands its
/// closure — §13.10.4/§13.14.5's atomic grouping of writes, which in M0 is
/// exactly what D3 says it is: host sugar over the staged-write buffer, sharing
/// the one commit path.
///
/// A transaction accumulates `(cell, value)` pairs and applies them to the
/// staged buffer at close, all or nothing. It grants no evaluation and no
/// visibility of its own: §13.10.4's "relationship to commit" is that the
/// applied values sit staged exactly as a bare
/// [`write_signal`](super::Engine::write_signal)'s would, and the next
/// [`commit`](super::Engine::commit) judges their dirtiness against the
/// previously-committed values. Two writes to one cell inside a transaction are
/// therefore last-write-wins at the store, like any other pair of writes.
///
/// # Validation happens at write time, application at close
///
/// Each `write_signal` is checked immediately (the same `write_check` a bare
/// write runs), so the closure sees a rejected write as its own `Err` and can
/// react. Nothing reaches the store until close — a rejected write leaves the
/// accumulated pairs alone, and the closure decides whether to continue or
/// propagate.
///
/// # Two vacuities, stated honestly
///
/// - **Nesting** (§13.10.4's outermost-flattening) is UNREPRESENTABLE here: the
///   engine is exclusively borrowed for the closure's duration, so the closure
///   cannot open a second transaction to flatten. The law holds vacuously in
///   M0 rather than being implemented.
/// - **Writes during a commit** are likewise unrepresentable — `commit` takes
///   `&mut self` and never calls back into host code — so `write_during_commit`
///   has NO emission site in M0. The class stays registered for the milestone
///   that makes it reachable (a re-entrant host callback), rather than being
///   deleted and re-invented.
pub struct Transaction<'e> {
    graph: &'e LoadedGraph,
    writes: Vec<(CellIndex, Value)>,
    aborted: bool,
}

impl<'e> Transaction<'e> {
    /// Opens a transaction over `graph`. Only the engine calls this — the host
    /// receives one through [`Engine::transaction`](super::Engine::transaction).
    pub(super) fn new(graph: &'e LoadedGraph) -> Transaction<'e> {
        Transaction {
            graph,
            writes: Vec::new(),
            aborted: false,
        }
    }

    /// Stages `value` for `path` as part of this transaction (§13.14.5).
    ///
    /// Validated here, applied at close: the checks are §13.14.2's, run by the
    /// same `write_check` a bare write runs, so a transaction's write is legal
    /// under exactly the law a bare write is.
    ///
    /// # Errors
    ///
    /// `unknown_input_path` for a path no cell carries, `write_to_non_input`
    /// for a derived cell, `type_mismatched_write` for a value outside the
    /// cell's declared type. The pairs accumulated so far are untouched.
    pub fn write_signal(&mut self, path: &CellPath, value: Value) -> Result<(), Diagnostic> {
        let cell = write_check(self.graph, path, &value)?;

        self.writes.push((cell, value));

        Ok(())
    }

    /// Cancels the transaction — §13.10.4's `tx.abort()`, "the only rollback
    /// path".
    ///
    /// The closure keeps running and returns normally; nothing this transaction
    /// accumulated ever reaches the staged buffer, so the staged values stay
    /// exactly as they were before the transaction opened. Aborting twice, or
    /// writing after aborting, changes nothing: the flag is one-way.
    pub fn abort(&mut self) {
        self.aborted = true;
    }

    /// The pairs to apply at close — empty when [`abort`](Transaction::abort)
    /// was called, which is how "rolls back the accumulated writes" is spelled
    /// when nothing was applied in the first place.
    pub(super) fn into_writes(self) -> Vec<(CellIndex, Value)> {
        if self.aborted {
            return Vec::new();
        }

        self.writes
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use ductus_ir::diag::DiagClass;
    use ductus_ir::graph::ValueLiteral;
    use ductus_ir::module::Module;
    use ductus_ir::types::Prim;

    use super::super::Engine;
    use super::super::test_support::{
        RecordingSink, STARTUP, StubProvider, derived, input, loaded, module, path, scope,
    };
    use super::*;
    use crate::provider::Behavior;

    fn sum() -> Behavior {
        Rc::new(|inputs| {
            Value::I64(
                inputs
                    .iter()
                    .map(|value| match value {
                        Value::I64(payload) => *payload,
                        other => panic!("an i64 argument, got {other:?}"),
                    })
                    .sum(),
            )
        })
    }

    /// Two `i64` inputs and their sum — the graph every transaction test
    /// drives.
    fn adder() -> Module {
        module(vec![scope(
            &["App"],
            vec![
                input(&["App", "x"], Prim::I64, ValueLiteral::int("2".to_string())),
                input(&["App", "y"], Prim::I64, ValueLiteral::int("3".to_string())),
                derived(&["App", "d"], Prim::I64, 1, &[&["App", "x"], &["App", "y"]]),
            ],
        )])
    }

    // The grouping law: two writes in one transaction commit as ONE block, with
    // both dirty lines and a single evaluation of the join — the same block a
    // pair of bare writes would produce, which is D3's point (sugar over the
    // staged buffer, one commit path).
    #[test]
    fn two_writes_in_a_transaction_commit_as_one_block() {
        let witness = adder();
        let provider = StubProvider::new().with(1, sum());
        let mut sink = RecordingSink::new();
        let mut engine = Engine::start(loaded(&witness, &provider), &mut sink);

        engine
            .transaction(|transaction| {
                transaction.write_signal(&path(&["App", "x"]), Value::I64(20))?;
                transaction.write_signal(&path(&["App", "y"]), Value::I64(30))
            })
            .expect("both writes are legal");
        engine.commit();
        engine.teardown();

        assert_eq!(
            sink.trace(),
            format!(
                "{STARTUP}commit_begin\n\
                 dirty App.x 2 -> 20\n\
                 dirty App.y 3 -> 30\n\
                 eval App.d -> 50\n\
                 publish\ncommit_end\n"
            )
        );
    }

    // Close applies to the STAGED buffer, not to the snapshot: a closed
    // transaction has changed nothing observable until a commit runs.
    #[test]
    fn a_closed_transaction_is_not_yet_visible() {
        let witness = adder();
        let provider = StubProvider::new().with(1, sum());
        let mut sink = RecordingSink::new();
        let mut engine = Engine::start(loaded(&witness, &provider), &mut sink);

        engine
            .transaction(|transaction| {
                transaction.write_signal(&path(&["App", "x"]), Value::I64(20))
            })
            .expect("the write is legal");

        let snapshot = engine.acquire_snapshot();

        assert!(
            snapshot
                .read(&path(&["App", "x"]))
                .expect("the cell exists")
                .bits_eq(&Value::I64(2))
        );

        engine.teardown();

        assert_eq!(sink.trace(), STARTUP);
    }

    // §13.10.4's `abort()` — "the only rollback path". The closure returns
    // normally, the transaction closes `Ok`, and the staged values are exactly
    // what they were, so the next commit is empty.
    #[test]
    fn an_aborted_transaction_applies_nothing() {
        let witness = adder();
        let provider = StubProvider::new().with(1, sum());
        let mut sink = RecordingSink::new();
        let mut engine = Engine::start(loaded(&witness, &provider), &mut sink);

        engine
            .transaction(|transaction| {
                transaction.write_signal(&path(&["App", "x"]), Value::I64(20))?;
                transaction.abort();

                Ok(())
            })
            .expect("an aborted transaction closes normally");
        engine.commit();
        engine.teardown();

        assert_eq!(sink.trace(), format!("{STARTUP}{STARTUP}"));
    }

    // A closure that fails applies nothing — all-or-nothing across the error
    // path — and its error reaches the host unchanged.
    #[test]
    fn a_failing_closure_applies_nothing_and_propagates() {
        let witness = adder();
        let provider = StubProvider::new().with(1, sum());
        let mut sink = RecordingSink::new();
        let mut engine = Engine::start(loaded(&witness, &provider), &mut sink);

        let failure = engine
            .transaction(|transaction| {
                transaction.write_signal(&path(&["App", "x"]), Value::I64(20))?;

                Err(Diagnostic {
                    class: DiagClass::TypeMismatchedWrite,
                    span: None,
                    path: None,
                    message: "the host gave up".to_string(),
                })
            })
            .expect_err("the closure failed");

        assert_eq!(failure.class, DiagClass::TypeMismatchedWrite);

        engine.commit();
        engine.teardown();

        assert_eq!(sink.trace(), format!("{STARTUP}{STARTUP}"));
    }

    // A transaction's write is checked by exactly the law a bare write is: the
    // derived target is refused at write time, and the legal write that
    // preceded it in the same closure never reaches the staged buffer.
    #[test]
    fn a_write_to_a_derived_cell_is_refused_and_applies_nothing() {
        let witness = adder();
        let provider = StubProvider::new().with(1, sum());
        let mut sink = RecordingSink::new();
        let mut engine = Engine::start(loaded(&witness, &provider), &mut sink);

        let failure = engine
            .transaction(|transaction| {
                transaction.write_signal(&path(&["App", "x"]), Value::I64(20))?;
                transaction.write_signal(&path(&["App", "d"]), Value::I64(1))
            })
            .expect_err("a derived cell is not writable");

        assert_eq!(failure.class, DiagClass::WriteToNonInput);
        assert_eq!(
            failure.path.as_ref().map(CellPath::render),
            Some("App.d".to_string())
        );

        engine.commit();
        engine.teardown();

        assert_eq!(sink.trace(), format!("{STARTUP}{STARTUP}"));
    }
}
