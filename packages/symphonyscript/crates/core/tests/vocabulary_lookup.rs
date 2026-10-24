use symphonyscript_core::{Count, Vocabulary};

#[test]
fn handles_of_another_vocabulary_find_nothing() {
    let mut a = Vocabulary::builder("a");
    let note = a.node("Note");
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
    assert!(a.node_kind(note).is_some());
    assert!(b.node_kind(note).is_none());
    assert!(b.wire_kind(plays).is_none());
    assert!(b.enum_decl(wave).is_none());
    assert!(b.family(time).is_none());
    assert_eq!(
        b.node_kind_at(0)
            .and_then(|kind| b.node_kind(kind))
            .map(|decl| decl.name.as_str()),
        Some("Clip")
    );
    assert!(b.node_kind_at(1).is_none());
    assert!(b.wire_kind_at(0).is_some());
    assert!(b.enum_at(0).is_some());
    assert!(b.family_at(0).is_some());
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
