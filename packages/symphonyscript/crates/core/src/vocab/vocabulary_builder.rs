use alloc::rc::Rc;
use alloc::string::String;
use alloc::vec::Vec;
use core::cell::RefCell;

use super::attr_draft::AttrDraft;
use super::enum_builder::EnumBuilder;
use super::enum_decl::EnumDecl;
use super::family_decl::FamilyDecl;
use super::invalid_vocabulary::InvalidVocabulary;
use super::native_decl::NativeDecl;
use super::native_fn::NativeFn;
use super::native_signature::NativeSignature;
use super::native_type::NativeType;
use super::node_draft::NodeDraft;
use super::node_kind_builder::NodeKindBuilder;
use super::owner::Owner;
use super::prelude_file::PreludeFile;
use super::unit_family_start::UnitFamilyStart;
use super::validation::validate;
use super::vocabulary::Vocabulary;
use super::vocabulary_error::VocabularyError;
use super::wire_draft::WireDraft;
use super::wire_kind_builder::WireKindBuilder;
use crate::ids::{EnumId, FamilyId, NativeId, NodeKind, VocabularyToken, WireKind};
use crate::types::{Scalar, Ty};

pub struct VocabularyBuilder {
    pub(crate) token: VocabularyToken,
    pub(crate) name: String,
    pub(crate) nodes: Vec<NodeDraft>,
    pub(crate) wires: Vec<WireDraft>,
    pub(crate) enums: Vec<Rc<RefCell<EnumDecl>>>,
    pub(crate) families: Vec<Rc<RefCell<FamilyDecl>>>,
    pub(crate) prelude: Vec<PreludeFile>,
    pub(crate) natives: Vec<NativeDecl>,
    pub(crate) errors: Vec<VocabularyError>,
}

impl VocabularyBuilder {
    pub(crate) fn new(name: &str) -> Self {
        Self {
            token: VocabularyToken::fresh(),
            name: String::from(name),
            nodes: Vec::new(),
            wires: Vec::new(),
            enums: Vec::new(),
            families: Vec::new(),
            prelude: Vec::new(),
            natives: Vec::new(),
            errors: Vec::new(),
        }
    }

    pub fn node(&mut self, name: &str) -> NodeKind {
        let kind = NodeKind::new(self.token, self.nodes.len() as u32);
        self.nodes.push(NodeDraft {
            name: String::from(name),
            doc: None,
            top_level: None,
            entry: false,
            children: Vec::new(),
            incoming: Vec::new(),
            outgoing: Vec::new(),
            attrs: Vec::new(),
        });
        kind
    }

    pub fn wire(&mut self, name: &str) -> WireKind {
        let kind = WireKind::new(self.token, self.wires.len() as u32);
        self.wires.push(WireDraft {
            name: String::from(name),
            doc: None,
            connects: Vec::new(),
            cyclic: false,
            attrs: Vec::new(),
        });
        kind
    }

    pub fn kind(&mut self, kind: NodeKind) -> NodeKindBuilder<'_> {
        let node = self.own_node(kind, None);
        NodeKindBuilder::new(self, node)
    }

    pub fn wire_kind(&mut self, kind: WireKind) -> WireKindBuilder<'_> {
        let wire = self.own_wire(kind, None);
        WireKindBuilder::new(self, wire)
    }

    pub fn enumeration<I, S>(&mut self, name: &str, variants: I) -> EnumBuilder
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let id = EnumId::new(self.token, self.enums.len() as u32);
        let draft = Rc::new(RefCell::new(EnumDecl {
            name: String::from(name),
            variants: variants
                .into_iter()
                .map(|variant| String::from(variant.as_ref()))
                .collect(),
            doc: None,
        }));
        self.enums.push(Rc::clone(&draft));
        EnumBuilder::new(draft, id)
    }

    pub fn unit_family(&mut self, name: &str) -> UnitFamilyStart<'_> {
        UnitFamilyStart::new(self, name)
    }

    pub fn prelude(&mut self, path: &str, text: &str) {
        self.prelude.push(PreludeFile {
            path: String::from(path),
            text: String::from(text),
        });
    }

    pub fn native(
        &mut self,
        name: &str,
        signature: NativeSignature,
        function: NativeFn,
    ) -> NativeId {
        let id = NativeId::new(
            self.token,
            u32::try_from(self.natives.len()).unwrap_or(u32::MAX),
        );
        let types = signature
            .params
            .iter()
            .chain(core::iter::once(&signature.result));
        let enums: Vec<EnumId> = types
            .filter_map(|ty| match ty {
                NativeType::Value(ty) | NativeType::Open(ty) | NativeType::CapacityOf(ty, _) => {
                    match ty.scalar {
                        Scalar::Enum(id) => Some(id),
                        _ => None,
                    }
                }
            })
            .collect();
        for enumeration in enums {
            self.own_enum(enumeration, None);
        }
        self.natives.push(NativeDecl {
            name: String::from(name),
            signature,
            function,
        });
        id
    }

    pub fn build(self) -> Result<Vocabulary, InvalidVocabulary> {
        validate(self)
    }

    pub(crate) fn own_node(&mut self, kind: NodeKind, referrer: Option<Owner>) -> Option<usize> {
        if kind.vocabulary() == self.token && kind.index() < self.nodes.len() {
            Some(kind.index())
        } else {
            self.foreign(referrer);
            None
        }
    }

    pub(crate) fn own_wire(&mut self, kind: WireKind, referrer: Option<Owner>) -> Option<usize> {
        if kind.vocabulary() == self.token && kind.index() < self.wires.len() {
            Some(kind.index())
        } else {
            self.foreign(referrer);
            None
        }
    }

    pub(crate) fn own_enum(&mut self, id: EnumId, referrer: Option<Owner>) -> bool {
        let own = id.vocabulary() == self.token && id.index() < self.enums.len();
        if !own {
            self.foreign(referrer);
        }
        own
    }

    pub(crate) fn own_family(&mut self, id: FamilyId, referrer: Option<Owner>) -> bool {
        let own = id.vocabulary() == self.token && id.index() < self.families.len();
        if !own {
            self.foreign(referrer);
        }
        own
    }

    fn foreign(&mut self, referrer: Option<Owner>) {
        let referrer = referrer.map(|owner| self.owner_name(owner));
        self.errors
            .push(VocabularyError::ForeignHandle { referrer });
    }

    pub(crate) fn owner_name(&self, owner: Owner) -> String {
        match owner {
            Owner::Node(index) => self.nodes[index].name.clone(),
            Owner::Wire(index) => self.wires[index].name.clone(),
        }
    }

    pub(crate) fn push_attr(&mut self, owner: Option<Owner>, draft: AttrDraft) -> usize {
        let attrs = match owner {
            Some(Owner::Node(index)) => &mut self.nodes[index].attrs,
            Some(Owner::Wire(index)) => &mut self.wires[index].attrs,
            None => return 0,
        };
        attrs.push(draft);
        attrs.len() - 1
    }

    pub(crate) fn attr_mut(&mut self, owner: Option<Owner>, attr: usize) -> Option<&mut AttrDraft> {
        match owner {
            Some(Owner::Node(index)) => self.nodes[index].attrs.get_mut(attr),
            Some(Owner::Wire(index)) => self.wires[index].attrs.get_mut(attr),
            None => None,
        }
    }

    pub(crate) fn push_enum_attr(
        &mut self,
        owner: Option<Owner>,
        name: &str,
        enumeration: EnumId,
        length: Option<u32>,
        default_variant: &str,
    ) -> usize {
        if owner.is_none() {
            return 0;
        }
        let own = self.own_enum(enumeration, owner);
        let ty = match length {
            Some(length) => Ty::of(Scalar::Enum(enumeration)).array(length),
            None => Ty::of(Scalar::Enum(enumeration)),
        };
        let mut draft = AttrDraft::new(name, ty);
        if own {
            draft.enum_default = Some((enumeration, String::from(default_variant)));
        }
        self.push_attr(owner, draft)
    }
}
