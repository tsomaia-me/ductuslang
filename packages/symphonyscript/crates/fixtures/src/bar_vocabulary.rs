use symphonyscript_core::{Count, Vocabulary};

pub fn bar_vocabulary() -> Vocabulary {
    let mut v = Vocabulary::builder("bars");
    let beats = v.unit_family("beats").base("beats").unit("bars", 4.0);
    let bar = v.node("Bar");
    v.kind(bar)
        .top_level(Count::Any)
        .attr::<i64>("bar_len")
        .default(0)
        .family(&beats, "bars")
        .attr::<i64>("beat_len")
        .default(0)
        .family(&beats, "beats")
        .attr::<i64>("plain")
        .default(0);
    v.build().expect("the bar vocabulary builds")
}
