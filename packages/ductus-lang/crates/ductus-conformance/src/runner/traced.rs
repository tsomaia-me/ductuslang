//! The `traced` function: load, drive, and hand back everything the sink heard.

use ductus_ir::module::Module;
use ductus_runtime::engine::Engine;
use ductus_runtime::loader::load;

use crate::runner::recording_sink::RecordingSink;
use crate::stub_library::stub_registry::StubRegistry;

/// Loads `module` under `registry`, drives `script` against a FRESH engine and
/// a FRESH sink, and hands back the whole trace.
///
/// The two construction routes of a witness's claim 5 differ ONLY in the module
/// they pass here: same registry, same script, same fresh engine — so a trace
/// difference between them can only be a module difference, which is the
/// question that claim asks.
///
/// # Panics
///
/// If `module` does not load. A witness whose module the loader rejects is a
/// broken witness, and the diagnostic belongs at the call that produced it.
#[must_use]
pub fn traced<S>(module: &Module, registry: &StubRegistry, script: S) -> String
where
    S: Fn(&mut Engine<'_>),
{
    let graph = load(module, registry).expect("the witness module loads");
    let mut sink = RecordingSink::new();
    let mut engine = Engine::start(graph, &mut sink);

    script(&mut engine);
    engine.teardown();

    sink.into_trace()
}

#[cfg(test)]
mod tests {
    use ductus_ir::graph::{CellKind, Entry, ValueLiteral};
    use ductus_ir::types::Prim;

    use crate::module_builder::{cell, module, scope};
    use crate::stub_library::registry_of::registry_of;

    use super::*;

    // The startup block alone — §13.14.1 step 4's first commit over a graph
    // with one input and nothing to evaluate.
    #[test]
    fn hands_back_the_trace_the_script_produced() {
        let mut input = cell(CellKind::Input, &["App", "x"], Prim::I64);

        input.init = Some(ValueLiteral::int("0".to_string()));

        let witness = module(vec![scope(&["App"], vec![Entry::Cell(input)])]);
        let trace = traced(&witness, &registry_of(&[]), |_| {});

        assert_eq!(trace, "commit_begin\npublish\ncommit_end\n");
    }
}
