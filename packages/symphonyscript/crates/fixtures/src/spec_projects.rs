use crate::project::Project;
use crate::project_file::ProjectFile;
use crate::spec_examples::spec_examples;
use crate::spec_sym_examples::SPEC_SYM_EXAMPLES;

pub fn spec_projects() -> Vec<Project> {
    spec_examples()
        .into_iter()
        .filter(|example| {
            example.tag == "sym"
                && SPEC_SYM_EXAMPLES
                    .iter()
                    .any(|(section, ordinal, undeclared)| {
                        *section == example.section
                            && *ordinal == example.ordinal
                            && undeclared.is_empty()
                    })
        })
        .map(|example| Project {
            name: example.name(),
            files: vec![ProjectFile {
                path: format!("{}.sym", example.name()),
                text: example.text,
            }],
        })
        .collect()
}
