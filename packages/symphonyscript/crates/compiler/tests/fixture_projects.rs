mod support;

use symphonyscript_fixtures::{dsp_project, music_project, pending_spec_examples, spec_projects};

#[test]
fn every_fixture_project_parses() {
    let pending = pending_spec_examples();
    let projects = [music_project(), dsp_project()].into_iter().chain(
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
