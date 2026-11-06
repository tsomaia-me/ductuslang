use crate::compiler::Compiler;
use crate::syntax::front_end_goldens::source;

const ENUMS: [&str; 37] = [
    "Synth s:\n    Osc | wave=([Wave::Saw, Wave::Sine][1])\n",
    "let a = Saw\nSynth s:\n    Osc | wave=a\n",
    "state m(Manual): Mode\nSynth s:\n    Osc | wave=Saw\n",
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

const DECLINED: [&str; 3] = [
    "fn f(w): w\nSynth s:\n    Osc | wave=(f(Saw))\n",
    "state w(Wave::Saw)\nstate m = w.past(1, Sine)\nSynth s:\n    Osc | wave=m\n",
    "Synth s:\n    Osc | wave=(if Saw then Saw else Saw)\n",
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

const ENUM_ANNOTATIONS: [(&str, &str); 11] = [
    ("state m(Manual): Mode\n", "Plant p | mode=m\n"),
    ("state m(Mode::Auto): Mode\n", "Plant p | mode=m\n"),
    ("state m(1): Mode\n", "Plant p | mode=m\n"),
    ("state m(Sideways): Mode\n", "Plant p | mode=m\n"),
    ("let w: Wave = Saw\n", "Synth s:\n    Osc | wave=w\n"),
    (
        "let w: Wave = Wave::Square\n",
        "Synth s:\n    Osc | wave=w\n",
    ),
    ("let w: Wave = Mode::Auto\n", "Synth s:\n    Osc | wave=w\n"),
    ("let w: Wave = 3\n", "Synth s:\n    Osc | wave=w\n"),
    (
        "state m(Manual): Mode\n",
        "Plant p | mode=(if m is Auto then Manual else m)\n",
    ),
    ("let w: Mode = Auto\n", "Synth s:\n    Osc | wave=w\n"),
    ("state m(Manual): Mode\n", "Clip c:\n    Note/m\n"),
];

#[test]
fn an_enum_annotation_types_its_let_or_cell_and_gives_a_bare_init_its_enum_in_both_orders() {
    let vocabulary = symphonyscript_fixtures::vocabulary();
    let old = Compiler::new(&vocabulary)
        .expect("the vocabulary compiles")
        .on_the_old_engine();
    let new = Compiler::new(&vocabulary).expect("the vocabulary compiles");
    for (first, second) in ENUM_ANNOTATIONS {
        for text in [format!("{first}{second}"), format!("{second}{first}")] {
            let sources = [source(&text)];
            let found = new
                .checked_by_new_core(&sources)
                .unwrap_or_else(|| panic!("the new core takes {text}"));
            assert_eq!(found, old.check(&sources), "{text}");
        }
    }
    let text = "state m(Manual): Nope\nPlant p | mode=m\n";
    assert_eq!(new.checked_by_new_core(&[source(text)]), None, "{text}");
}

const BARE_LETS: [(&str, &str); 6] = [
    (
        "let b = a\nlet a = Saw\n",
        "Synth s:\n    Osc | wave=b\n    Osc | wave=a\n",
    ),
    ("let a = b\nlet b = a\n", "Synth s:\n    Osc | wave=b\n"),
    ("let a = Saw\n", "Synth s:\n    Osc | wave=a\n"),
    ("let a = Saw\nlet b = a\n", "Synth s:\n    Osc | wave=b\n"),
    (
        "let a = Saw\n",
        "Synth s:\n    Osc | wave=a\n    Osc | wave=a\n",
    ),
    (
        "let a = Square\n",
        "Synth s:\n    Osc | wave=a\nSynth t:\n    Osc | wave=a\n",
    ),
];

const BARE_LETS_DECLINED: [&str; 2] = [
    "state b(true)\nlet a = if b then Saw else Square\nSynth s:\n    Osc | wave=a\n",
    "state t(true)\nlet a = Saw\nlet b = a\nlet c = if t then b else Square\nSynth s:\n    Osc | wave=c\n",
];

#[test]
fn a_bare_valued_let_read_only_at_attributes_of_one_enum_takes_that_enum_in_both_orders() {
    let vocabulary = symphonyscript_fixtures::vocabulary();
    let old = Compiler::new(&vocabulary)
        .expect("the vocabulary compiles")
        .on_the_old_engine();
    let new = Compiler::new(&vocabulary).expect("the vocabulary compiles");
    for (first, second) in BARE_LETS {
        for text in [format!("{first}{second}"), format!("{second}{first}")] {
            let sources = [source(&text)];
            let found = new
                .checked_by_new_core(&sources)
                .unwrap_or_else(|| panic!("the new core takes {text}"));
            assert_eq!(found, old.check(&sources), "{text}");
        }
    }
    for text in BARE_LETS_DECLINED {
        assert_eq!(new.checked_by_new_core(&[source(text)]), None, "{text}");
    }
}

const ENUM_ARRAYS: [(&str, &str); 12] = [
    (
        "",
        "Synth s:\n    Osc | wave=([Wave::Saw, Wave::Sine][1])\n",
    ),
    (
        "",
        "Synth s:\n    Osc | wave=([Wave::Saw, Wave::Sine][5])\n",
    ),
    ("", "Synth s:\n    Osc | wave=([Wave::Saw, 1][0])\n"),
    (
        "",
        "Synth s:\n    Osc | wave=([Wave::Saw, Mode::Auto][0])\n",
    ),
    (
        "let ws = [Wave::Saw, Wave::Sine]\n",
        "Synth s:\n    Osc | wave=(ws[1])\n",
    ),
    ("", "Clip c:\n    Note/([Wave::Saw][0])\n"),
    (
        "",
        "Synth s:\n    Osc | wave=([Wave::Saw, Wave::Sine][^1])\n",
    ),
    (
        "state i(0)\n",
        "Synth s:\n    Osc | wave=([Wave::Saw, Wave::Sine][i])\n",
    ),
    ("", "Synth s:\n    Osc | wave=[Wave::Saw, Wave::Sine]\n"),
    (
        "let ws = [Wave::Saw, Wave::Sine]\n",
        "Clip c:\n    Note/(if ws[0] is Wave::Saw then 1 else 2)\n",
    ),
    (
        "",
        "Synth s:\n    Osc | wave=([[Wave::Saw], [Wave::Sine]][1][0])\n",
    ),
    (
        "let ws = [Wave::Saw, Wave::Sine] + [Wave::Square]\n",
        "Synth s:\n    Osc | wave=(ws[2])\n",
    ),
];

#[test]
fn an_array_of_qualified_variants_is_built_joined_and_read_by_index_as_the_old_engine_reads_it_in_both_orders()
 {
    let vocabulary = symphonyscript_fixtures::vocabulary();
    let old = Compiler::new(&vocabulary)
        .expect("the vocabulary compiles")
        .on_the_old_engine();
    let new = Compiler::new(&vocabulary).expect("the vocabulary compiles");
    for (first, second) in ENUM_ARRAYS {
        for text in [format!("{first}{second}"), format!("{second}{first}")] {
            let sources = [source(&text)];
            let found = new
                .checked_by_new_core(&sources)
                .unwrap_or_else(|| panic!("the new core takes {text}"));
            assert_eq!(found, old.check(&sources), "{text}");
        }
    }
}

const ENUMS_IN_BODIES: [(&str, &str); 17] = [
    (
        "state w(Wave::Saw)\n",
        "Synth s:\n    Osc | wave=(w.past(1, Wave::Sine))\n",
    ),
    (
        "state w(Wave::Saw)\n",
        "Synth s:\n    Osc | wave=(w.past(1, Mode::Auto))\n",
    ),
    (
        "state w(Wave::Saw)\nstate m = w.past(1, Wave::Sine)\n",
        "Synth s:\n    Osc | wave=m\n",
    ),
    (
        "state w(Wave::Saw)\n",
        "Clip c:\n    Note/(w.past(2, Wave::Sine))\n",
    ),
    (
        "state w(Wave::Saw)\n",
        "Clip c:\n    Note/(if w.past(1) is Saw then 1 else 2)\n",
    ),
    (
        "state w(Wave::Saw)\n",
        "Clip c:\n    Note/(if w.past(1) is Wave::Saw then 1 else 2)\n",
    ),
    (
        "fn same(w): w\n",
        "Synth s:\n    Osc | wave=(same(Wave::Saw))\n",
    ),
    (
        "fn same(w): w\n",
        "Synth s:\n    Osc | wave=(same(Wave::Saw))\nPlant p | mode=(same(Mode::Auto))\n",
    ),
    (
        "fn same(w): w\n",
        "Synth s:\n    Osc | wave=(same(Mode::Auto))\n",
    ),
    (
        "fn f(w): if w is Wave::Saw then 1 else 2\n",
        "Clip c:\n    Note/(f(Wave::Saw) + f(Wave::Sine))\n",
    ),
    (
        "let n = for w in [Wave::Saw, Wave::Sine] into a from Wave::Saw: w\n",
        "Synth s:\n    Osc | wave=n\n",
    ),
    (
        "fn pick(x): if x then Wave::Saw else Wave::Sine\n",
        "Synth s:\n    Osc | wave=(pick(true))\n",
    ),
    (
        "fn pick(x): if x then Wave::Saw else Wave::Sine\nstate b(true)\n",
        "Synth s:\n    Osc | wave=(pick(b))\n",
    ),
    (
        "",
        "Synth s:\n    for w in [Wave::Saw, Wave::Sine]:\n        Osc | wave=w\n",
    ),
    (
        "fn bad(x): if x then Wave::Saw else Mode::Auto\n",
        "Synth s:\n    Osc | wave=(bad(true))\n",
    ),
    (
        "",
        "Clip c:\n    for i in 0..2:\n        Note/(if [Wave::Saw, Wave::Sine][i] is Wave::Saw then 1 else 2)\n",
    ),
    (
        "let a = Sine\n",
        "Synth s:\n    for i in 0..2:\n        Osc | wave=a\n",
    ),
];

#[test]
fn a_qualified_variant_inside_a_function_or_a_placement_for_body_draws_what_the_old_engine_draws_in_both_orders()
 {
    let vocabulary = symphonyscript_fixtures::vocabulary();
    let old = Compiler::new(&vocabulary)
        .expect("the vocabulary compiles")
        .on_the_old_engine();
    let new = Compiler::new(&vocabulary).expect("the vocabulary compiles");
    for (first, second) in ENUMS_IN_BODIES {
        for text in [format!("{first}{second}"), format!("{second}{first}")] {
            let sources = [source(&text)];
            let found = new
                .checked_by_new_core(&sources)
                .unwrap_or_else(|| panic!("the new core takes {text}"));
            assert_eq!(found, old.check(&sources), "{text}");
        }
    }
}

const UNTYPED_FLOWS: [(&str, &str); 22] = [
    (
        "let a = Saw\n",
        "Clip c1:\n    Note/(if a is Mode::Auto then 1 else 2)\n",
    ),
    ("state m(Saw): Mode\n", "Plant p | mode=m\n"),
    ("let a = Manual\n", "Synth s:\n    Osc | wave=a\n"),
    ("let w: Wave = Auto\n", "Synth s:\n    Osc | wave=w\n"),
    (
        "let a = Manual\nlet b = a\n",
        "Synth s:\n    Osc | wave=b\n",
    ),
    (
        "state b(true)\nstate m(if b then Saw else Auto): Mode\n",
        "Plant p | mode=m\n",
    ),
    ("state w(Saw)\n", "Synth s:\n    Osc | wave=w\n"),
    (
        "state w(Saw)\n",
        "Clip c:\n    Note/(if w is Wave::Saw then 1 else 2)\n",
    ),
    ("state w(Saw)\n", "Clip c:\n    Note/w\n"),
    ("state w(Auto)\n", "Plant p | mode=w\n"),
    (
        "let a = Saw\nlet b = Sine\n",
        "Clip c1:\n    Note/(if a is b then 1 else 2)\n",
    ),
    (
        "let a = Saw\n",
        "Clip c1:\n    Note/(if a is Wave::Sine then 1 else 2)\n",
    ),
    (
        "let a = Saw\n",
        "Clip c1:\n    Note/(if Wave::Sine is not a then 1 else 2)\n",
    ),
    ("let a = Saw\n", "Clip c:\n    Note/a\n"),
    (
        "let a = Saw\nlet b = a\nlet c = b\n",
        "Clip c1:\n    Note/c\n",
    ),
    (
        "let c = b\nlet b = a\nlet a = Saw\n",
        "Clip c1:\n    Note/c\n",
    ),
    (
        "let a = Saw\nlet b = if true then a else Square\n",
        "Clip c1:\n    Note/b\n",
    ),
    ("let a = Saw\n", "Clip c1:\n    Note/a\n    Note/a\n"),
    (
        "let a = Saw\n",
        "Synth s:\n    Osc | wave=a\nClip c1:\n    Note/a\n",
    ),
    (
        "let a = Saw\nlet b = a\n",
        "Synth s:\n    Osc | wave=b\nPlant p | mode=a\n",
    ),
    (
        "let a = Saw\n",
        "Synth s:\n    Osc | wave=a\nPlant p | mode=a\n",
    ),
    (
        "let a = Auto\n",
        "Synth s:\n    Osc | wave=a\nPlant p | mode=a\n",
    ),
];

#[test]
fn an_untyped_bare_variant_flow_draws_one_diagnostic_at_its_first_writer_naming_every_other_let_once_in_both_orders()
 {
    let vocabulary = symphonyscript_fixtures::vocabulary();
    let old = Compiler::new(&vocabulary)
        .expect("the vocabulary compiles")
        .on_the_old_engine();
    let new = Compiler::new(&vocabulary).expect("the vocabulary compiles");
    for (first, second) in UNTYPED_FLOWS {
        for text in [format!("{first}{second}"), format!("{second}{first}")] {
            let sources = [source(&text)];
            let found = new
                .checked_by_new_core(&sources)
                .unwrap_or_else(|| panic!("the new core takes {text}"));
            assert_eq!(found, old.check(&sources), "{text}");
        }
    }
}
