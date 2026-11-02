use symphonyscript_evolve::{Program, ProgramSource, expanded_items, syntax_nodes};
use symphonyscript_fixtures::vocabulary;

fn program(text: &str) -> Program {
    Program {
        id: serde_json::json!("counted"),
        sources: vec![ProgramSource {
            path: "main.sym".to_string(),
            text: text.to_string(),
        }],
    }
}

fn expanding(iterations: usize) -> Program {
    program(&format!(
        "Clip c:\n    for i in 0..{iterations}:\n        Note/(60 + i % 12) | velocity=0.5\n"
    ))
}

#[test]
fn the_expanded_items_are_the_instances_and_wires_the_expansion_yields() {
    let vocabulary = vocabulary();
    for iterations in [0, 1, 7, 300] {
        let counted = expanded_items(&expanding(iterations), &vocabulary).expect("it compiles");
        assert_eq!(counted, Some(iterations as u64 + 1), "{iterations}");
    }
    let wired = program("Clip a:\n    Note/60\n    Plays: b\nClip b:\n    Note/62 Note/64\n");
    assert_eq!(
        expanded_items(&wired, &vocabulary).expect("it compiles"),
        Some(6)
    );
}

#[test]
fn the_syntax_nodes_are_counted_over_every_source_and_do_not_grow_with_the_iterations() {
    let vocabulary = vocabulary();
    let small = syntax_nodes(&expanding(3), &vocabulary);
    assert!(small > 10, "{small}");
    assert_eq!(syntax_nodes(&expanding(300_000), &vocabulary), small);
    let mut twice = expanding(3);
    twice.sources.push(ProgramSource {
        path: "other.sym".to_string(),
        text: twice.sources[0].text.replace("Clip c", "Clip d"),
    });
    assert_eq!(syntax_nodes(&twice, &vocabulary), 2 * small);
}
