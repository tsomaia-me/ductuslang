use crate::project_file::ProjectFile;

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Project {
    pub name: String,
    pub files: Vec<ProjectFile>,
}
