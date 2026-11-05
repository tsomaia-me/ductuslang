use crate::compiler::Compiler;
use crate::syntax::front_end_goldens::source;

const TYPE_SUFFIXES: [&str; 32] = [
    "Clip c:\n    for i in 0..3i64:\n        Note/i\n",
    "Clip c:\n    Note/([1, 2, 3][0..2u8][0])\n",
    "state x(0)\nClip c:\n    Note/(x.past(1i64))\n",
    "Clip c:\n    Note/(for i in 0..3 into a from 0u8: a + i)\n",
    "Clip c:\n    Note/(for i in 0..3 into a from 0u8: a + 300)\n",
    "state x(0)\nClip c:\n    Note/(x + 5i64)\n",
    "Clip c:\n    Note/5i64\n",
    "Clip c:\n    Note/5i32\n",
    "Clip c:\n    Note/(0.5i32)\n",
    "Clip c:\n    Note/(0.5i64)\n",
    "Clip c:\n    Note/(5u8 + 300)\n",
    "Clip c:\n    Note/(5u8 + 4)\n",
    "let a = 5f64\nClip c:\n    Note/a\n",
    "let a = 3i64\nClip c:\n    Note/(a + 1)\n",
    "let a = 300u8\nClip c:\n    Note/1\n",
    "let a = 300u8\nClip c:\n    Note/a\n",
    "Clip c:\n    Note | velocity=(5i64 * 0.5)\n",
    "Clip c:\n    Note | velocity=(5i32 * 0.5)\n",
    "let a = 2i64\nClip c:\n    Note | velocity=(a)\n",
    "Clip c:\n    Note/(-128i32)\n",
    "Clip c:\n    Note | velocity=(1e300f32)\n",
    "Clip c:\n    Note | velocity=(0.5f32)\n",
    "Clip c:\n    Note | velocity=(0.5f64)\n",
    "Clip c:\n    Note/(i32(5u8))\n",
    "state x(1u8)\nClip c:\n    Note/x\n",
    "state x(300u8)\nClip c:\n    Note/x\n",
    "fn f(v): v + 1\nClip c:\n    Note/(f(5i64))\n",
    "let a = [1u8, 2]\nClip c:\n    Note/(a[0])\n",
    "let a = [1u8, 2.5]\nClip c:\n    Note/(a[0])\n",
    "Clip c:\n    Note/([1, 2][1i64])\n",
    "Clip c:\n    Note/(5i64 div 2)\n",
    "Clip c:\n    Note/(if true then 1u8 else 300)\n",
];

#[test]
fn a_type_suffix_fixes_its_literals_type_and_an_integer_suffix_on_a_float_is_an_error_as_the_old_engine_draws()
 {
    let vocabulary = symphonyscript_fixtures::vocabulary();
    let old = Compiler::new(&vocabulary)
        .expect("the vocabulary compiles")
        .on_the_old_engine();
    let new = Compiler::new(&vocabulary).expect("the vocabulary compiles");
    for text in TYPE_SUFFIXES {
        let sources = [source(text)];
        let found = new
            .checked_by_new_core(&sources)
            .unwrap_or_else(|| panic!("the new core takes {text}"));
        assert_eq!(found, old.check(&sources), "{text}");
    }
}
