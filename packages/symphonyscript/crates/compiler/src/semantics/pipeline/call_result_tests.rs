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

fn hubs(levels: usize, reads: usize) -> String {
    let mut text =
        String::from("Session main\nClip verse0 | gain=2:\n    Note\nfn s1(c, d, m): c\n");
    for level in 1..=levels {
        let below = if level == 1 {
            "s1".to_string()
        } else {
            format!("h{}", level - 1)
        };
        let sum: Vec<String> = (0..reads)
            .map(|at| {
                let (one, other) = if at % 2 == 0 { ("c", "d") } else { ("d", "c") };
                format!("{below}({one}, {other}, 0).gain")
            })
            .collect();
        text.push_str(&format!(
            "fn h{level}(c, d, m): if {} > 99999 then d else c\n",
            sum.join(" + ")
        ));
    }
    text.push_str(&format!(
        "Clip k:\n    Note/([60][i32(h{levels}(verse0, verse0, 1).gain) - 2])\n"
    ));
    text
}

#[test]
fn a_receiver_calls_arguments_bind_in_the_body_that_holds_the_path_so_an_inline_stop_draws_no_unknown_name_482()
 {
    let vocabulary = symphonyscript_fixtures::vocabulary();
    let overrides = symphonyscript_core::LimitOverrides {
        inlined_calls: Some(10),
        inlined_calls_warning: Some(8),
        ..Default::default()
    };
    let old = Compiler::with_limits(&vocabulary, overrides)
        .expect("the vocabulary compiles")
        .on_the_old_engine();
    let new = Compiler::with_limits(&vocabulary, overrides).expect("the vocabulary compiles");
    let stopped = hubs(3, 3);
    let sources = [source(&stopped)];
    let found = new
        .checked_by_new_core(&sources)
        .expect("the new core takes the inline stop");
    assert!(
        found
            .iter()
            .all(|diagnostic| diagnostic.code != Code::UnknownName),
        "{found:?}"
    );
    assert_eq!(found, old.check(&sources));
    for valid in [hubs(1, 1), hubs(2, 2)] {
        let sources = [source(&valid)];
        assert_eq!(new.check(&sources), old.check(&sources), "{valid}");
        assert!(
            new.check(&sources)
                .iter()
                .all(|diagnostic| diagnostic.code != Code::UnknownName
                    && diagnostic.code != Code::InlineLimit),
            "{valid}"
        );
    }
}
