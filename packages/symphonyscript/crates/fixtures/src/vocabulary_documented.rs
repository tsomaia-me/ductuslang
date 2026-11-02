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
    let time = v
        .unit_family("time")
        .doc(&doc("Clock time."))
        .base("s")
        .doc(&doc("One second."))
        .unit("ms", 0.001)
        .doc(&doc("One millisecond."));
    v.unit_family("pressure")
        .doc(&doc("Pressure."))
        .base("pa")
        .doc(&doc("One pascal."))
        .unit("kpa", 1000.0)
        .doc(&doc("One kilopascal."));
    let wave = v
        .enumeration("Wave", ["Sine", "Saw", "Square"])
        .doc(&doc("An oscillator waveform."));
    let mode = v
        .enumeration("Mode", ["Auto", "Manual", "Off"])
        .doc(&doc("How a plant runs."));

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
    let rig = v.node("Rig");
    let lamp = v.node("Lamp");
    let scene = v.node("Scene");
    let cue = v.node("Cue");
    let plant = v.node("Plant");
    let sensor = v.node("Sensor");
    let valve = v.node("Valve");
    let controller = v.node("Controller");
    let plays = v.wire("Plays");
    let starts = v.wire("Starts");
    let includes = v.wire("Includes");
    let feeds = v.wire("Feeds");
    let modulates = v.wire("Modulates");
    let triggers = v.wire("Triggers");
    let dims = v.wire("Dims");
    let reads = v.wire("Reads");
    let drives = v.wire("Drives");

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
    v.kind(rig)
        .doc(&doc("A set of lamps and the scenes that light them."))
        .top_level(Count::Any)
        .children(lamp, Count::Any)
        .children(scene, Count::Any);
    v.kind(lamp)
        .doc(&doc("A light source."))
        .attr::<f32>("level")
        .default(0.0)
        .default_attr()
        .doc(&doc("Brightness, 0 to 1."))
        .attr::<f32>("hue")
        .default(0.0)
        .doc(&doc("Colour, 0 to 1 around the colour wheel."))
        .attr::<bool>("strobe")
        .default(false)
        .flag('*')
        .doc(&doc("Flashes the lamp."));
    v.kind(scene)
        .doc(&doc("A look built from timed cues."))
        .top_level(Count::Any)
        .children(cue, Count::Any)
        .attr::<f64>("hold")
        .default(4.0)
        .family(&time, "s")
        .default_attr()
        .doc(&doc("How long the scene holds."));
    v.kind(cue)
        .doc(&doc("One step of a scene."))
        .attr::<f64>("at")
        .default(0.0)
        .family(&time, "s")
        .default_attr()
        .doc(&doc("When the cue starts within its scene."))
        .attr::<f64>("fade")
        .default(1.0)
        .family(&time, "s")
        .doc(&doc("How long the cue takes to reach its look."));
    v.kind(plant)
        .doc(&doc("A controlled process and its devices."))
        .top_level(Count::Any)
        .children(sensor, Count::Any)
        .children(valve, Count::Any)
        .children(controller, Count::Any)
        .attr_enum("mode", &mode, "Auto")
        .doc(&doc("How the plant runs."));
    v.kind(sensor)
        .doc(&doc("A measurement the host supplies."))
        .attr::<f32>("reading")
        .default_attr()
        .doc(&doc("The measured value."));
    v.kind(valve)
        .doc(&doc("An actuator that opens and closes."))
        .attr::<f32>("open")
        .default(0.0)
        .default_attr()
        .doc(&doc("How far open, 0 to 1."));
    v.kind(controller)
        .doc(&doc("Holds a reading at its target."))
        .attr::<f32>("target")
        .default(0.0)
        .default_attr()
        .doc(&doc("The value the controller holds."))
        .attr::<f32>("gain")
        .default(1.0)
        .doc(&doc("How strongly the controller responds."));

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
    v.wire_kind(triggers)
        .doc(&doc("Starts another scene after this one."))
        .connects(scene, scene)
        .cyclic();
    v.wire_kind(dims)
        .doc(&doc("Sets a lamp's level from a scene."))
        .connects(scene, lamp)
        .attr::<f32>("depth")
        .default(1.0)
        .default_attr()
        .doc(&doc("How much of the scene reaches the lamp."));
    v.wire_kind(reads)
        .doc(&doc("Feeds a sensor's reading to a controller."))
        .connects(sensor, controller);
    v.wire_kind(drives)
        .doc(&doc("Sets a valve from a controller."))
        .connects(controller, valve)
        .attr::<f32>("limit")
        .default(1.0)
        .default_attr()
        .doc(&doc("The most the controller opens the valve."));

    v.build()
        .unwrap_or_else(|invalid| panic!("the example vocabulary is invalid: {invalid}"))
}
