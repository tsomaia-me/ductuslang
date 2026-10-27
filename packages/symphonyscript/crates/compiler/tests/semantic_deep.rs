mod support;

use std::collections::BTreeMap;

use symphonyscript::{Code, Compiler, Source};
use symphonyscript_core::{Count, Vocabulary};
use symphonyscript_fixtures::deep_projects;

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
fn deep_valid_programs_a_deep_array_type_and_a_long_call_chain_pass_check_on_a_small_stack() {
    let workers: Vec<_> = deep_projects()
        .into_iter()
        .map(|project| {
            std::thread::Builder::new()
                .stack_size(1 << 20)
                .spawn(move || {
                    for file in &project.files {
                        assert_eq!(code_counts(&file.text), BTreeMap::new(), "{}", project.name);
                    }
                })
                .expect("the thread starts")
        })
        .collect();
    for worker in workers {
        worker
            .join()
            .expect("the work finishes without overflowing");
    }
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

#[test]
fn a_bounded_function_expanded_100000_levels_deep_passes_check_on_a_small_stack() {
    on_small_stack(|| {
        let expanded = format!(
            "input held: i32[..4] = []\nfn walk(i) bounded {DEPTH} else 0:\n    if i >= len(held) then 0 else held[i] + walk(i + 1)\nClip c:\n    Note/(walk(0))\n"
        );
        assert_eq!(code_counts(&expanded), BTreeMap::new());
        let evaluated = format!(
            "fn count(i) bounded {DEPTH} else 0: count(i + 1) + 1\nClip c:\n    Note/(count(0) - {DEPTH})\n"
        );
        assert_eq!(code_counts(&evaluated), BTreeMap::new());
    });
}

#[test]
fn a_chain_of_100000_slices_passes_check_on_a_small_stack() {
    on_small_stack(|| {
        let chained = format!(
            "input held: i32[..4] = []\nlet s = held{}\nClip c:\n    Note/(s[0])\n",
            "[1..]".repeat(DEPTH)
        );
        assert_eq!(code_counts(&chained), BTreeMap::new());
        let mut lets = String::from("input held: i32[..4] = []\nlet s0 = held\n");
        for level in 1..=DEPTH {
            lets.push_str(&format!("let s{level} = s{}[1..]\n", level - 1));
        }
        lets.push_str(&format!("Clip c:\n    Note/(s{DEPTH}[0])\n"));
        assert_eq!(code_counts(&lets), BTreeMap::new());
        let fixed = format!(
            "input note = 60\nstate s = [note, note, note]\nlet t = s{}\nClip c:\n    Note/(len(t) + note)\n",
            "[1..]".repeat(DEPTH)
        );
        assert_eq!(code_counts(&fixed), BTreeMap::new());
    });
}
