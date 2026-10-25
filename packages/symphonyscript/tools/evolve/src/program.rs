use serde::{Deserialize, Serialize};
use symphonyscript::Source;

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Program {
    pub id: serde_json::Value,
    pub sources: Vec<ProgramSource>,
}

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct ProgramSource {
    pub path: String,
    pub text: String,
}

impl Program {
    pub fn compiler_sources(&self) -> Vec<Source> {
        self.sources
            .iter()
            .map(|source| Source {
                path: source.path.clone(),
                text: source.text.clone(),
            })
            .collect()
    }

    pub fn with_sources(&self, sources: Vec<ProgramSource>) -> Program {
        Program {
            id: self.id.clone(),
            sources,
        }
    }

    pub fn ordered_sources(&self) -> Vec<&ProgramSource> {
        let mut ordered: Vec<&ProgramSource> = self.sources.iter().collect();
        ordered.sort_by(|a, b| a.path.as_bytes().cmp(b.path.as_bytes()));
        ordered
    }

    pub fn size(&self) -> usize {
        self.sources.iter().map(|source| source.text.len()).sum()
    }
}
