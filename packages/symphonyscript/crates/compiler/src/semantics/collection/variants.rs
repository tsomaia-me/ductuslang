use std::collections::HashMap;

use symphonyscript_core::{EnumId, Vocabulary};

#[derive(Clone, Debug, Default)]
pub(crate) struct Variants {
    owners: HashMap<String, EnumId>,
    positions: Vec<HashMap<String, u16>>,
}

impl Variants {
    pub fn of(vocabulary: &Vocabulary) -> Self {
        let mut variants = Variants::default();
        for index in 0..vocabulary.enums().len() {
            let (Some(id), Some(decl)) = (vocabulary.enum_at(index), vocabulary.enums().get(index))
            else {
                continue;
            };
            let mut positions = HashMap::with_capacity(decl.variants.len());
            for (position, name) in decl.variants.iter().enumerate() {
                if let Ok(position) = u16::try_from(position) {
                    positions.entry(name.clone()).or_insert(position);
                }
                variants.owners.entry(name.clone()).or_insert(id);
            }
            variants.positions.push(positions);
        }
        variants
    }

    pub fn owner(&self, name: &str) -> Option<EnumId> {
        self.owners.get(name).copied()
    }

    pub fn position(&self, enumeration: EnumId, name: &str) -> Option<u16> {
        self.positions
            .get(enumeration.index())
            .and_then(|positions| positions.get(name))
            .copied()
    }
}
