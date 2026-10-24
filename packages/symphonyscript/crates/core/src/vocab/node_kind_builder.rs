use alloc::string::String;
use core::marker::PhantomData;

use super::attr_builder::AttrBuilder;
use super::attr_draft::AttrDraft;
use super::attr_type::AttrType;
use super::count::Count;
use super::enum_attr::EnumAttr;
use super::owner::Owner;
use super::vocabulary_builder::VocabularyBuilder;
use crate::ids::{EnumId, NodeKind, WireKind};

pub struct NodeKindBuilder<'a> {
    builder: &'a mut VocabularyBuilder,
    node: Option<usize>,
}

impl<'a> NodeKindBuilder<'a> {
    pub(crate) fn new(builder: &'a mut VocabularyBuilder, node: Option<usize>) -> Self {
        Self { builder, node }
    }

    pub fn doc(self, text: &str) -> Self {
        if let Some(index) = self.node {
            self.builder.nodes[index].doc = Some(String::from(text));
        }
        self
    }

    pub fn top_level(self, count: Count) -> Self {
        if let Some(index) = self.node {
            self.builder.nodes[index].top_level = Some(count);
        }
        self
    }

    pub fn entry(self) -> Self {
        if let Some(index) = self.node {
            self.builder.nodes[index].entry = true;
        }
        self
    }

    pub fn children(self, kind: NodeKind, count: Count) -> Self {
        if let Some(index) = self.node
            && self
                .builder
                .own_node(kind, Some(Owner::Node(index)))
                .is_some()
        {
            let children = &mut self.builder.nodes[index].children;
            match children.iter_mut().find(|(child, _)| *child == kind) {
                Some(entry) => entry.1 = count,
                None => children.push((kind, count)),
            }
        }
        self
    }

    pub fn incoming(self, wire: WireKind, count: Count) -> Self {
        if let Some(index) = self.node
            && self
                .builder
                .own_wire(wire, Some(Owner::Node(index)))
                .is_some()
        {
            let incoming = &mut self.builder.nodes[index].incoming;
            match incoming.iter_mut().find(|(kind, _)| *kind == wire) {
                Some(entry) => entry.1 = count,
                None => incoming.push((wire, count)),
            }
        }
        self
    }

    pub fn outgoing(self, wire: WireKind, count: Count) -> Self {
        if let Some(index) = self.node
            && self
                .builder
                .own_wire(wire, Some(Owner::Node(index)))
                .is_some()
        {
            let outgoing = &mut self.builder.nodes[index].outgoing;
            match outgoing.iter_mut().find(|(kind, _)| *kind == wire) {
                Some(entry) => entry.1 = count,
                None => outgoing.push((wire, count)),
            }
        }
        self
    }

    pub fn attr<T: AttrType>(self, name: &str) -> AttrBuilder<'a, T> {
        let owner = self.node.map(Owner::Node);
        let attr = self.builder.push_attr(owner, AttrDraft::new(name, T::ty()));
        AttrBuilder::new(self.builder, owner, attr, PhantomData)
    }

    pub fn attr_enum(
        self,
        name: &str,
        enumeration: impl Into<EnumId>,
        default_variant: &str,
    ) -> AttrBuilder<'a, EnumAttr> {
        let owner = self.node.map(Owner::Node);
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
        let owner = self.node.map(Owner::Node);
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
