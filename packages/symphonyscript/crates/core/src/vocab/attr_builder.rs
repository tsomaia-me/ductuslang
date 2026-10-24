use alloc::string::String;
use core::marker::PhantomData;

use super::attr_draft::AttrDraft;
use super::attr_type::AttrType;
use super::enum_attr::EnumAttr;
use super::numeric::Numeric;
use super::owner::Owner;
use super::vocabulary_builder::VocabularyBuilder;
use crate::ids::{EnumId, FamilyId};

pub struct AttrBuilder<'a, T> {
    builder: &'a mut VocabularyBuilder,
    owner: Option<Owner>,
    attr: usize,
    marker: PhantomData<fn() -> T>,
}

impl<'a, T> AttrBuilder<'a, T> {
    pub(crate) fn new(
        builder: &'a mut VocabularyBuilder,
        owner: Option<Owner>,
        attr: usize,
        marker: PhantomData<fn() -> T>,
    ) -> Self {
        Self {
            builder,
            owner,
            attr,
            marker,
        }
    }

    pub fn default_attr(self) -> Self {
        if let Some(draft) = self.builder.attr_mut(self.owner, self.attr) {
            draft.is_default = true;
        }
        self
    }

    pub fn flag(self, flag: char) -> Self {
        if let Some(draft) = self.builder.attr_mut(self.owner, self.attr) {
            draft.flag = Some(flag);
        }
        self
    }

    pub fn doc(self, text: &str) -> Self {
        if let Some(draft) = self.builder.attr_mut(self.owner, self.attr) {
            draft.doc = Some(String::from(text));
        }
        self
    }

    pub fn attr<U: AttrType>(self, name: &str) -> AttrBuilder<'a, U> {
        let attr = self
            .builder
            .push_attr(self.owner, AttrDraft::new(name, U::ty()));
        AttrBuilder::new(self.builder, self.owner, attr, PhantomData)
    }

    pub fn attr_enum(
        self,
        name: &str,
        enumeration: impl Into<EnumId>,
        default_variant: &str,
    ) -> AttrBuilder<'a, EnumAttr> {
        let attr = self.builder.push_enum_attr(
            self.owner,
            name,
            enumeration.into(),
            None,
            default_variant,
        );
        AttrBuilder::new(self.builder, self.owner, attr, PhantomData)
    }

    pub fn attr_enum_array(
        self,
        name: &str,
        enumeration: impl Into<EnumId>,
        length: u32,
        default_variant: &str,
    ) -> AttrBuilder<'a, EnumAttr> {
        let attr = self.builder.push_enum_attr(
            self.owner,
            name,
            enumeration.into(),
            Some(length),
            default_variant,
        );
        AttrBuilder::new(self.builder, self.owner, attr, PhantomData)
    }
}

impl<T: AttrType> AttrBuilder<'_, T> {
    pub fn default(self, value: T) -> Self {
        if let Some(draft) = self.builder.attr_mut(self.owner, self.attr) {
            draft.default = Some(value.into_default());
        }
        self
    }
}

impl<T: Numeric> AttrBuilder<'_, T> {
    pub fn family(self, family: impl Into<FamilyId>, default_unit: &str) -> Self {
        let family = family.into();
        if self.owner.is_some()
            && self.builder.own_family(family, self.owner)
            && let Some(draft) = self.builder.attr_mut(self.owner, self.attr)
        {
            draft.unit = Some((family, String::from(default_unit)));
        }
        self
    }
}
