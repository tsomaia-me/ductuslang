use symphonyscript_core::{Count, Vocabulary};

pub fn envelope_vocabulary() -> Vocabulary {
    let mut v = Vocabulary::builder("envelope");
    let time = v.unit_family("time").base("s").unit("ms", 0.001);
    let envelope = v.node("Env");
    v.kind(envelope)
        .top_level(Count::Any)
        .attr::<f64>("attack")
        .default(0.0)
        .family(&time, "ms")
        .attr::<f32>("release")
        .default(0.0)
        .family(&time, "ms")
        .attr::<i32>("pitch")
        .default(0);
    v.build().expect("the envelope vocabulary builds")
}
