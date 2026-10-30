use symphonyscript_core::{
    AttrId, Binding, Entry, Instance, InstanceId, Path, Program, Segment, Span, Target, Value,
    Wire, dump,
};
use symphonyscript_fixtures::{
    Rows, SPEC_SYM_EXAMPLES, covered, covering, dsp_project, music_project, native_vocabulary,
    signature_vocabulary, spec_examples, spec_productions, spec_projects, to_array_vocabulary,
    vocabulary, vocabulary_documented,
};

#[test]
fn the_vocabulary_builds_with_the_kinds_of_spec_1_3() {
    let vocab = vocabulary();
    let kinds: Vec<&str> = vocab
        .node_kinds()
        .iter()
        .map(|kind| kind.name.as_str())
        .collect();
    assert_eq!(
        kinds,
        [
            "Session", "Track", "Clip", "Note", "Kit", "Sampler", "Piano", "Synth", "Osc",
            "Filter", "Mixer", "Output"
        ]
    );
    let wires: Vec<&str> = vocab
        .wire_kinds()
        .iter()
        .map(|kind| kind.name.as_str())
        .collect();
    assert_eq!(wires, ["Plays", "Starts", "Includes", "Feeds", "Modulates"]);
    assert!(vocab.prelude().is_empty());
}

#[test]
fn the_vocabulary_stamp_is_pinned() {
    assert_eq!(vocabulary().stamp(), vocabulary().stamp());
    assert_eq!(vocabulary().stamp().name, "example");
    assert_eq!(vocabulary().stamp().hash, 0xb24e_d010_8497_1fb2);
}

#[test]
fn the_native_vocabulary_builds_with_one_stamp() {
    assert_eq!(native_vocabulary().stamp(), native_vocabulary().stamp());
}

#[test]
fn the_signature_and_to_array_vocabularies_build_under_their_names_with_one_stamp() {
    assert_eq!(
        signature_vocabulary().stamp(),
        signature_vocabulary().stamp()
    );
    assert_eq!(signature_vocabulary().stamp().name, "signatures");
    assert_eq!(to_array_vocabulary().stamp(), to_array_vocabulary().stamp());
    assert_eq!(to_array_vocabulary().stamp().name, "to-arrays");
}

#[test]
fn a_doc_edit_leaves_the_stamp_unchanged() {
    let edited = vocabulary_documented(|text| format!("{text} Edited."));
    assert_eq!(edited.stamp(), vocabulary().stamp());
    let note = edited.node_kind_named("Note").expect("Note exists");
    assert_eq!(
        edited.node_kind(note).and_then(|decl| decl.doc.as_deref()),
        Some("A single pitched event inside a clip. Edited.")
    );
}

#[test]
fn every_sym_example_of_the_spec_is_listed_once() {
    let examples = spec_examples();
    let sym: Vec<(String, usize)> = examples
        .iter()
        .filter(|example| example.tag == "sym")
        .map(|example| (example.section.clone(), example.ordinal))
        .collect();
    let listed: Vec<(String, usize)> = SPEC_SYM_EXAMPLES
        .iter()
        .map(|(section, ordinal, _)| (section.to_string(), *ordinal))
        .collect();
    assert_eq!(sym, listed);
}

#[test]
fn the_listed_undeclared_names_appear_in_their_examples() {
    let examples = spec_examples();
    for (section, ordinal, undeclared) in SPEC_SYM_EXAMPLES {
        let example = examples
            .iter()
            .find(|example| {
                example.tag == "sym" && example.section == *section && example.ordinal == *ordinal
            })
            .expect("the listed example exists");
        for name in *undeclared {
            assert!(
                words(&example.text).any(|word| word == *name),
                "{name} is not used in {}",
                example.name()
            );
        }
    }
}

#[test]
fn the_spec_has_sym_and_sym_error_examples() {
    let examples = spec_examples();
    let count = |tag: &str| examples.iter().filter(|example| example.tag == tag).count();
    assert_eq!(count("sym"), 50);
    assert!(count("ebnf") > 0);
    let errors: Vec<(&str, &str)> = examples
        .iter()
        .filter(|example| example.tag == "sym-error")
        .map(|example| (example.section.as_str(), example.text.as_str()))
        .collect();
    assert_eq!(
        errors,
        [
            (
                "3.9",
                "state port(none): i32?\n\nSynth s:\n    Osc | freq=port\n"
            ),
            (
                "3.11",
                "state body(json {}): json[..4096]\n\nstate small: json[..256] = body[\"items\"]\n"
            ),
            ("4.1", "Clip melody:\n    Note/300000000000\n"),
            (
                "5.10",
                "state held([]): i32[..10]\n\nstate total = 1 + for x in held into sum from 0: sum + x\n"
            ),
            (
                "5.10",
                "fn count(xs): xs.size\n\nstate held([]): i32[..10]\n\nstate high = count(for x in held into kept from []: if x > 60 then kept + [x] else kept)\n"
            ),
            (
                "6.3",
                "state n(5)\n\nfn fib(k): if k < 2 then k else fib(k - 1) + fib(k - 2)\n\nClip c:\n    Note/(fib(n))\n"
            ),
            (
                "6.6",
                "state note(60)\n\nstate recent = recent.past(1, []) + [note]\n"
            ),
            ("6.7", "state g(x: x + 1)\n"),
            (
                "6.7",
                "fn bar(n): n + 1\n\nlet baz = bar\nlet same = baz is bar\n\nClip c:\n    Note when same\n"
            ),
            ("6.7", "let f = x: f(x)\n\nClip c:\n    Note/(f(60))\n"),
            (
                "6.7",
                "fn apply(f: (i32) -> i32, x: i32) -> i32: f(x)\n\nClip c:\n    Note/(apply((a, b): a + b, 60))\n"
            ),
            (
                "6.8",
                "fn shorter[const N](xs: i32[N]) -> i32[N - 1]: xs[1..]\n\nClip c:\n    Note/(shorter([60]).size)\n"
            ),
            (
                "6.8",
                "fn half[const N](xs: i32[N]) -> i32[N / 2]: xs[..N div 2]\n\nClip c:\n    Note/(half([60, 64, 67, 72])[0])\n"
            ),
            (
                "6.8",
                "fn same[T](a: T, b: T) -> T: a\n\nClip c:\n    Note/(same(60i32, true))\n"
            ),
            (
                "6.8",
                "fn tail[const N](xs: i32[N + 1]) -> i32[N]: xs[1..]\n\nClip c:\n    Note/(tail([60, 64, 67])[0])\n"
            ),
            (
                "6.8",
                "type Pair[A, B]:\n    left: A\n    right: B\n\nlet p = Pair {left: 1i32}\n\nClip c:\n    Note/(p.left)\n"
            ),
            (
                "7.9",
                "Clip c:\n    for i, k in 0..4:\n        Note/(60 + k)\n"
            ),
            (
                "7.9",
                "state n(4)\n\nstate total = for i in 0..n into sum from 0: sum + i\n\nClip c:\n    Note/total\n"
            ),
            ("10.2", "Pad kick | hit=Hit {}\n")
        ]
    );
}

#[test]
fn spec_projects_are_the_self_contained_examples() {
    let projects = spec_projects();
    let expected = SPEC_SYM_EXAMPLES
        .iter()
        .filter(|(_, _, undeclared)| undeclared.is_empty())
        .count();
    assert_eq!(projects.len(), expected);
    assert_eq!(projects[0].name, "spec-1.1-sym-1");
    assert_eq!(projects[0].files[0].path, "spec-1.1-sym-1.sym");
    assert!(projects[0].files[0].text.starts_with("state cutoff(0.5)\n"));
}

#[test]
fn the_spec_productions_are_extracted() {
    let productions = spec_productions();
    for name in [
        "File",
        "TopItem",
        "NumberLit",
        "Expr",
        "Block",
        "Destination",
    ] {
        assert!(productions.iter().any(|production| production == name));
    }
    let mut unique = productions.clone();
    unique.sort();
    unique.dedup();
    assert_eq!(unique.len(), productions.len());
}

#[test]
fn the_music_and_dsp_projects_use_every_kind_and_wire_kind() {
    let vocab = vocabulary();
    let text: String = [music_project(), dsp_project()]
        .iter()
        .flat_map(|project| project.files.iter().map(|file| file.text.clone()))
        .collect();
    let names = vocab
        .node_kinds()
        .iter()
        .map(|kind| kind.name.as_str())
        .chain(vocab.wire_kinds().iter().map(|kind| kind.name.as_str()));
    for name in names {
        assert!(words(&text).any(|word| word == name), "{name} is unused");
    }
}

fn words(text: &str) -> impl Iterator<Item = &str> {
    text.split(|c: char| !(c.is_alphanumeric() || c == '_' || c == '#'))
        .filter(|word| !word.is_empty())
}

#[test]
fn two_builds_of_the_vocabulary_share_their_handles() {
    let first = vocabulary();
    let second = vocabulary();
    for (index, decl) in first.node_kinds().iter().enumerate() {
        let from_first = first.node_kind_named(&decl.name).expect("the kind exists");
        let from_second = second.node_kind_named(&decl.name).expect("the kind exists");
        assert_eq!(from_first, from_second);
        assert_eq!(second.node_kind(from_first), Some(decl));
        assert_eq!(
            first.node_kind(from_second).map(|other| &other.name),
            Some(&decl.name)
        );
        assert_eq!(first.node_kind_at(index), Some(from_second));
    }
    for decl in first.wire_kinds() {
        let handle = first
            .wire_kind_named(&decl.name)
            .expect("the wire kind exists");
        assert_eq!(
            second.wire_kind(handle).map(|other| &other.name),
            Some(&decl.name)
        );
        assert_eq!(second.wire_kind_named(&decl.name), Some(handle));
    }
    let wave = first.enum_named("Wave").expect("Wave exists");
    assert_eq!(
        second.enum_decl(wave).map(|decl| decl.name.as_str()),
        Some("Wave")
    );
    let (beats, bars) = first.unit_named("bars").expect("bars exists");
    assert_eq!(
        second
            .family(beats)
            .map(|decl| decl.units[bars].name.as_str()),
        Some("bars")
    );

    let span = Span {
        file: "main.sym".into(),
        range: 0..10,
    };
    let named = |name: &str| Path {
        segments: vec![Segment::Named(name.into())],
    };
    let clip = first.node_kind_named("Clip").expect("Clip exists");
    let plays = second.wire_kind_named("Plays").expect("Plays exists");
    let program = Program {
        vocabulary: first.stamp().clone(),
        top_level: vec![InstanceId(0)],
        instances: vec![Instance {
            kind: clip,
            path: named("intro"),
            span: span.clone(),
            expanded_from: vec![],
            attrs: vec![(AttrId(0), Binding::Const(Value::F32(0.5)))],
            gate: None,
            sequence: vec![Entry::Wire(symphonyscript_core::WireId(0))],
        }],
        wires: vec![Wire {
            kind: plays,
            path: Path {
                segments: vec![
                    Segment::Named("intro".into()),
                    Segment::Anonymous {
                        name: "Plays".into(),
                        index: 0,
                    },
                ],
            },
            span,
            expanded_from: vec![],
            source: InstanceId(0),
            target: Target::Fixed(InstanceId(0)),
            attrs: vec![(AttrId(1), Binding::Const(Value::I32(2)))],
            gate: None,
        }],
        inputs: vec![],
        cells: vec![],
        strings: vec![],
    };
    let text = dump(&program, &first);
    assert_eq!(text, dump(&program, &second));
    assert!(text.contains("  kind Clip\n"));
    assert!(text.contains("    transpose const i32 2\n"));
}

fn pairs(rows: &[&Vec<usize>]) -> std::collections::BTreeSet<(usize, usize, usize, usize)> {
    let mut found = std::collections::BTreeSet::new();
    for row in rows {
        for (first, a) in row.iter().enumerate() {
            for (second, b) in row.iter().enumerate().skip(first) {
                found.insert((first, *a, second, *b));
            }
        }
    }
    found
}

#[test]
fn a_covering_design_holds_every_pair_of_values_its_rows_hold_in_fewer_rows() {
    let mut full: Vec<Vec<usize>> = vec![Vec::new()];
    for size in [5, 4, 3, 3, 4, 3, 2] {
        full = full
            .into_iter()
            .flat_map(|row| {
                (0..size).map(move |value| {
                    let mut grown = row.clone();
                    grown.push(value * 7);
                    grown
                })
            })
            .collect();
    }
    assert_eq!(full.len(), 4_320);
    let constrained: Vec<Vec<usize>> = full
        .iter()
        .filter(|row| row[0] != 28 || row[1] == 0)
        .cloned()
        .collect();
    for (rows, most) in [(&full, 40), (&constrained, 40)] {
        let chosen = covering(rows);
        assert_eq!(chosen, covering(rows));
        assert!(chosen.windows(2).all(|pair| pair[0] < pair[1]));
        let picked: Vec<&Vec<usize>> = chosen.iter().map(|index| &rows[*index]).collect();
        let every: Vec<&Vec<usize>> = rows.iter().collect();
        assert_eq!(pairs(&picked), pairs(&every));
        assert!(
            picked.len() >= 20 && picked.len() <= most,
            "{}",
            picked.len()
        );
    }
    assert_eq!(
        covering(&[vec![0], vec![1], vec![1], vec![2]]),
        vec![0, 1, 3]
    );
    assert_eq!(covering(&[]), Vec::<usize>::new());
}

#[test]
fn a_covered_run_takes_only_the_rows_of_a_covering_design_of_the_rows_it_records() {
    let run = |rows: &Rows| {
        let mut taken = Vec::new();
        for first in 0..4 {
            for second in [10, 20, 30] {
                for third in 0..2 {
                    if rows.take(&[first, second, third]) {
                        taken.push((first, second, third));
                    }
                }
            }
        }
        taken
    };
    assert_eq!(run(&Rows::Every).len(), 24);
    let taken = covered(run);
    assert!(taken.len() >= 12 && taken.len() < 24, "{}", taken.len());
    for first in 0..4 {
        for second in [10, 20, 30] {
            assert!(taken.iter().any(|row| row.0 == first && row.1 == second));
        }
        for third in 0..2 {
            assert!(taken.iter().any(|row| row.0 == first && row.2 == third));
        }
    }
    for second in [10, 20, 30] {
        for third in 0..2 {
            assert!(taken.iter().any(|row| row.1 == second && row.2 == third));
        }
    }
}
