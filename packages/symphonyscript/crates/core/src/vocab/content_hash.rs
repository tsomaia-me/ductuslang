use super::attr_decl::AttrDecl;
use super::attr_default::AttrDefault;
use super::count::Count;
use super::enum_decl::EnumDecl;
use super::family_decl::FamilyDecl;
use super::fnv::Fnv;
use super::node_kind_decl::NodeKindDecl;
use super::prelude_file::PreludeFile;
use super::wire_kind_decl::WireKindDecl;
use crate::ids::AttrId;
use crate::types::{Ty, Value};

pub(crate) fn content_hash(
    node_kinds: &[NodeKindDecl],
    wire_kinds: &[WireKindDecl],
    enums: &[EnumDecl],
    families: &[FamilyDecl],
    prelude: &[PreludeFile],
) -> u64 {
    let mut fnv = Fnv::new();
    fnv.str("symphonyscript-vocabulary-1");
    fnv.len(node_kinds.len());
    for kind in node_kinds {
        fnv.str(&kind.name);
        attrs(&mut fnv, &kind.attrs, kind.default_attr);
        match kind.top_level {
            Some(count) => {
                fnv.byte(1);
                self::count(&mut fnv, count);
            }
            None => fnv.byte(0),
        }
        fnv.byte(u8::from(kind.entry));
        fnv.len(kind.children.len());
        for (child, count) in &kind.children {
            fnv.len(child.index());
            self::count(&mut fnv, *count);
        }
        for list in [&kind.incoming, &kind.outgoing] {
            fnv.len(list.len());
            for (wire, count) in list {
                fnv.len(wire.index());
                self::count(&mut fnv, *count);
            }
        }
    }
    fnv.len(wire_kinds.len());
    for kind in wire_kinds {
        fnv.str(&kind.name);
        attrs(&mut fnv, &kind.attrs, kind.default_attr);
        fnv.len(kind.connects.len());
        for (from, to) in &kind.connects {
            fnv.len(from.index());
            fnv.len(to.index());
        }
        fnv.byte(u8::from(kind.cyclic));
    }
    fnv.len(enums.len());
    for decl in enums {
        fnv.str(&decl.name);
        fnv.len(decl.variants.len());
        for variant in &decl.variants {
            fnv.str(variant);
        }
    }
    fnv.len(families.len());
    for family in families {
        fnv.str(&family.name);
        fnv.len(family.units.len());
        for unit in &family.units {
            fnv.str(&unit.name);
            fnv.u64(unit.scale.to_bits());
        }
    }
    fnv.len(prelude.len());
    for file in prelude {
        fnv.str(&file.text.replace("\r\n", "\n"));
    }
    fnv.finish()
}

fn attrs(fnv: &mut Fnv, attrs: &[AttrDecl], default_attr: Option<AttrId>) {
    fnv.len(attrs.len());
    for attr in attrs {
        fnv.str(&attr.name);
        ty(fnv, &attr.ty);
        match &attr.default {
            None => fnv.byte(0),
            Some(AttrDefault::Elements(elements)) => {
                fnv.byte(1);
                fnv.len(elements.len());
                for element in elements {
                    value(fnv, *element);
                }
            }
            Some(AttrDefault::Text(text)) => {
                fnv.byte(2);
                fnv.str(text);
            }
        }
        match attr.unit {
            None => fnv.byte(0),
            Some(unit) => {
                fnv.byte(1);
                fnv.len(unit.family.index());
                fnv.len(unit.default_unit);
            }
        }
        match attr.flag {
            None => fnv.byte(0),
            Some(flag) => {
                fnv.byte(1);
                fnv.u32(u32::from(flag));
            }
        }
    }
    match default_attr {
        None => fnv.byte(0),
        Some(attr) => {
            fnv.byte(1);
            fnv.u32(attr.0);
        }
    }
}

fn ty(fnv: &mut Fnv, ty: &Ty) {
    match ty {
        Ty::I32 => fnv.byte(0),
        Ty::I64 => fnv.byte(1),
        Ty::F32 => fnv.byte(2),
        Ty::F64 => fnv.byte(3),
        Ty::Bool => fnv.byte(4),
        Ty::Enum(id) => {
            fnv.byte(5);
            fnv.len(id.index());
        }
        Ty::Str => fnv.byte(6),
        Ty::Array(element, length) => {
            fnv.byte(7);
            fnv.u32(*length);
            self::ty(fnv, element);
        }
    }
}

fn value(fnv: &mut Fnv, value: Value) {
    match value {
        Value::I32(v) => {
            fnv.byte(0);
            fnv.bytes(&v.to_le_bytes());
        }
        Value::I64(v) => {
            fnv.byte(1);
            fnv.bytes(&v.to_le_bytes());
        }
        Value::F32(v) => {
            fnv.byte(2);
            fnv.u32(v.to_bits());
        }
        Value::F64(v) => {
            fnv.byte(3);
            fnv.u64(v.to_bits());
        }
        Value::Bool(v) => {
            fnv.byte(4);
            fnv.byte(u8::from(v));
        }
        Value::Enum(id, variant) => {
            fnv.byte(5);
            fnv.len(id.index());
            fnv.bytes(&variant.to_le_bytes());
        }
        Value::Str(id) => {
            fnv.byte(6);
            fnv.u32(id.0);
        }
    }
}

fn count(fnv: &mut Fnv, count: Count) {
    match count {
        Count::Exactly(n) => {
            fnv.byte(0);
            fnv.u32(n);
        }
        Count::AtLeast(n) => {
            fnv.byte(1);
            fnv.u32(n);
        }
        Count::AtMost(n) => {
            fnv.byte(2);
            fnv.u32(n);
        }
        Count::Between(n, m) => {
            fnv.byte(3);
            fnv.u32(n);
            fnv.u32(m);
        }
        Count::Any => fnv.byte(4),
    }
}
