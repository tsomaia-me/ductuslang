use crate::compiler::Compiler;
use crate::syntax::front_end_goldens::source;

const FAMILIES: [&str; 59] = [
    "Clip c:\n    Note/(1000ms)\n",
    "Clip c:\n    Note/(2bars)\n",
    "Clip c:\n    Note | length=1000ms\n",
    "Clip c:\n    Note | length=([1bars, 2bars][0..1][0])\n",
    "Clip c:\n    Note | length=(2 - 1bars)\n",
    "Clip c:\n    Note | length=(2bars + 1.5)\n",
    "Clip c:\n    Note | length=2bars + 1\n",
    "Clip c:\n    Note | length=(2bars div 3)\n",
    "Clip c:\n    Note | length=(-(2bars))\n",
    "Clip c:\n    Note | length=2bars\n",
    "Clip c:\n    Note | length=(2bars + true)\n",
    "Clip c:\n    Note | length=(2bars + zz)\n",
    "Clip c:\n    Note | length=(5i64 + 1bars)\n",
    "Clip c:\n    Note | length=(5u8 * 1bars)\n",
    "Clip c:\n    Note | length=(zz + 2)\n",
    "Clip c:\n    Note | velocity=(2s)\n",
    "Clip c:\n    Note | velocity=(f32(1bars / 2bars))\n",
    "let a = [1bars, 2]\nClip c:\n    Note/(a[0])\n",
    "let a = [1bars, 2]\nClip c:\n    Note | length=(a[0])\n",
    "let a = 2bars + 1\nClip c:\n    Note/1\n",
    "let a = 2bars\nClip c:\n    Note/(a div 1bars)\n",
    "let a = 2bars\nClip c:\n    Note/a\n",
    "let a = 2s\nlet b = a + 1\nScene s:\n    Cue | at=b\n",
    "let a = 2s\nScene s:\n    Cue | at=a\n",
    "let k = 1bars\nlet m = k * 2\nlet n = m + 3\nClip c:\n    Note | length=n\n",
    "Scene s:\n    Cue | at=(1.5s + 1ms)\n",
    "Scene s:\n    Cue | at=(1s + 1bars + 2)\n",
    "Scene s:\n    Cue | at=(1s + 1bars)\n",
    "Scene s:\n    Cue | at=(1s * 1s)\n",
    "Scene s:\n    Cue | at=(1s * 2)\n",
    "Scene s:\n    Cue | at=(1s ** 2)\n",
    "Scene s:\n    Cue | at=(1s + 3)\n",
    "Scene s:\n    Cue | at=(-1s)\n",
    "Scene s:\n    Cue | at=(2bars / 1bars)\n",
    "Scene s:\n    Cue | at=(2s % 1)\n",
    "Scene s:\n    Cue | at=(2s > 1)\n",
    "Scene s:\n    Cue | at=(2s / 1s)\n",
    "Scene s:\n    Cue | at=(2s div 1s)\n",
    "Scene s:\n    Cue | at=(300000000000ms)\n",
    "Scene s:\n    Cue | at=(3 / 1s)\n",
    "Scene s:\n    Cue | at=500ms\n    Cue | at=2\n",
    "Scene s:\n    Cue | at=(f32(1ms))\n",
    "Scene s:\n    Cue | at=(f64(2s))\n",
    "Scene s:\n    Cue | at=(if (1s > 2) then 1 else 2)\n",
    "Scene s:\n    Cue | at=(if true then 1s else 1bars)\n",
    "state d(5ms)\nScene s:\n    Cue | at=d\n",
    "state d(5ms)\nstate e = d + 1\nClip c:\n    Note/1\n",
    "state d(5ms)\nstate e = d + 1\nScene s:\n    Cue | at=e\n",
    "state d(5ms)\nstate e = d * 2\nScene s:\n    Cue | at=e\n",
    "state p(5.0)\nScene s:\n    Cue | at=p\nClip c:\n    Note | length=p\n",
    "state p(5.0)\nScene s:\n    Cue | at=p\n    Cue | fade=p\n",
    "state x(1)\nClip c:\n    Note | length=(x + 1bars)\n",
    "state x(1)\nClip c:\n    Note/(x + 1bars)\n",
    "state x(1)\nScene s:\n    Cue | at=(x.past(1, 2s))\n",
    "state x(1s)\nClip c:\n    Note/(x.past(1))\n",
    "state x(1s)\nScene s:\n    Cue | at=(if x.stepped then 1s else 2)\n",
    "state x(1s)\nScene s:\n    Cue | at=(x.past(1, 2bars))\n",
    "state x(1s)\nScene s:\n    Cue | at=(x.past(1, 2))\n",
    "state x(1s)\nstate y = x.past(1) + 1\nScene s:\n    Cue | at=y\n",
];

const DECLINED: [&str; 7] = [
    "Clip c:\n    for i in 0..2s:\n        Note\n",
    "Clip c:\n    Note/([1, 2][0..1bars][0])\n",
    "Clip c:\n    Note/([1, 2][1s])\n",
    "Clip c:\n    Note | length=([1bars, 2bars][0..1bars][0])\n",
    "Clip c:\n    Note | length=(for i in 0..3 into a from 0bars: a + 1)\n",
    "fn f(x): x + 1\nScene s:\n    Cue | at=(f(5ms))\n",
    "fn f(x): x\nClip c:\n    Note/(f(2bars))\n",
];

#[test]
fn unit_literals_families_through_expressions_and_unit_attributes_draw_what_the_old_engine_draws() {
    let vocabulary = symphonyscript_fixtures::vocabulary();
    let old = Compiler::new(&vocabulary)
        .expect("the vocabulary compiles")
        .on_the_old_engine();
    let new = Compiler::new(&vocabulary).expect("the vocabulary compiles");
    for text in FAMILIES {
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
