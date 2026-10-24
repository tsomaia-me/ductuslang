use std::collections::HashMap;

use symphonyscript_core::{EnumId, Vocabulary};

use crate::semantics::Internal;

#[derive(Clone, Debug, Default)]
pub(crate) struct Variants {
    owners: HashMap<String, EnumId>,
    positions: Vec<HashMap<String, u16>>,
}

impl Variants {
    pub fn of(vocabulary: &Vocabulary) -> Result<Self, Internal> {
        let mut variants = Variants::default();
        for (index, decl) in vocabulary.enums().iter().enumerate() {
            let id = vocabulary.enum_at(index).ok_or(Internal::missed("enum"))?;
            let mut positions = HashMap::with_capacity(decl.variants.len());
            for (position, name) in decl.variants.iter().enumerate() {
                let position =
                    u16::try_from(position).map_err(|_| Internal::missed("enum variant"))?;
                positions.entry(name.clone()).or_insert(position);
                variants.owners.entry(name.clone()).or_insert(id);
            }
            variants.positions.push(positions);
        }
        Ok(variants)
    }

    pub fn owner(&self, name: &str) -> Option<EnumId> {
        self.owners.get(name).copied()
    }

    pub fn position(&self, enumeration: EnumId, name: &str) -> Result<Option<u16>, Internal> {
        Ok(self
            .positions
            .get(enumeration.index())
            .ok_or(Internal::missed("enum"))?
            .get(name)
            .copied())
    }
}
