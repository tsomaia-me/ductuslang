use symphonyscript_core::{AttrDefault, Count, InvalidVocabulary, Ty, Value, Vocabulary};

fn example() -> Result<Vocabulary, InvalidVocabulary> {
    let mut v = Vocabulary::builder("music");

    let beats = v.unit_family("beats").base("beats").unit("bars", 4.0);
    let wave = v.enumeration("Wave", ["Sine", "Saw", "Square"]);

    let note = v.node("Note");
    let clip = v.node("Clip");
    let track = v.node("Track");
    let osc = v.node("Osc");
    let session = v.node("Session");
    let plays = v.wire("Plays");
    let starts = v.wire("Starts");

    v.kind(note)
        .doc("A single pitched event inside a clip.")
        .attr::<i32>("pitch")
        .default(60)
        .default_attr()
        .attr::<f32>("velocity")
        .default(1.0)
        .doc("Loudness, 0 to 1.")
        .attr::<f32>("brightness")
        .default(0.5)
        .attr::<f64>("length")
        .default(0.25)
        .family(beats, "beats")
        .attr::<bool>("accent")
        .default(false)
        .flag('\'');

    v.kind(clip)
        .top_level(Count::Any)
        .children(note, Count::Any)
        .attr::<f32>("gain")
        .default(1.0)
        .attr::<f32>("level")
        .default(1.0);

    v.kind(track)
        .top_level(Count::Any)
        .attr::<&'static str>("label")
        .default("");

    v.kind(osc)
        .top_level(Count::Any)
        .attr::<f64>("freq")
        .default(440.0)
        .default_attr()
        .attr_enum("wave", wave, "Sine")
        .attr::<f32>("rate")
        .default(1.0);

    v.kind(session)
        .top_level(Count::AtMost(1))
        .entry()
        .children(track, Count::Any)
        .attr::<f64>("tempo")
        .default(120.0)
        .default_attr();

    v.wire_kind(plays)
        .connects(clip, clip)
        .cyclic()
        .attr::<f32>("gain")
        .default(1.0)
        .default_attr()
        .attr::<i32>("transpose")
        .default(0);

    v.wire_kind(starts)
        .connects(clip, clip)
        .connects(track, clip)
        .cyclic();

    v.prelude("music.sym", include_str!("music.sym"));

    let vocab = v.build()?;
    Ok(vocab)
}

#[test]
fn the_architecture_example_builds() {
    let vocab = example().expect("the example vocabulary is valid");
    assert_eq!(vocab.name(), "music");
    let note = vocab.node_kind_named("Note").expect("Note is declared");
    let decl = vocab
        .node_kind(note)
        .expect("Note belongs to the vocabulary");
    assert_eq!(decl.attrs.len(), 5);
    assert_eq!(decl.default_attr.map(|attr| attr.index()), Some(0));
    assert_eq!(decl.attrs[4].flag, Some('\''));
    assert_eq!(decl.attrs[1].doc.as_deref(), Some("Loudness, 0 to 1."));
    assert_eq!(decl.attrs[3].ty, Ty::F64);
    let unit = decl.attrs[3].unit.expect("length has a unit");
    let family = vocab
        .family(unit.family)
        .expect("beats belongs to the vocabulary");
    assert_eq!(family.units[unit.default_unit].name, "beats");
    assert_eq!(vocab.unit_named("bars").map(|(_, unit)| unit), Some(1));
    let osc = vocab.node_kind_named("Osc").expect("Osc is declared");
    let wave_attr = &vocab.node_kind(osc).expect("Osc belongs").attrs[1];
    let wave = vocab.enum_named("Wave").expect("Wave is declared");
    assert_eq!(wave_attr.ty, Ty::Enum(wave));
    assert_eq!(
        wave_attr.default,
        Some(AttrDefault::Elements(vec![Value::Enum(wave, 0)]))
    );
    let starts = vocab.wire_kind_named("Starts").expect("Starts is declared");
    let starts = vocab.wire_kind(starts).expect("Starts belongs");
    assert_eq!(starts.connects.len(), 2);
    assert!(starts.cyclic);
    assert_eq!(vocab.prelude()[0].path, "music.sym");
}
