use crate::project::Project;
use crate::project_file::ProjectFile;

pub fn lighting_project() -> Project {
    Project {
        name: String::from("lighting"),
        files: vec![
            ProjectFile {
                path: String::from("rig.sym"),
                text: String::from(include_str!("../projects/lighting/rig.sym")),
            },
            ProjectFile {
                path: String::from("show.sym"),
                text: String::from(include_str!("../projects/lighting/show.sym")),
            },
        ],
    }
}
