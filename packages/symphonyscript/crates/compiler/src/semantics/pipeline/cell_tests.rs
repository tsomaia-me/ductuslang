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
