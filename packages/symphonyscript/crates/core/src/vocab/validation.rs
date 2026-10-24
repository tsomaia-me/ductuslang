use alloc::collections::BTreeSet;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use super::attr_decl::AttrDecl;
use super::attr_default::AttrDefault;
use super::attr_draft::AttrDraft;
use super::attr_unit::AttrUnit;
use super::content_hash::content_hash;
use super::count::Count;
use super::enum_decl::EnumDecl;
use super::family_decl::FamilyDecl;
use super::invalid_vocabulary::InvalidVocabulary;
use super::node_kind_decl::NodeKindDecl;
use super::vocabulary::Vocabulary;
use super::vocabulary_builder::VocabularyBuilder;
use super::vocabulary_error::VocabularyError;
use super::vocabulary_stamp::VocabularyStamp;
use super::wire_kind_decl::WireKindDecl;
use crate::ids::AttrId;
use crate::language::{BUILTINS, FLAG_CHARS, KEYWORDS, RESERVED_WORDS};
use crate::types::{Scalar, Ty, Value};

pub(crate) fn validate(builder: VocabularyBuilder) -> Result<Vocabulary, InvalidVocabulary> {
    let enums: Vec<EnumDecl> = builder
        .enums
        .iter()
        .map(|decl| decl.borrow().clone())
        .collect();
    let families: Vec<FamilyDecl> = builder
        .families
        .iter()
        .map(|decl| decl.borrow().clone())
        .collect();
    let mut errors = builder.errors;
    let variants: BTreeSet<&str> = enums
        .iter()
        .flat_map(|decl| decl.variants.iter().map(String::as_str))
        .collect();
    let mut check = Checker {
        variants,
        enums: &enums,
        families: &families,
        errors: &mut errors,
    };

    for node in &builder.nodes {
        check.name(&node.name);
        check.attrs(&node.name, &node.attrs);
        if let Some(count) = node.top_level {
            check.count(&node.name, count);
        }
        for (_, count) in &node.children {
            check.count(&node.name, *count);
        }
        for (_, count) in node.incoming.iter().chain(&node.outgoing) {
            check.count(&node.name, *count);
        }
    }
    for wire in &builder.wires {
        check.name(&wire.name);
        check.attrs(&wire.name, &wire.attrs);
    }
    for decl in &enums {
        check.name(&decl.name);
        let mut variants = BTreeSet::new();
        for variant in &decl.variants {
            check.word(variant);
            if !variants.insert(variant.as_str()) {
                check.errors.push(VocabularyError::DuplicateVariant {
                    enumeration: decl.name.clone(),
                    variant: variant.clone(),
                });
            }
        }
        if decl.variants.is_empty() {
            check.errors.push(VocabularyError::EmptyEnum {
                name: decl.name.clone(),
            });
        } else if decl.variants.len() > 65_536 {
            check.errors.push(VocabularyError::EnumTooLarge {
                name: decl.name.clone(),
                variants: decl.variants.len(),
            });
        }
    }
    let mut units = BTreeSet::new();
    for family in &families {
        check.name(&family.name);
        for (position, unit) in family.units.iter().enumerate() {
            check.name(&unit.name);
            if looks_numeric(&unit.name) {
                check.errors.push(VocabularyError::NumericUnitName {
                    unit: unit.name.clone(),
                });
            }
            if position > 0 && !(unit.scale > 0.0 && unit.scale.is_finite()) {
                check.errors.push(VocabularyError::InvalidScale {
                    unit: unit.name.clone(),
                    scale: unit.scale,
                });
            }
            if !units.insert(unit.name.as_str()) {
                check.errors.push(VocabularyError::RepeatedUnit {
                    unit: unit.name.clone(),
                });
            }
        }
    }
    let mut declared = BTreeSet::new();
    let declarations = builder
        .nodes
        .iter()
        .map(|node| &node.name)
        .chain(builder.wires.iter().map(|wire| &wire.name))
        .chain(enums.iter().map(|decl| &decl.name))
        .chain(families.iter().map(|family| &family.name));
    for name in declarations {
        if !declared.insert(name.as_str()) {
            check
                .errors
                .push(VocabularyError::DuplicateName { name: name.clone() });
        }
    }

    if !errors.is_empty() {
        return Err(InvalidVocabulary { errors });
    }

    let node_kinds: Vec<NodeKindDecl> = builder
        .nodes
        .into_iter()
        .map(|node| {
            let (attrs, default_attr) = assemble_attrs(node.attrs, &enums, &families);
            let mut children = node.children;
            children.sort_by_key(|(kind, _)| kind.index());
            let mut incoming = node.incoming;
            incoming.sort_by_key(|(kind, _)| kind.index());
            let mut outgoing = node.outgoing;
            outgoing.sort_by_key(|(kind, _)| kind.index());
            NodeKindDecl {
                name: node.name,
                attrs,
                default_attr,
                top_level: node.top_level,
                entry: node.entry,
                children,
                incoming,
                outgoing,
                doc: node.doc,
            }
        })
        .collect();
    let wire_kinds: Vec<WireKindDecl> = builder
        .wires
        .into_iter()
        .map(|wire| {
            let (attrs, default_attr) = assemble_attrs(wire.attrs, &enums, &families);
            let mut connects = wire.connects;
            connects.sort_by_key(|(from, to)| (from.index(), to.index()));
            WireKindDecl {
                name: wire.name,
                attrs,
                default_attr,
                connects,
                cyclic: wire.cyclic,
                doc: wire.doc,
            }
        })
        .collect();
    let hash = content_hash(
        &node_kinds,
        &wire_kinds,
        &enums,
        &families,
        &builder.prelude,
    );
    let stamp = VocabularyStamp {
        name: builder.name,
        hash,
    };
    Ok(Vocabulary::assemble(
        builder.token,
        stamp,
        node_kinds,
        wire_kinds,
        enums,
        families,
        builder.prelude,
    ))
}

struct Checker<'a> {
    variants: BTreeSet<&'a str>,
    enums: &'a [EnumDecl],
    families: &'a [FamilyDecl],
    errors: &'a mut Vec<VocabularyError>,
}

impl Checker<'_> {
    fn word(&mut self, name: &str) -> bool {
        let error = if !is_ascii_identifier(name) {
            VocabularyError::NotAsciiIdentifier {
                name: String::from(name),
            }
        } else if KEYWORDS.contains(&name) || RESERVED_WORDS.contains(&name) {
            VocabularyError::ReservedName {
                name: String::from(name),
            }
        } else if BUILTINS.contains(&name) {
            VocabularyError::BuiltinName {
                name: String::from(name),
            }
        } else {
            return true;
        };
        self.errors.push(error);
        false
    }

    fn name(&mut self, name: &str) {
        if self.word(name) && self.variants.contains(name) {
            self.errors.push(VocabularyError::VariantName {
                name: String::from(name),
            });
        }
    }

    fn count(&mut self, owner: &str, count: Count) {
        if let Count::Between(min, max) = count
            && min > max
        {
            self.errors.push(VocabularyError::CountRange {
                owner: String::from(owner),
                count,
            });
        }
    }

    fn attrs(&mut self, owner: &str, attrs: &[AttrDraft]) {
        let mut names = BTreeSet::new();
        let mut flags = BTreeSet::new();
        for attr in attrs {
            self.name(&attr.name);
            if !names.insert(attr.name.as_str()) {
                self.errors.push(VocabularyError::DuplicateAttribute {
                    owner: String::from(owner),
                    name: attr.name.clone(),
                });
            }
            if has_empty_array(&attr.ty) {
                self.errors.push(VocabularyError::EmptyArrayAttribute {
                    owner: String::from(owner),
                    attribute: attr.name.clone(),
                });
            }
            if let Some(flag) = attr.flag {
                if !FLAG_CHARS.contains(&flag) {
                    self.errors.push(VocabularyError::InvalidFlag {
                        owner: String::from(owner),
                        attribute: attr.name.clone(),
                        flag,
                    });
                }
                if attr.ty != Ty::BOOL {
                    self.errors.push(VocabularyError::FlagOnNonBool {
                        owner: String::from(owner),
                        attribute: attr.name.clone(),
                    });
                }
                if !flags.insert(flag) {
                    self.errors.push(VocabularyError::DuplicateFlag {
                        owner: String::from(owner),
                        flag,
                    });
                }
            }
            if let Some((id, variant)) = &attr.enum_default
                && !self.enums[id.index()].variants.contains(variant)
            {
                self.errors.push(VocabularyError::UnknownDefaultVariant {
                    owner: String::from(owner),
                    attribute: attr.name.clone(),
                    variant: variant.clone(),
                });
            }
            if let Some((id, unit)) = &attr.unit {
                let family = &self.families[id.index()];
                if !family.units.iter().any(|decl| decl.name == *unit) {
                    self.errors.push(VocabularyError::UnknownDefaultUnit {
                        owner: String::from(owner),
                        attribute: attr.name.clone(),
                        unit: unit.clone(),
                    });
                }
                if is_integer(&attr.ty)
                    && let Some(fractional) =
                        family.units.iter().find(|decl| !is_integral(decl.scale))
                {
                    self.errors.push(VocabularyError::FractionalScale {
                        owner: String::from(owner),
                        attribute: attr.name.clone(),
                        unit: fractional.name.clone(),
                    });
                }
            }
        }
        if attrs.iter().filter(|attr| attr.is_default).count() > 1 {
            self.errors
                .push(VocabularyError::DuplicateDefaultAttribute {
                    owner: String::from(owner),
                });
        }
    }
}

fn assemble_attrs(
    drafts: Vec<AttrDraft>,
    enums: &[EnumDecl],
    families: &[FamilyDecl],
) -> (Vec<AttrDecl>, Option<AttrId>) {
    let default_attr = drafts
        .iter()
        .position(|attr| attr.is_default)
        .map(|position| AttrId(position as u32));
    let attrs = drafts
        .into_iter()
        .map(|draft| {
            let default = match &draft.enum_default {
                Some((id, variant)) => {
                    let index = enums[id.index()]
                        .variants
                        .iter()
                        .position(|name| name == variant)
                        .unwrap_or(0);
                    Some(AttrDefault::Elements(vec![
                        Value::Enum(*id, index as u16);
                        draft.ty.element_count()
                    ]))
                }
                None => draft.default,
            };
            let unit = draft.unit.map(|(family, unit)| AttrUnit {
                family,
                default_unit: families[family.index()]
                    .units
                    .iter()
                    .position(|decl| decl.name == unit)
                    .unwrap_or(0),
            });
            AttrDecl {
                name: draft.name,
                ty: draft.ty,
                default,
                unit,
                flag: draft.flag,
                doc: draft.doc,
            }
        })
        .collect();
    (attrs, default_attr)
}

fn has_empty_array(ty: &Ty) -> bool {
    ty.lengths.contains(&0)
}

fn is_integer(ty: &Ty) -> bool {
    matches!(ty.scalar, Scalar::I32 | Scalar::I64)
}

fn is_integral(scale: f64) -> bool {
    scale >= 9_007_199_254_740_992.0 || (scale as u64) as f64 == scale
}

fn is_ascii_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || first == '_' || first == '#')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '#')
        && name.chars().any(|c| c != '_' && c != '#')
}

fn looks_numeric(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if first == '_' {
        return true;
    }
    let Some(second) = chars.next() else {
        return false;
    };
    match first {
        'x' => second.is_ascii_hexdigit(),
        'b' => second == '0' || second == '1',
        'o' => ('0'..='7').contains(&second),
        'e' | 'E' => second.is_ascii_digit() || second == '+' || second == '-',
        _ => false,
    }
}
