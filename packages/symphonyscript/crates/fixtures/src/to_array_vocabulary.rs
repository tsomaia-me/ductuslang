use symphonyscript_core::{Bounded, Count, Vocabulary};

pub fn to_array_vocabulary() -> Vocabulary {
    let mut v = Vocabulary::builder("to-arrays");
    let clip = v.node("Clip");
    let lane = v.node("Lane");
    let note = v.node("Note");
    let plays = v.wire("Plays");
    let cues = v.wire("Cues");
    v.kind(clip)
        .top_level(Count::Any)
        .children(note, Count::Any)
        .attr::<f32>("gain")
        .default(1.0)
        .attr::<[f32; 3]>("steps")
        .default([0.0, 0.5, 1.0])
        .attr::<Bounded<i32, 4>>("seq")
        .default(Bounded::default());
    v.kind(lane)
        .top_level(Count::Any)
        .attr::<[f32; 2]>("steps")
        .default([0.0, 1.0])
        .attr::<Bounded<i32, 8>>("seq")
        .default(Bounded::default());
    v.kind(note).attr::<i32>("pitch").default(60).default_attr();
    v.wire_kind(plays)
        .connects(clip, clip)
        .cyclic()
        .attr::<f32>("gain")
        .default(1.0)
        .default_attr()
        .attr::<[f32; 3]>("trio")
        .default([0.0, 0.0, 0.0])
        .attr::<Bounded<i32, 2>>("picks")
        .default(Bounded::default());
    v.wire_kind(cues)
        .connects(clip, clip)
        .connects(clip, lane)
        .cyclic()
        .attr::<f32>("gain")
        .default(1.0)
        .default_attr();
    v.build().expect("the to-array vocabulary builds")
}
