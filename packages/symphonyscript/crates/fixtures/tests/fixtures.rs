use symphonyscript_fixtures::{
    SPEC_SYM_EXAMPLES, dsp_project, music_project, spec_examples, spec_productions, spec_projects,
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
    assert_eq!(count("sym"), 28);
    assert_eq!(count("sym-error"), 1);
    assert!(count("ebnf") > 0);
    let error = examples
        .iter()
        .find(|example| example.tag == "sym-error")
        .expect("one sym-error example");
    assert_eq!(error.section, "4.1");
    assert_eq!(error.text, "Clip melody:\n    Note/300000000000\n");
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
    assert!(
        projects[0].files[0]
            .text
            .starts_with("input cutoff = 0.5\n")
    );
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
