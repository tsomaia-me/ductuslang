use symphonyscript_core::{Count, Vocabulary};

pub fn vocabulary_documented(doc: impl Fn(&str) -> String) -> Vocabulary {
    let mut v = Vocabulary::builder("example");

    let beats = v
        .unit_family("beats")
        .doc(&doc("Musical time."))
        .base("beats")
        .doc(&doc("One beat."))
        .unit("bars", 4.0)
        .doc(&doc("Four beats."));
    v.unit_family("time")
        .doc(&doc("Clock time."))
        .base("s")
        .doc(&doc("One second."))
        .unit("ms", 0.001)
        .doc(&doc("One millisecond."));
    let wave = v
        .enumeration("Wave", ["Sine", "Saw", "Square"])
        .doc(&doc("An oscillator waveform."));

    let session = v.node("Session");
    let track = v.node("Track");
    let clip = v.node("Clip");
    let note = v.node("Note");
    let kit = v.node("Kit");
    let sampler = v.node("Sampler");
    let piano = v.node("Piano");
    let synth = v.node("Synth");
    let osc = v.node("Osc");
    let filter = v.node("Filter");
    let mixer = v.node("Mixer");
    let output = v.node("Output");
    let plays = v.wire("Plays");
    let starts = v.wire("Starts");
    let includes = v.wire("Includes");
    let feeds = v.wire("Feeds");
    let modulates = v.wire("Modulates");

    v.kind(session)
        .doc(&doc("The whole piece."))
        .top_level(Count::AtMost(1))
        .entry()
        .children(track, Count::Any)
        .attr::<f64>("tempo")
        .default(120.0)
        .default_attr()
        .doc(&doc("Beats per minute."));
    v.kind(track)
        .doc(&doc("A lane of instruments."))
        .top_level(Count::Any)
        .children(piano, Count::Any)
        .children(synth, Count::Any)
        .attr::<&'static str>("label")
        .default("")
        .doc(&doc("The name shown for the track."));
    v.kind(clip)
        .doc(&doc("A sequence of notes."))
        .top_level(Count::Any)
        .children(note, Count::Any)
        .attr::<f32>("gain")
        .default(1.0)
        .doc(&doc("Output gain."))
        .attr::<f32>("level")
        .default(1.0)
        .doc(&doc("Mix level."));
    v.kind(note)
        .doc(&doc("A single pitched event inside a clip."))
        .attr::<i32>("pitch")
        .default(60)
        .default_attr()
        .doc(&doc("The note number."))
        .attr::<f32>("velocity")
        .default(1.0)
        .doc(&doc("Loudness, 0 to 1."))
        .attr::<f32>("brightness")
        .default(0.5)
        .doc(&doc("Timbre, 0 to 1."))
        .attr::<f64>("length")
        .default(0.25)
        .family(&beats, "beats")
        .doc(&doc("Duration in beats."))
        .attr::<bool>("accent")
        .default(false)
        .flag('\'')
        .doc(&doc("Plays the note accented."));
    v.kind(kit)
        .doc(&doc("A set of samples."))
        .top_level(Count::Any)
        .children(sampler, Count::Any);
    v.kind(sampler)
        .doc(&doc("Plays one sample file."))
        .attr::<&'static str>("file")
        .default_attr()
        .doc(&doc("The sample file."));
    v.kind(piano).doc(&doc("A piano instrument."));
    v.kind(synth)
        .doc(&doc("A synthesizer built from audio nodes."))
        .top_level(Count::Any)
        .children(osc, Count::Any)
        .children(filter, Count::Any)
        .children(mixer, Count::Any)
        .children(output, Count::Any);
    v.kind(osc)
        .doc(&doc("An oscillator."))
        .top_level(Count::Any)
        .attr::<f64>("freq")
        .default(440.0)
        .default_attr()
        .doc(&doc("Frequency in hertz."))
        .attr_enum("wave", &wave, "Sine")
        .doc(&doc("The waveform."))
        .attr::<f32>("rate")
        .default(1.0)
        .doc(&doc("Playback rate."));
    v.kind(filter)
        .doc(&doc("A filter."))
        .top_level(Count::Any)
        .attr::<f32>("cutoff")
        .default(1000.0)
        .default_attr()
        .doc(&doc("Cutoff frequency in hertz."));
    v.kind(mixer)
        .doc(&doc("Sums its inputs."))
        .top_level(Count::Any);
    v.kind(output)
        .doc(&doc("An audio output."))
        .top_level(Count::Any);

    v.wire_kind(plays)
        .doc(&doc("Plays another clip after this one."))
        .connects(clip, clip)
        .cyclic()
        .attr::<f32>("gain")
        .default(1.0)
        .default_attr()
        .doc(&doc("Gain applied to the played clip."))
        .attr::<i32>("transpose")
        .default(0)
        .doc(&doc("Semitones to transpose by."));
    v.wire_kind(starts)
        .doc(&doc("Starts a clip."))
        .connects(clip, clip)
        .connects(track, clip)
        .cyclic();
    v.wire_kind(includes)
        .doc(&doc("Includes a track in the session."))
        .connects(session, track);
    v.wire_kind(feeds)
        .doc(&doc("Sends audio onward."))
        .connects(osc, filter)
        .connects(osc, mixer)
        .connects(osc, output)
        .connects(filter, filter)
        .connects(filter, mixer)
        .connects(filter, output);
    v.wire_kind(modulates)
        .doc(&doc("Modulates a filter's cutoff."))
        .connects(osc, filter)
        .attr::<f32>("depth")
        .default(1.0)
        .default_attr()
        .doc(&doc("Modulation depth."));

    v.build()
        .unwrap_or_else(|invalid| panic!("the example vocabulary is invalid: {invalid}"))
}
