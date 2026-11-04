use symphonyscript_core::Vocabulary;

use crate::compiler::Compiler;
use crate::semantics::counted_walk::CountedWalk;
use crate::syntax::front_end_goldens::source;

const VALID_LOOPS: [&str; 9] = [
    "Clip c:\n    for i in 0..3:\n        Note/(60 + i)\n",
    "Clip c:\n    for i in 0..3:\n        Note/(i) | velocity=i\n",
    "let xs = [1, 2, 3]\nClip c:\n    for i, p in xs:\n        Note/(p + i)\n",
    "Clip c:\n    for p in [1.5, 2.5]:\n        Note/1 | velocity=p\n",
    "let n = 3\nClip c:\n    for i in 0..n:\n        Note/i\n",
    "fn f(x): x + 1\nClip c:\n    for i in 0..3:\n        Note/(f(i))\n",
    "Clip c:\n    for i in 0..2:\n        for j in 0..2:\n            Note/(i * 10 + j)\n",
    "Clip c:\n    for x in [[1, 2], [3, 4]]:\n        for y in x:\n            Note/y\n",
    "Clip c:\n    for i in 0..3:\n        Note/1 Note/i\n",
];

const FAULTY_LOOPS: [&str; 22] = [
    "Clip c:\n    for i in 0..3:\n        Note/(i * 3000000000)\n",
    "Clip c:\n    for i in 0..3:\n        Note/(i + true)\n",
    "Clip c:\n    for i in 0..3:\n        Note/1.5\n",
    "Clip c:\n    for p in [1.5, 2.5]:\n        Note/p\n",
    "Clip c:\n    for i in 0..3:\n        Note/([1, 2][i])\n",
    "Clip c:\n    for i in 0..3:\n        Lamp\n",
    "Clip c:\n    for i, j in 0..3:\n        Note/i\n",
    "state s(3)\nClip c:\n    for i in 0..s:\n        Note/i\n",
    "state x([1.5])\nClip c:\n    for p in x:\n        Note/1\n",
    "Clip c:\n    for i in true:\n        Note/1\n",
    "Clip c:\n    for i in 0..2.5:\n        Note/1\n",
    "Clip c:\n    for i in 0..true:\n        Note/1\n",
    "Clip c:\n    for i in [1, 2]..3:\n        Note/1\n",
    "Clip c:\n    for i in 0..(1.5 + true):\n        Note/1\n",
    "Clip c:\n    for i in 0..(1.5 + [1, 2][5]):\n        Note/60\n",
    "Clip c:\n    for i in 0..3:\n        for i in 0..2:\n            Note/i\n",
    "let i = 4\nClip c:\n    for i in 0..3:\n        Note/i\n",
    "Clip c:\n    Note as i\n    for i in 0..3:\n        Note/i\n",
    "Clip c:\n    for i in 0..300000000000:\n        Note/1\n",
    "Clip c:\n    for i in 0..3:\n        Note/(i)\n    Note/zz\n",
    "Clip c:\n    for i in 0..0:\n        Note | loud=1\n",
    "Clip c:\n    for i in 0..0:\n        for j in 0..3:\n            Zap | loud=1\n",
];

const UNRUN_TWINS: [&str; 4] = [
    "Clip c:\n    for i in 0..0:\n        Note/(i + true)\n",
    "Clip c:\n    for i in 0..0:\n        Note/1 | pitch=2\n",
    "Clip c:\n    for i in 0..0:\n        Clip/1\n",
    "Clip c:\n    for i in 0..0:\n        Note | velocity=(1 + true)\n",
];

fn assert_engines_agree(vocabulary: &Vocabulary, programs: &[&str]) {
    let old = Compiler::new(vocabulary)
        .expect("the vocabulary compiles")
        .on_the_old_engine();
    let new = Compiler::new(vocabulary).expect("the vocabulary compiles");
    for text in programs {
        let sources = [source(text)];
        let found = new
            .checked_by_new_core(&sources)
            .unwrap_or_else(|| panic!("the new core takes {text}"));
        assert_eq!(found, old.check(&sources), "{text}");
    }
}

#[test]
fn a_placement_for_over_a_range_or_an_array_checks_as_the_old_engine_does() {
    let vocabulary = symphonyscript_fixtures::vocabulary();
    assert_engines_agree(&vocabulary, &VALID_LOOPS);
    assert_engines_agree(&vocabulary, &FAULTY_LOOPS);
}

#[test]
fn the_body_of_a_placement_for_that_runs_no_iteration_has_its_names_resolved_and_nothing_else() {
    let vocabulary = symphonyscript_fixtures::vocabulary();
    let new = Compiler::new(&vocabulary).expect("the vocabulary compiles");
    for text in UNRUN_TWINS {
        let found = new
            .checked_by_new_core(&[source(text)])
            .unwrap_or_else(|| panic!("the new core takes {text}"));
        let codes: Vec<_> = found.iter().map(|diagnostic| diagnostic.code).collect();
        assert_eq!(
            codes,
            vec![crate::diagnostics::Code::NoEntryInstance],
            "{text}"
        );
    }
    assert_engines_agree(&vocabulary, &UNRUN_TWINS);
}

#[test]
fn a_placement_for_of_a_thousand_iterations_types_its_body_once() {
    let vocabulary = symphonyscript_fixtures::vocabulary();
    let new = Compiler::new(&vocabulary).expect("the vocabulary compiles");
    let text = "Clip c:\n    for i in 0..1000:\n        Note/(60 + i)\n";
    let bodies = CountedWalk::CoreLoopBody.steps();
    let iterations = CountedWalk::CoreLoopIteration.steps();
    let found = new
        .checked_by_new_core(&[source(text)])
        .expect("the new core takes the loop");
    assert_eq!(CountedWalk::CoreLoopBody.steps() - bodies, 1);
    assert_eq!(CountedWalk::CoreLoopIteration.steps() - iterations, 1000);
    assert!(
        found
            .iter()
            .all(|diagnostic| diagnostic.code == crate::diagnostics::Code::NoEntryInstance)
    );
    let text = "Clip c:\n    for p in [1, 2.5]:\n        Note/1 | velocity=p\n    for i in 0..3:\n        Note/i\n";
    let bodies = CountedWalk::CoreLoopBody.steps();
    new.checked_by_new_core(&[source(text)])
        .expect("the new core takes the loops");
    assert_eq!(CountedWalk::CoreLoopBody.steps() - bodies, 2);
}
