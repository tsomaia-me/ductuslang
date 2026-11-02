use crate::project::Project;
use crate::project_file::ProjectFile;

pub fn control_project() -> Project {
    Project {
        name: String::from("control"),
        files: vec![
            ProjectFile {
                path: String::from("boiler.sym"),
                text: String::from(include_str!("../projects/control/boiler.sym")),
            },
            ProjectFile {
                path: String::from("lines.sym"),
                text: String::from(include_str!("../projects/control/lines.sym")),
            },
        ],
    }
}
