use alloc::string::String;
use core::marker::PhantomData;

use super::attr_builder::AttrBuilder;
use super::attr_draft::AttrDraft;
use super::attr_type::AttrType;
use super::enum_attr::EnumAttr;
use super::owner::Owner;
use super::vocabulary_builder::VocabularyBuilder;
use crate::ids::{EnumId, NodeKind};

pub struct WireKindBuilder<'a> {
    builder: &'a mut VocabularyBuilder,
    wire: Option<usize>,
}

impl<'a> WireKindBuilder<'a> {
    pub(crate) fn new(builder: &'a mut VocabularyBuilder, wire: Option<usize>) -> Self {
        Self { builder, wire }
    }

    pub fn doc(self, text: &str) -> Self {
        if let Some(index) = self.wire {
            self.builder.wires[index].doc = Some(String::from(text));
        }
        self
    }

    pub fn connects(self, from: NodeKind, to: NodeKind) -> Self {
        if let Some(index) = self.wire {
            let referrer = Some(Owner::Wire(index));
            let from_own = self.builder.own_node(from, referrer).is_some();
            let to_own = self.builder.own_node(to, referrer).is_some();
            let connects = &mut self.builder.wires[index].connects;
            if from_own && to_own && !connects.contains(&(from, to)) {
                connects.push((from, to));
            }
        }
        self
    }

    pub fn cyclic(self) -> Self {
        if let Some(index) = self.wire {
            self.builder.wires[index].cyclic = true;
        }
        self
    }

    pub fn attr<T: AttrType>(self, name: &str) -> AttrBuilder<'a, T> {
        let owner = self.wire.map(Owner::Wire);
        let attr = self.builder.push_attr(owner, AttrDraft::new(name, T::ty()));
        AttrBuilder::new(self.builder, owner, attr, PhantomData)
    }

    pub fn attr_enum(
        self,
        name: &str,
        enumeration: impl Into<EnumId>,
        default_variant: &str,
    ) -> AttrBuilder<'a, EnumAttr> {
        let owner = self.wire.map(Owner::Wire);
        let attr =
            self.builder
                .push_enum_attr(owner, name, enumeration.into(), None, default_variant);
        AttrBuilder::new(self.builder, owner, attr, PhantomData)
    }

    pub fn attr_enum_array(
        self,
        name: &str,
        enumeration: impl Into<EnumId>,
        length: u32,
        default_variant: &str,
    ) -> AttrBuilder<'a, EnumAttr> {
        let owner = self.wire.map(Owner::Wire);
        let attr = self.builder.push_enum_attr(
            owner,
            name,
            enumeration.into(),
            Some(length),
            default_variant,
        );
        AttrBuilder::new(self.builder, owner, attr, PhantomData)
    }
}
