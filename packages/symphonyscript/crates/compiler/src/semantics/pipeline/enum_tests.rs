use crate::compiler::Compiler;
use crate::syntax::front_end_goldens::source;

const ENUMS: [&str; 34] = [
    "Clip c:\n    Note/(i32(Wave::Saw))\n",
    "Clip c:\n    Note/(if Wave::Saw is Mode::Auto then 1 else 2)\n",
    "Clip c:\n    Note/(if Wave::Saw is Wave::Sine then 1 else 2)\n",
    "Clip c:\n    Note/(not Wave::Saw)\n",
    "Clip c:\n    Note/Saw\n",
    "Clip c:\n    Note/(Wave::Saw + 1)\n",
    "Clip c:\n    Note/(-Wave::Saw)\n",
    "Clip c:\n    Note/(Wave::Zig)\n",
    "let a = Saw\nlet b = a\nlet c = if true then b else Square\nSynth s:\n    Osc | wave=Sine\n",
    "let a = Wave::Saw\nClip c:\n    Note/a\n",
    "let a = Wave::Saw\nSynth s:\n    Osc | wave=a\n",
    "let Saw = 1\nClip c:\n    Note/1\n",
    "let Square = 2\nSynth s:\n    Osc | wave=Square\n",
    "state f(false)\nSynth s:\n    Osc | wave=(if f then Saw else Square)\n",
    "state mode(Mode::Manual)\nPlant p:\n    Valve | open=(if mode is Auto then 0.5 else 0.0)\n",
    "state w(Wave::Saw)\nClip c:\n    Note/(i32(w))\n",
    "state w(Wave::Saw)\nClip c:\n    Note/(if w is Manual then 1 else 2)\n",
    "state w(Wave::Saw)\nClip c:\n    Note/(if w is not Sine then 1 else 2)\n",
    "state w(Wave::Saw)\nClip c:\n    Note/w\n",
    "state w(Wave::Saw)\nstate v = if w is Saw then Wave::Sine else w\nSynth s:\n    Osc | wave=v\n",
    "state w(Wave::Saw)\nstate v = w\nSynth s:\n    Osc | wave=v\n",
    "state w(Wave::Saw)\nSynth s:\n    Osc | wave=(if w is Saw then Square else Sine)\n",
    "state w(Wave::Saw)\nSynth s:\n    Osc | wave=w\n",
    "Synth s:\n    Osc | wave=1\n",
    "Synth s:\n    Osc | wave=(if true then Manual else Wave::Square)\n",
    "Synth s:\n    Osc | wave=(if true then Mode::Auto else Square)\n",
    "Synth s:\n    Osc | wave=(if true then Saw else Square)\n",
    "Synth s:\n    Osc | wave=(if true then Saw else Wave::Square)\n",
    "Synth s:\n    Osc | wave=Manual\n",
    "Synth s:\n    Osc | wave=Mode::Manual\n",
    "Synth s:\n    Osc | wave=Saw\n",
    "Synth s:\n    Osc | wave=true\n",
    "Synth s:\n    Osc | wave=Wave::Saw\n",
    "Synth s:\n    Osc | wave=Wave::Zig\n",
];

const DECLINED: [&str; 10] = [
    "fn f(w): w\nSynth s:\n    Osc | wave=(f(Saw))\n",
    "fn pick(x): if x then Wave::Saw else Wave::Sine\nSynth s:\n    Osc | wave=(pick(true))\n",
    "let a = Saw\nSynth s:\n    Osc | wave=a\n",
    "state m(Manual): Mode\nSynth s:\n    Osc | wave=Saw\n",
    "state w(Saw)\nSynth s:\n    Osc | wave=w\n",
    "state w(Wave::Saw)\nClip c:\n    Note/(if w.past(1) is Saw then 1 else 2)\n",
    "state w(Wave::Saw)\nstate m = w.past(1, Sine)\nSynth s:\n    Osc | wave=m\n",
    "Synth s:\n    for w in [Wave::Saw, Wave::Sine]:\n        Osc | wave=w\n",
    "Synth s:\n    Osc | wave=(if Saw then Saw else Saw)\n",
    "Synth s:\n    Osc | wave=([Wave::Saw, Wave::Sine][1])\n",
];

#[test]
fn qualified_and_bare_variants_enum_attributes_enum_cells_and_is_draw_what_the_old_engine_draws() {
    let vocabulary = symphonyscript_fixtures::vocabulary();
    let old = Compiler::new(&vocabulary)
        .expect("the vocabulary compiles")
        .on_the_old_engine();
    let new = Compiler::new(&vocabulary).expect("the vocabulary compiles");
    for text in ENUMS {
        let sources = [source(text)];
        let found = new
            .checked_by_new_core(&sources)
            .unwrap_or_else(|| panic!("the new core takes {text}"));
        assert_eq!(found, old.check(&sources), "{text}");
    }
    for text in DECLINED {
        assert_eq!(new.checked_by_new_core(&[source(text)]), None, "{text}");
    }
}
