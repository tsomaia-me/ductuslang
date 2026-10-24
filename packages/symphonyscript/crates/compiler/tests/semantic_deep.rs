mod support;

use std::collections::BTreeMap;

use symphonyscript::{Code, Compiler, Source};
use symphonyscript_core::{Count, Vocabulary};

const DEPTH: usize = 100_000;
const BODY_DEPTH: usize = 3_000;

fn on_small_stack(work: impl FnOnce() + Send + 'static) {
    std::thread::Builder::new()
        .stack_size(1 << 20)
        .spawn(work)
        .expect("the thread starts")
        .join()
        .expect("the work finishes without overflowing");
}

fn nested(open: &str, middle: &str, close: &str) -> String {
    format!("{}{middle}{}", open.repeat(DEPTH), close.repeat(DEPTH))
}

fn code_counts(text: &str) -> BTreeMap<Code, usize> {
    let mut counts = BTreeMap::new();
    for diagnostic in support::checked(text) {
        *counts.entry(diagnostic.code).or_insert(0) += 1;
    }
    counts
}

fn bodies(head: &str, cycle: &[&str]) -> String {
    let mut text = String::from(head);
    let mut indent = 1;
    for level in 0..BODY_DEPTH {
        let pad = " ".repeat(indent);
        let line = cycle[level % cycle.len()].replace("{level}", &level.to_string());
        if line == "when:" {
            text.push_str(&format!("{pad}when:\n{pad} x:\n"));
            indent += 1;
        } else {
            text.push_str(&format!("{pad}{line}\n"));
        }
        indent += 1;
    }
    text.push_str(&format!("{}Note\n", " ".repeat(indent)));
    text
}

#[test]
fn m1_deep_sources_pass_every_stage_on_a_small_stack() {
    on_small_stack(|| {
        let text = format!(
            "let a = {}\nlet b = {}\nlet c = {}\nlet d = {}\n",
            nested("(", "1", ")"),
            nested("[", "1", "]"),
            nested("x[", "0", "]"),
            nested("([", "1", "])"),
        );
        assert_eq!(
            code_counts(&text),
            BTreeMap::from([(Code::UnknownName, DEPTH)])
        );
        let text = format!(
            "let a = {}1\nlet b = {}x\nlet c = {}\nlet d = {}\nlet e = 2{}\nClip c:\n    Note/{}\n",
            "-".repeat(DEPTH),
            "not ".repeat(DEPTH),
            nested("f(", "1", ")"),
            nested("\"{", "1", "}\""),
            " ** 2".repeat(DEPTH),
            nested("(", "60", ")"),
        );
        assert_eq!(
            code_counts(&text),
            BTreeMap::from([(Code::DuplicateName, 1), (Code::UnknownName, 1 + DEPTH)])
        );
        let text = format!("Clip c:\n    {}\n", nested("[", "Note", "]"));
        assert_eq!(
            code_counts(&text),
            BTreeMap::from([(Code::BundleSyntax, 1)])
        );
        let text = format!("Clip c:\n    {}\n", nested("(", "Note", ")"));
        assert!(!code_counts(&text).is_empty());
        let text = bodies(
            "Clip top:\n",
            &["Note:", "for i in 0..1:", "when x:", "when:"],
        );
        assert_eq!(
            code_counts(&text),
            BTreeMap::from([
                (Code::UnknownName, BODY_DEPTH / 2),
                (Code::ForVariableClash, BODY_DEPTH / 4 - 1),
                (Code::ChildNotAccepted, BODY_DEPTH / 4),
            ])
        );
    });
}

#[test]
fn deep_valid_programs_pass_check_on_a_small_stack() {
    on_small_stack(|| {
        let text = format!(
            "input x = true\nfn f(v): v\nlet a = {}\nlet b = {}\nlet c = {}\nlet d = {}x\nlet e = {}\nlet g = {}1\nClip top:\n    Note/{} when d\n    Note/(f(a) + c + g + b{})\n",
            nested("(", "1", ")"),
            nested("[", "1", "]"),
            nested("f(", "1", ")"),
            "not ".repeat(DEPTH),
            nested("\"{", "1", "}\""),
            "-".repeat(DEPTH),
            nested("(", "60", ")"),
            "[0]".repeat(DEPTH),
        );
        assert_eq!(code_counts(&text), BTreeMap::new());
        let text = bodies(
            "input x = true\nClip top:\n",
            &["for v{level} in 0..1:", "when x:", "when:"],
        );
        assert_eq!(code_counts(&text), BTreeMap::new());
    });
}

#[test]
fn a_deep_array_type_and_a_long_chain_of_functions_pass_check_on_a_small_stack() {
    on_small_stack(|| {
        let text = format!(
            "input k: i32{} = {}\nClip c:\n    Note/(k{})\n",
            "[1]".repeat(DEPTH),
            nested("[", "1", "]"),
            "[0]".repeat(DEPTH),
        );
        assert_eq!(code_counts(&text), BTreeMap::new());
        let mut text = String::new();
        for index in 0..10_000 {
            text.push_str(&format!("fn step{index}(x): step{}(x)\n", index + 1));
        }
        text.push_str(
            "fn step10000(x): x\ninput i = 1\nClip c:\n    Note/(step0(i))\n    Note/(step0(64))\n",
        );
        assert_eq!(code_counts(&text), BTreeMap::new());
    });
}

#[test]
fn deeply_nested_kinds_pass_check_on_a_small_stack() {
    on_small_stack(|| {
        let mut v = Vocabulary::builder("nested");
        let group = v.node("Group");
        v.kind(group)
            .top_level(Count::Any)
            .children(group, Count::Any)
            .attr::<i32>("level")
            .default(0)
            .default_attr();
        let vocabulary = v.build().expect("the vocabulary builds");
        let mut text = String::from("input x = true\nGroup top:\n");
        let mut indent = 1;
        for level in 0..BODY_DEPTH {
            let pad = " ".repeat(indent);
            match level % 3 {
                0 => text.push_str(&format!("{pad}Group/{level}:\n")),
                1 => text.push_str(&format!("{pad}for v{level} in 0..1:\n")),
                _ => text.push_str(&format!("{pad}when x:\n")),
            }
            indent += 1;
        }
        text.push_str(&format!("{}Group\n", " ".repeat(indent)));
        let compiler = Compiler::new(&vocabulary).expect("the vocabulary compiles");
        let diagnostics = compiler.check(&[Source {
            path: "nested.sym".into(),
            text,
        }]);
        assert_eq!(support::errors(diagnostics), vec![]);
    });
}
