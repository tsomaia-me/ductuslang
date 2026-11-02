use symphonyscript::{Compiler, Source};
use symphonyscript_evolve::{fixture_program, syntax_nodes};
use symphonyscript_fixtures::{large_project, vocabulary};

const LINES: usize = 10_000;
const WARNINGS: [&str; 1] = ["no-entry-instance"];

#[test]
fn the_fixture_project_holds_ten_thousand_lines_across_several_files_and_reads_the_same_each_time()
{
    let project = large_project();
    let lines: usize = project
        .files
        .iter()
        .map(|file| file.text.lines().count())
        .sum();
    assert!(lines >= LINES, "{lines}");
    assert!(project.files.len() > 3, "{}", project.files.len());
    for domain in ["main", "music", "lighting", "control"] {
        assert!(
            project
                .files
                .iter()
                .any(|file| file.path.starts_with(domain)),
            "{domain}"
        );
    }
    assert_eq!(project, large_project());
    let program = fixture_program();
    assert_eq!(program, fixture_program());
    assert_eq!(program.id, serde_json::json!("large"));
    assert_eq!(program.sources.len(), project.files.len());
    assert!(syntax_nodes(&program, &vocabulary()) > 5 * LINES as u64);
}

#[test]
#[ignore = "the fixture vocabulary gains SPEC §1.3's lighting and control kinds in R0a"]
fn the_fixture_project_compiles_against_the_fixture_vocabulary_drawing_only_its_listed_warnings() {
    let compiler = Compiler::new(&vocabulary()).expect("the vocabulary compiles");
    let sources: Vec<Source> = large_project()
        .files
        .into_iter()
        .map(|file| Source {
            path: file.path,
            text: file.text,
        })
        .collect();
    let drawn: Vec<String> = compiler
        .check(&sources)
        .iter()
        .map(|diagnostic| format!("{} {}", diagnostic.code.as_str(), diagnostic.summary))
        .collect();
    let codes: Vec<&str> = drawn
        .iter()
        .map(|line| line.split(' ').next().unwrap_or_default())
        .collect();
    assert_eq!(codes, WARNINGS, "{drawn:#?}");
}
