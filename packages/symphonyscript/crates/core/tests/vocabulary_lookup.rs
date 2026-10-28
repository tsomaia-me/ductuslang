use symphonyscript_core::{
    Count, NativeArg, NativeOut, NativeSignature, NativeType, Ty, Vocabulary,
};

#[test]
fn handles_look_up_by_index_alone() {
    let mut a = Vocabulary::builder("a");
    let note = a.node("Note");
    let rest = a.node("Rest");
    let plays = a.wire("Plays");
    let wave = a.enumeration("Wave", ["Sine"]).id();
    let time = a.unit_family("time").base("s").id();
    let mut b = Vocabulary::builder("b");
    b.node("Clip");
    b.wire("Starts");
    b.enumeration("Shape", ["Round"]);
    b.unit_family("beats").base("beats");
    let a = a.build().expect("a is valid");
    let b = b.build().expect("b is valid");
    let name =
        |decl: Option<&symphonyscript_core::NodeKindDecl>| decl.map(|decl| decl.name.clone());
    assert_eq!(name(a.node_kind(note)).as_deref(), Some("Note"));
    assert_eq!(name(b.node_kind(note)).as_deref(), Some("Clip"));
    assert!(b.node_kind(rest).is_none());
    assert_eq!(
        b.wire_kind(plays).map(|decl| decl.name.as_str()),
        Some("Starts")
    );
    assert_eq!(
        b.enum_decl(wave).map(|decl| decl.name.as_str()),
        Some("Shape")
    );
    assert_eq!(b.family(time).map(|decl| decl.name.as_str()), Some("beats"));
    assert_eq!(b.node_kind_at(0), Some(note));
    assert!(b.node_kind_at(1).is_none());
    assert_eq!(b.wire_kind_at(0), Some(plays));
    assert_eq!(b.enum_at(0), Some(wave));
    assert_eq!(b.family_at(0), Some(time));
    assert!(b.unit_named("s").is_none());
}

#[test]
fn counts_bound_numbers() {
    assert!(Count::Exactly(2).contains(2));
    assert!(!Count::Exactly(2).contains(3));
    assert!(Count::AtLeast(1).contains(9));
    assert!(!Count::AtLeast(1).contains(0));
    assert!(Count::AtMost(1).contains(0));
    assert!(!Count::AtMost(1).contains(2));
    assert!(Count::Between(1, 3).contains(3));
    assert!(!Count::Between(1, 3).contains(4));
    assert!(Count::Any.contains(u32::MAX));
    assert_eq!(Count::Between(1, 3).min(), 1);
    assert_eq!(Count::AtLeast(4).max(), None);
}

fn zero(_: &[NativeArg<'_>], _: &mut NativeOut<'_>) {}

#[test]
fn a_native_id_looks_up_only_in_its_own_vocabulary() {
    let signature =
        || NativeSignature::new([NativeType::Value(Ty::I32)], NativeType::Value(Ty::I32));
    let mut a = Vocabulary::builder("a");
    let first = a.native("first", signature(), zero);
    let second = a.native("second", signature(), zero);
    let mut b = Vocabulary::builder("b");
    b.native("other", signature(), zero);
    let a = a.build().expect("a is valid");
    let b = b.build().expect("b is valid");
    let name = |vocabulary: &Vocabulary, id| vocabulary.native(id).map(|decl| decl.name.clone());
    assert_eq!(name(&a, first).as_deref(), Some("first"));
    assert_eq!(name(&a, second).as_deref(), Some("second"));
    assert_eq!(a.native_named("second"), Some(second));
    assert_eq!(a.native_at(1), Some(second));
    assert_eq!(a.native_at(2), None);
    assert_eq!(name(&b, first), None);
    assert_eq!(name(&b, second), None);
    let other = b.native_named("other").expect("b declares other");
    assert_eq!(name(&a, other), None);
}
