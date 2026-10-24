use crate::project::Project;
use crate::project_file::ProjectFile;

pub fn music_project() -> Project {
    Project {
        name: String::from("music"),
        files: vec![
            ProjectFile {
                path: String::from("clips.sym"),
                text: String::from(include_str!("../projects/music/clips.sym")),
            },
            ProjectFile {
                path: String::from("main.sym"),
                text: String::from(include_str!("../projects/music/main.sym")),
            },
        ],
    }
}
