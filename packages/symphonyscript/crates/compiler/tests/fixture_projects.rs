mod support;

use symphonyscript_fixtures::{dsp_project, music_project, spec_projects};

#[test]
fn every_fixture_project_parses() {
    let projects = [music_project(), dsp_project()]
        .into_iter()
        .chain(spec_projects());
    for project in projects {
        for file in &project.files {
            let parsed = support::parsed(&file.text);
            assert_eq!(parsed.diagnostics, vec![], "{} {}", project.name, file.path);
        }
    }
}
