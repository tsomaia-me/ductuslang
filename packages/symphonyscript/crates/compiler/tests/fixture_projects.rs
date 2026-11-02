mod support;

use symphonyscript::Code;
use symphonyscript_fixtures::{
    control_project, dsp_project, lighting_project, music_project, pending_spec_examples,
    spec_projects,
};

#[test]
fn every_fixture_project_parses() {
    let pending = pending_spec_examples();
    let projects = [
        music_project(),
        dsp_project(),
        lighting_project(),
        control_project(),
    ]
    .into_iter()
    .chain(
        spec_projects()
            .into_iter()
            .filter(|project| !pending.contains(&project.name)),
    );
    for project in projects {
        for file in &project.files {
            let parsed = support::parsed(&file.text);
            assert_eq!(parsed.diagnostics, vec![], "{} {}", project.name, file.path);
        }
    }
}

#[test]
fn every_fixture_project_draws_exactly_its_listed_warnings() {
    let expected = [
        ("music", vec![(Code::UnreachableInstance, "main.sym")]),
        ("dsp", vec![(Code::NoEntryInstance, "synth.sym")]),
        ("lighting", vec![(Code::NoEntryInstance, "rig.sym")]),
        ("control", vec![(Code::NoEntryInstance, "boiler.sym")]),
    ];
    let projects = [
        music_project(),
        dsp_project(),
        lighting_project(),
        control_project(),
    ];
    for (project, (name, warnings)) in projects.iter().zip(expected) {
        assert_eq!(project.name, name);
        let files: Vec<(&str, &str)> = project
            .files
            .iter()
            .map(|file| (file.path.as_str(), file.text.as_str()))
            .collect();
        let drawn: Vec<(Code, String)> = support::checked_files_all(&files)
            .into_iter()
            .map(|diagnostic| (diagnostic.code, diagnostic.span.file.to_string()))
            .collect();
        let warnings: Vec<(Code, String)> = warnings
            .into_iter()
            .map(|(code, file)| (code, file.to_string()))
            .collect();
        assert_eq!(drawn, warnings, "{name}");
    }
}
