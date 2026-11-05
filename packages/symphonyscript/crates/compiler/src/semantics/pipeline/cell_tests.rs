use crate::compiler::Compiler;
use crate::diagnostics::Code;
use crate::syntax::front_end_goldens::source;

const CHAINS: [&str; 16] = [
    "state a(1)\nstate d = a + 1\nstate e = d * 2\nClip c:\n    Note/e\n",
    "state a(1)\nstate e = d * 2\nstate d = a + 1\nClip c:\n    Note/e\n",
    "state a(1)\nstate e = d * 2\nstate d = a + 1.5\nClip c:\n    Note/e\n",
    "state a(1)\nstate d = a + 1\nstate e = d * 2\nClip c | gain=e:\n    Note/d\n",
    "state a(1)\nstate d = a + 300000000000\nstate e: u8 = d + 1\nClip c:\n    Note/e\n",
    "state a(1)\nstate d = a + 300000000000\nstate e = d + 1\nClip c:\n    Note/e\n",
    "state a(1)\nstate d = a + 300\nstate e: u8 = d + 1\nstate f: i32 = d + 1\n",
    "state a(1)\nstate d = a + 3000000000\nstate e: i64 = d + 1\nClip c:\n    Note/d\n",
    "state a(1)\nstate d = a + 1\nstate e = d + true\nClip c:\n    Note/e\n",
    "state a(true)\nstate d = not a\nstate e = d and true\nClip c:\n    Note/e\n",
    "state a([1, 2])\nstate d = a + [3]\nstate e = d[0] + 1\nClip c:\n    Note/e\n",
    "fn f(x): x + 1\nstate a(1)\nstate d = f(a)\nstate e = f(d) * 2\nClip c:\n    Note/e\n",
    "fn g(): d\nstate a(1)\nstate e = g() * 2\nstate d = a + 1.5\nClip c:\n    Note/e\n",
    "state a(1)\nlet k = a + 1\nstate e = k * 2\nClip c:\n    Note/e\n",
    "state a(1)\nstate d = a + 1\nlet k = d + 1\nstate e = k * 2\nClip c:\n    Note/e\n",
    "let k = 5\nstate a(1)\nstate e = k * a\nClip c:\n    Note/e\n",
];

const CYCLES: [&str; 6] = [
    "state a = b + 1\nstate b = a + 1\nClip c:\n    Note/a\n",
    "state a = a + 1\nClip c:\n    Note/a\n",
    "state a = k + 1\nlet k = a\nClip c:\n    Note/a\n",
    "state s(1)\nstate a = k + s\nlet k = a + s\nClip c:\n    Note/a\n",
    "state d = c.gain + 1\nClip c | gain=d:\n    Note/1\n",
    "state x(1)\nstate d = c.gain + x\nClip c | gain=d:\n    Note/d\n",
];

#[test]
fn a_derived_cell_reading_derived_cells_takes_the_widest_width_its_readers_expect_and_a_cycle_draws_one_declaration_cycle()
 {
    let vocabulary = symphonyscript_fixtures::vocabulary();
    let old = Compiler::new(&vocabulary)
        .expect("the vocabulary compiles")
        .on_the_old_engine();
    let new = Compiler::new(&vocabulary).expect("the vocabulary compiles");
    for text in CHAINS.iter().chain(CYCLES.iter()) {
        let sources = [source(text)];
        let found = new
            .checked_by_new_core(&sources)
            .unwrap_or_else(|| panic!("the new core takes {text}"));
        assert_eq!(found, old.check(&sources), "{text}");
    }
    let cases: [(&str, &[Code]); 6] = [
        (CHAINS[0], &[]),
        (CHAINS[4], &[Code::DoesNotFit]),
        (CHAINS[6], &[]),
        (
            "state a(1)\nstate d = a + 300\nstate e: u8 = d + 1\n",
            &[Code::DoesNotFit],
        ),
        (CYCLES[0], &[Code::DeclarationCycle]),
        ("state a = b + 1\nstate b = 2\nClip c:\n    Note/a\n", &[]),
    ];
    for (text, codes) in cases {
        let found = new
            .checked_by_new_core(&[source(text)])
            .unwrap_or_else(|| panic!("the new core takes {text}"));
        let found: Vec<Code> = found
            .iter()
            .map(|diagnostic| diagnostic.code)
            .filter(|code| *code != Code::NoEntryInstance && *code != Code::UnreachableInstance)
            .collect();
        assert_eq!(found, codes, "{text}");
    }
}

#[test]
fn a_derived_cell_reading_a_reactive_let_draws_what_its_repaired_twin_draws_beside_reactive_let() {
    let vocabulary = symphonyscript_fixtures::vocabulary();
    let new = Compiler::new(&vocabulary).expect("the vocabulary compiles");
    let cases: [(&str, &[Code]); 4] = [
        (
            "state a(1)\nlet k = a + 1.5\nstate e = k * 2\nClip c:\n    Note/e\n",
            &[Code::ReactiveLet, Code::FloatToInteger],
        ),
        (
            "let k = 1.5\nstate e = k * 2\nClip c:\n    Note/e\n",
            &[Code::FloatToInteger],
        ),
        (
            "state a(1)\nlet k = a + 1.5\nstate e = k * 2 + a\nClip c:\n    Note/e\n",
            &[Code::IntegerCell, Code::ReactiveLet, Code::FloatToInteger],
        ),
        (
            "state a(1)\nlet k = 1.5\nstate e = k * 2 + a\nClip c:\n    Note/e\n",
            &[Code::IntegerCell, Code::FloatToInteger],
        ),
    ];
    for (text, codes) in cases {
        let found = new
            .checked_by_new_core(&[source(text)])
            .unwrap_or_else(|| panic!("the new core takes {text}"));
        let found: Vec<Code> = found
            .iter()
            .map(|diagnostic| diagnostic.code)
            .filter(|code| *code != Code::NoEntryInstance && *code != Code::UnreachableInstance)
            .collect();
        assert_eq!(found, codes, "{text}");
    }
}

const VARIABLE_CELLS: [&str; 16] = [
    "state held([]): i32[..10]\nClip c:\n    Note/(held[0])\n",
    "state held([1, 2]): i32[..1]\nClip c:\n    Note/(held[0])\n",
    "state held(3): i32[..4]\nClip c:\n    Note/(held[0])\n",
    "state held([[1]]): i32[..4]\nClip c:\n    Note/(held[0])\n",
    "state hh([300]): u8[..2]\nClip c:\n    Note/(hh[0])\n",
    "state hh([1, 300, 2.5]): u8[..2]\nClip c:\n    Note/(hh[0])\n",
    "state hh([1, true]): i32[..2]\nClip c:\n    Note/(hh[0])\n",
    "state hh([1]): i32[..1]\nstate v = hh + [2]\nClip c:\n    Note/(v[0])\n",
    "state hh([1]): f32[..2]\nClip c:\n    Note/(hh[0])\n",
    "state hh([1]): i32[..2]\nClip c:\n    Note/(hh)\n",
    "state hh([1]): i32[..2]\nClip c:\n    Note/(hh + 1)\n",
    "state hh([1, 2]): i32[..2]\nstate t = hh[0] + hh[1]\nClip c | gain=t:\n    Note/(hh[^1])\n",
    "state hh([]): i32[..2]\nClip c | gain=hh[0]:\n    Note/(hh[^1])\n",
    "state hh([1]): i32[..2]\nstate s = for x in hh into a from 0: a + x\nClip c:\n    Note/(s)\n",
    "stream hh([1]): i32[..2]\nClip c:\n    Note/(hh[0])\n",
    "state hh([1 + true]): i32[..2]\nClip c:\n    Note/(hh[0])\n",
];

const REACTIVE_FOLDS: [&str; 10] = [
    "state n(3)\nClip c:\n    Note/(for i in 0..n into a from 0: a + i)\n",
    "state n(3)\nClip c:\n    Note/(for i in n..4 into a from 0: a + i)\n",
    "state n(3)\nClip c:\n    Note/(for i in 0..(n - 1) into a from 0: a + i)\n",
    "state n(3)\nClip c:\n    Note/(for i in 0..4 into a from n: a + i)\n",
    "state hh([1, 2]): i32[..4]\nstate v = for x in hh into a from []: a + [x]\nClip c:\n    Note/(v[0])\n",
    "state hh([1, 2]): i32[..4]\nstate v = for x in hh into a from []: if x > 1 then a + [x] else a\nClip c:\n    Note/(v[^1])\n",
    "state hh([1, 2]): i32[..4]\nstate v = for x in hh into a from [0]: a + [x, x]\nClip c:\n    Note/(v[0])\n",
    "state s(1)\nClip c:\n    Note/((for i in 0..3 into a from []: a + [s])[0])\n",
    "state hh([1, 2]): i32[..4]\nstate v = for x in hh into a from []: (a + [x])[^2..]\nClip c | gain=v[0]:\n    Note/(v[1])\n",
    "state s(1)\nstate v = [[s]] + [s]\nClip c:\n    Note/(v[0])\n",
];

#[test]
fn a_host_fed_variable_array_cell_and_a_reactive_fold_over_it_or_a_cell_bounded_range_draw_as_the_old_engine_does()
 {
    let vocabulary = symphonyscript_fixtures::vocabulary();
    let old = Compiler::new(&vocabulary)
        .expect("the vocabulary compiles")
        .on_the_old_engine();
    let new = Compiler::new(&vocabulary).expect("the vocabulary compiles");
    for text in VARIABLE_CELLS.iter().chain(REACTIVE_FOLDS.iter()) {
        let sources = [source(text)];
        let found = new
            .checked_by_new_core(&sources)
            .unwrap_or_else(|| panic!("the new core takes {text}"));
        assert_eq!(found, old.check(&sources), "{text}");
    }
    let cases: [(&str, &[Code]); 8] = [
        (VARIABLE_CELLS[0], &[]),
        (VARIABLE_CELLS[1], &[Code::CapacityExceeded]),
        (VARIABLE_CELLS[2], &[Code::TypeMismatch]),
        (VARIABLE_CELLS[4], &[Code::DoesNotFit]),
        (
            "state held([1.5]): i32[..4]\nClip c:\n    Note/(held[0])\n",
            &[Code::TypeMismatch],
        ),
        (REACTIVE_FOLDS[0], &[Code::RangeBound]),
        (
            "state n(3)\nClip c:\n    Note/(for i in 0..4 into a from n: a + i)\n",
            &[],
        ),
        (REACTIVE_FOLDS[4], &[]),
    ];
    for (text, codes) in cases {
        let found = new
            .checked_by_new_core(&[source(text)])
            .unwrap_or_else(|| panic!("the new core takes {text}"));
        let found: Vec<Code> = found
            .iter()
            .map(|diagnostic| diagnostic.code)
            .filter(|code| *code != Code::NoEntryInstance)
            .collect();
        assert_eq!(found, codes, "{text}");
    }
    let doubling = "state hh([1, 2]): i32[..40]\nstate v = for x in hh into a from [0]: a + a\nClip c:\n    Note/(v[0])\n";
    assert!(
        new.checked_by_new_core(&[source(doubling)]).is_none(),
        "{doubling}"
    );
}

const BODY_USES: [(&str, &[Code]); 12] = [
    (
        "state v0([]): i32[..4]\nstate v1([]): f64[..4]\nfn pass(v): v\nClip c:\n    Note/((pass(if true then v0 else v1))[0])\n",
        &[Code::IntegerCell, Code::FloatToInteger],
    ),
    (
        "state v0([]): i32[..4]\nstate v1([]): f64[..4]\nfn h(z):\n    let x = if true then v0 else v1\n    x\nClip c:\n    Note/((h(0))[0])\n",
        &[Code::IntegerCell, Code::FloatToInteger],
    ),
    (
        "state v0([]): i32[..4]\nstate v1([]): f64[..4]\nstate flag(true)\nfn h(z):\n    let x = if flag then v0 else v1\n    x\nClip c:\n    Note/((h(0))[0])\n",
        &[Code::IntegerCell, Code::FloatToInteger],
    ),
    (
        "state v0([]): i32[..4]\nstate v1([]): f64[..4]\nClip c:\n    Note/((if true then v0 else v1)[0])\n",
        &[Code::TypeMismatch],
    ),
    (
        "state v0(1)\nstate v1(1.5)\nstate flag(true)\nfn h(z):\n    let x = if flag then v0 else v1\n    x\nClip c:\n    Note/(h(0))\n",
        &[Code::IntegerCell, Code::FloatToInteger],
    ),
    (
        "state v0(1)\nstate v1(1.5)\nstate flag(true)\nfn pass(v): v\nClip c:\n    Note/(pass(if flag then v0 else v1))\n",
        &[Code::IntegerCell, Code::FloatToInteger],
    ),
    (
        "state v0(1)\nfn h(z):\n    let x = v0 * 1.5\n    x\nClip c:\n    Note/(h(0) + true)\n",
        &[Code::IntegerCell, Code::OperandType],
    ),
    (
        "state v0(1)\nstate v1(1.5)\nstate flag(true)\nfn h(z):\n    let x = if flag then v0 + 300000000000 else v1\n    x\nClip c:\n    Note/(h(0))\n",
        &[Code::IntegerCell, Code::DoesNotFit, Code::FloatToInteger],
    ),
    (
        "state v0(1)\nstate v1(1.5)\nstate flag(true)\nfn h(z): if flag then v0 else v1\nClip c:\n    Note/(h(0))\n",
        &[Code::TypeMismatch],
    ),
    (
        "state v0(1)\nstate v1(1.5)\nstate flag(true)\nClip c:\n    Note/(if flag then v0 else v1)\n",
        &[Code::TypeMismatch],
    ),
    (
        "state v0(1)\nfn h(v): v * 1.5\nClip c | gain=v0:\n    Note/(h(v0))\n",
        &[Code::TypeMismatch],
    ),
    (
        "state v0(1)\nstate v1(1.5)\nstate flag(true)\nfn h(z):\n    let x = if flag then v0 else v1\n    x\nClip c | gain=h(0):\n    Note/(1)\n",
        &[Code::IntegerCell],
    ),
];

#[test]
fn a_local_let_or_an_argument_a_refused_call_reads_reads_its_cells_in_its_own_type_so_a_float_use_there_counts_for_integer_cell()
 {
    let vocabulary = symphonyscript_fixtures::vocabulary();
    let old = Compiler::new(&vocabulary)
        .expect("the vocabulary compiles")
        .on_the_old_engine();
    let new = Compiler::new(&vocabulary).expect("the vocabulary compiles");
    for (text, codes) in BODY_USES {
        let sources = [source(text)];
        let found = new
            .checked_by_new_core(&sources)
            .unwrap_or_else(|| panic!("the new core takes {text}"));
        assert_eq!(found, old.check(&sources), "{text}");
        let found: Vec<Code> = found
            .iter()
            .map(|diagnostic| diagnostic.code)
            .filter(|code| *code != Code::NoEntryInstance)
            .collect();
        assert_eq!(found, codes, "{text}");
    }
}
