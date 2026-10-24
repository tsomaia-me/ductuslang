use crate::project::Project;
use crate::project_file::ProjectFile;

pub fn dsp_project() -> Project {
    Project {
        name: String::from("dsp"),
        files: vec![ProjectFile {
            path: String::from("synth.sym"),
            text: String::from(include_str!("../projects/dsp/synth.sym")),
        }],
    }
}
