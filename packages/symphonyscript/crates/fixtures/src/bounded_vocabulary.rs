use symphonyscript_core::{Count, Vocabulary};

pub fn bounded_vocabulary() -> Vocabulary {
    let mut v = Vocabulary::builder("bounded");
    let beats = v.unit_family("beats").base("beats").unit("bars", 4.0);
    let root = v.node("Root");
    let item = v.node("Item");
    let first = v.node("A");
    let second = v.node("B");
    let link = v.wire("Link");
    let pick = v.wire("Pick");
    v.kind(root)
        .top_level(Count::Exactly(1))
        .entry()
        .children(item, Count::Between(1, 2))
        .outgoing(link, Count::AtMost(1))
        .incoming(link, Count::AtLeast(1));
    v.kind(item);
    v.kind(first)
        .top_level(Count::Any)
        .attr::<i32>("x")
        .default(0)
        .attr::<f64>("long")
        .default(0.0)
        .family(&beats, "bars");
    v.kind(second)
        .top_level(Count::Any)
        .attr::<f32>("x")
        .default(0.0)
        .attr::<f64>("short")
        .default(0.0)
        .family(&beats, "beats");
    v.wire_kind(link).connects(root, root).cyclic();
    v.wire_kind(pick)
        .connects(root, first)
        .connects(root, second)
        .attr::<f32>("level")
        .default(1.0);
    v.build().expect("the bounded vocabulary builds")
}
