use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

use super::enum_decl::EnumDecl;
use super::family_decl::FamilyDecl;
use super::node_kind_decl::NodeKindDecl;
use super::prelude_file::PreludeFile;
use super::vocabulary_builder::VocabularyBuilder;
use super::vocabulary_stamp::VocabularyStamp;
use super::wire_kind_decl::WireKindDecl;
use crate::ids::{EnumId, FamilyId, NodeKind, VocabularyToken, WireKind};

#[derive(Clone, Debug)]
pub struct Vocabulary {
    token: VocabularyToken,
    stamp: VocabularyStamp,
    node_kinds: Vec<NodeKindDecl>,
    wire_kinds: Vec<WireKindDecl>,
    enums: Vec<EnumDecl>,
    families: Vec<FamilyDecl>,
    prelude: Vec<PreludeFile>,
    node_names: BTreeMap<String, u32>,
    wire_names: BTreeMap<String, u32>,
    enum_names: BTreeMap<String, u32>,
    unit_names: BTreeMap<String, (u32, usize)>,
}

impl Vocabulary {
    pub fn builder(name: &str) -> VocabularyBuilder {
        VocabularyBuilder::new(name)
    }

    pub(crate) fn assemble(
        token: VocabularyToken,
        stamp: VocabularyStamp,
        node_kinds: Vec<NodeKindDecl>,
        wire_kinds: Vec<WireKindDecl>,
        enums: Vec<EnumDecl>,
        families: Vec<FamilyDecl>,
        prelude: Vec<PreludeFile>,
    ) -> Self {
        let indexed = |names: &mut dyn Iterator<Item = &String>| {
            names
                .enumerate()
                .map(|(index, name)| (name.clone(), index as u32))
                .collect::<BTreeMap<_, _>>()
        };
        let node_names = indexed(&mut node_kinds.iter().map(|kind| &kind.name));
        let wire_names = indexed(&mut wire_kinds.iter().map(|kind| &kind.name));
        let enum_names = indexed(&mut enums.iter().map(|decl| &decl.name));
        let mut unit_names = BTreeMap::new();
        for (family, decl) in families.iter().enumerate() {
            for (unit, unit_decl) in decl.units.iter().enumerate() {
                unit_names.insert(unit_decl.name.clone(), (family as u32, unit));
            }
        }
        Self {
            token,
            stamp,
            node_kinds,
            wire_kinds,
            enums,
            families,
            prelude,
            node_names,
            wire_names,
            enum_names,
            unit_names,
        }
    }

    pub fn name(&self) -> &str {
        &self.stamp.name
    }

    pub fn stamp(&self) -> &VocabularyStamp {
        &self.stamp
    }

    pub fn node_kinds(&self) -> &[NodeKindDecl] {
        &self.node_kinds
    }

    pub fn wire_kinds(&self) -> &[WireKindDecl] {
        &self.wire_kinds
    }

    pub fn enums(&self) -> &[EnumDecl] {
        &self.enums
    }

    pub fn families(&self) -> &[FamilyDecl] {
        &self.families
    }

    pub fn prelude(&self) -> &[PreludeFile] {
        &self.prelude
    }

    pub fn node_kind(&self, kind: NodeKind) -> Option<&NodeKindDecl> {
        self.node_kinds.get(kind.index())
    }

    pub fn wire_kind(&self, kind: WireKind) -> Option<&WireKindDecl> {
        self.wire_kinds.get(kind.index())
    }

    pub fn enum_decl(&self, id: EnumId) -> Option<&EnumDecl> {
        self.enums.get(id.index())
    }

    pub fn family(&self, id: FamilyId) -> Option<&FamilyDecl> {
        self.families.get(id.index())
    }

    pub fn node_kind_at(&self, index: usize) -> Option<NodeKind> {
        (index < self.node_kinds.len()).then(|| NodeKind::new(self.token, index as u32))
    }

    pub fn wire_kind_at(&self, index: usize) -> Option<WireKind> {
        (index < self.wire_kinds.len()).then(|| WireKind::new(self.token, index as u32))
    }

    pub fn enum_at(&self, index: usize) -> Option<EnumId> {
        (index < self.enums.len()).then(|| EnumId::new(self.token, index as u32))
    }

    pub fn family_at(&self, index: usize) -> Option<FamilyId> {
        (index < self.families.len()).then(|| FamilyId::new(self.token, index as u32))
    }

    pub fn node_kind_named(&self, name: &str) -> Option<NodeKind> {
        let index = *self.node_names.get(name)?;
        Some(NodeKind::new(self.token, index))
    }

    pub fn wire_kind_named(&self, name: &str) -> Option<WireKind> {
        let index = *self.wire_names.get(name)?;
        Some(WireKind::new(self.token, index))
    }

    pub fn enum_named(&self, name: &str) -> Option<EnumId> {
        let index = *self.enum_names.get(name)?;
        Some(EnumId::new(self.token, index))
    }

    pub fn unit_named(&self, name: &str) -> Option<(FamilyId, usize)> {
        let (family, unit) = *self.unit_names.get(name)?;
        Some((FamilyId::new(self.token, family), unit))
    }
}
