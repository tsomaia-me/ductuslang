use symphonyscript_fixtures::large_project;

use crate::program::{Program, ProgramSource};

pub fn fixture_program() -> Program {
    let project = large_project();
    Program {
        id: serde_json::Value::String(project.name),
        sources: project
            .files
            .into_iter()
            .map(|file| ProgramSource {
                path: file.path,
                text: file.text,
            })
            .collect(),
    }
}
