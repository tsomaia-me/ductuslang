use crate::compiler::Compiler;
use crate::diagnostics::Code;
use crate::syntax::front_end_goldens::source;

const AT_THE_BODY: [(&str, &str); 5] = [
    (
        "let xs = [60, 62, 64]\nfn mid(n): n / 2\nClip c:\n    Note/(xs[mid(2)])\n",
        "let xs = [60, 62, 64]\nfn mid(n): n - 1\nClip c:\n    Note/(xs[mid(2)])\n",
    ),
    (
        "let xs = [60, 62, 64]\nfn mid(n): n / 2\nClip c:\n    Note/(xs[mid(2)..][0])\n",
        "let xs = [60, 62, 64]\nfn mid(n): n - 1\nClip c:\n    Note/(xs[mid(2)..][0])\n",
    ),
    (
        "fn mid(n): n / 2\nClip c:\n    Note/(if mid(2) then 1 else 2)\n",
        "fn mid(n): n > 2\nClip c:\n    Note/(if mid(2) then 1 else 2)\n",
    ),
    (
        "fn mid(n): n / 2\nClip c:\n    Note/(if mid(2) and true then 1 else 2)\n",
        "fn mid(n): n > 2\nClip c:\n    Note/(if mid(2) and true then 1 else 2)\n",
    ),
    (
        "fn mid(n): n / 2\nfn twice(n): mid(n)\nlet xs = [60, 62, 64]\nClip c:\n    Note/(xs[twice(2)])\n",
        "fn mid(n): n - 1\nfn twice(n): mid(n)\nlet xs = [60, 62, 64]\nClip c:\n    Note/(xs[twice(2)])\n",
    ),
];

fn errors(found: &[crate::diagnostics::Diagnostic]) -> Vec<Code> {
    found
        .iter()
        .map(|diagnostic| diagnostic.code)
        .filter(|code| *code != Code::NoEntryInstance && *code != Code::UnreachableInstance)
        .collect()
}

#[test]
fn a_type_fault_at_a_calls_result_draws_at_the_bodys_node_chained_to_the_call_beside_its_valid_twin()
 {
    let vocabulary = symphonyscript_fixtures::vocabulary();
    let old = Compiler::new(&vocabulary)
        .expect("the vocabulary compiles")
        .on_the_old_engine();
    let new = Compiler::new(&vocabulary).expect("the vocabulary compiles");
    for (faulty, valid) in AT_THE_BODY {
        let sources = [source(faulty)];
        let found = new
            .checked_by_new_core(&sources)
            .unwrap_or_else(|| panic!("the new core takes {faulty}"));
        assert_eq!(found, old.check(&sources), "{faulty}");
        let fault = found
            .iter()
            .find(|diagnostic| !errors(std::slice::from_ref(diagnostic)).is_empty())
            .unwrap_or_else(|| panic!("{faulty} draws its fault"));
        assert!(
            fault
                .related
                .iter()
                .all(|related| related.label == "in this call")
                && !fault.related.is_empty(),
            "{faulty}"
        );
        let twin = new
            .checked_by_new_core(&[source(valid)])
            .unwrap_or_else(|| panic!("the new core takes {valid}"));
        assert_eq!(errors(&twin), Vec::<Code>::new(), "{valid}");
    }
}
