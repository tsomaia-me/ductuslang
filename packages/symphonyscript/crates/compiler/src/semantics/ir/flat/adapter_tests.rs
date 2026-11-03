use symphonyscript_core::Vocabulary;
use symphonyscript_fixtures::{covered, prelude_vocabulary, spec_examples};

use super::{FlatIr, adapt_file, build_flat_file};
use crate::semantics::ir::{Hir, build_file};
use crate::source::Source;
use crate::syntax::front_end_goldens::{
    Corpus, covered_mutants, deep_corpus, fixture_corpus, generated_corpus, large_corpus,
    nest_corpus, parser_test_corpus, robustness_quarter, source, spec_and_lesson_corpora,
};
use crate::syntax::parse;

struct Preludes {
    old: Hir,
    new: Hir,
    ir: FlatIr,
}

fn preludes(vocabulary: &Vocabulary) -> Result<Preludes, String> {
    let mut old = Hir::default();
    let mut new = Hir::default();
    let mut ir = FlatIr::default();
    let mut files: Vec<Source> = vocabulary
        .prelude()
        .iter()
        .map(|file| Source {
            path: file.path.clone(),
            text: file.text.clone(),
        })
        .collect();
    files.sort_by(|a, b| a.path.as_bytes().cmp(b.path.as_bytes()));
    for file in &files {
        let parsed = parse(file, vocabulary);
        build_file(&mut old, &file.path, &parsed, vocabulary)
            .map_err(|error| format!("{error:?}"))?;
        let at = ir.files.len();
        build_flat_file(&mut ir, &file.path, &parsed, vocabulary)
            .and_then(|()| adapt_file(&mut new, &ir, at))
            .map_err(|error| format!("{error:?}"))?;
    }
    old.seal_prelude();
    new.seal_prelude();
    ir.freeze();
    Ok(Preludes { old, new, ir })
}

fn first_difference(old: &Hir, new: &Hir) -> Option<String> {
    macro_rules! vectors {
        ($($field:ident),*) => {
            $(
                if old.$field != new.$field {
                    let at = old
                        .$field
                        .iter()
                        .zip(new.$field.iter())
                        .position(|(left, right)| left != right)
                        .unwrap_or(old.$field.len().min(new.$field.len()));
                    return Some(format!(
                        "{} at {at} (lengths {} and {}): old {:?}, new {:?}",
                        stringify!($field),
                        old.$field.len(),
                        new.$field.len(),
                        old.$field.get(at),
                        new.$field.get(at)
                    ));
                }
            )*
        };
    }
    vectors!(
        files,
        exprs,
        blocks,
        placements,
        wires,
        inputs,
        lets,
        states,
        fns,
        patterns,
        top,
        drawn,
        placeholders
    );
    if old.skipped_top != new.skipped_top {
        return Some(format!(
            "skipped_top: old {:?}, new {:?}",
            old.skipped_top, new.skipped_top
        ));
    }
    if old.faulty_names != new.faulty_names {
        return Some(format!(
            "faulty_names: old {:?}, new {:?}",
            old.faulty_names, new.faulty_names
        ));
    }
    if old.prelude != new.prelude {
        return Some(format!(
            "prelude: old {:?}, new {:?}",
            old.prelude, new.prelude
        ));
    }
    (old != new).then(|| "the two differ outside every listed field".to_string())
}

fn compare(text: &str, vocabulary: &Vocabulary, preludes: &Preludes) -> Option<String> {
    let parsed = parse(&source(text), vocabulary);
    let mut old = preludes.old.clone();
    if let Err(error) = build_file(&mut old, "main.sym", &parsed, vocabulary) {
        return Some(format!("the old builder failed: {error:?}"));
    }
    let mut new = preludes.new.clone();
    let mut ir = FlatIr::after(&preludes.ir);
    if let Err(error) = build_flat_file(&mut ir, "main.sym", &parsed, vocabulary)
        .and_then(|()| adapt_file(&mut new, &ir, 0))
    {
        return Some(format!("the IR path failed: {error:?}"));
    }
    first_difference(&old, &new)
}

fn checked(corpora: &[Corpus]) -> usize {
    let mut failures = Vec::new();
    let mut count = 0;
    for corpus in corpora {
        let preludes = match preludes(&corpus.vocabulary) {
            Ok(preludes) => preludes,
            Err(error) => {
                failures.push(format!("{}: the prelude: {error}", corpus.name));
                continue;
            }
        };
        for text in &corpus.texts {
            count += 1;
            if let Some(difference) = compare(text, &corpus.vocabulary, &preludes) {
                failures.push(crate::pinned_row::failed_counts(
                    text,
                    &[],
                    format!("{}: {difference}", corpus.name),
                ));
            }
        }
    }
    crate::pinned_row::assert_no_failure(&failures);
    count
}

#[test]
fn the_adapters_hir_equals_the_old_builders_on_every_spec_and_lesson_example() {
    assert_eq!(checked(&spec_and_lesson_corpora()), 404);
}

#[test]
fn the_adapters_hir_equals_the_old_builders_on_the_fixture_projects_and_the_parser_tests() {
    assert_eq!(checked(&[fixture_corpus(), parser_test_corpus()]), 737);
}

#[test]
fn the_adapters_hir_equals_the_old_builders_on_the_covered_robustness_mutants() {
    assert_eq!(checked(&covered(covered_mutants)), 1_355);
}

#[test]
fn the_adapters_hir_equals_the_old_builders_beside_a_prelude_of_every_spec_example() {
    let mut corpora = Vec::new();
    for example in spec_examples().into_iter().step_by(2) {
        corpora.push(Corpus {
            name: "prelude",
            vocabulary: prelude_vocabulary(&example.text),
            texts: vec!["Clip c:\n    Note/60\n".to_string()],
            bases: Vec::new(),
        });
    }
    assert!(checked(&corpora) > 50);
}

#[test]
#[ignore = "stress"]
fn stress_the_adapters_hir_equals_the_old_builders_on_the_robustness_corpus() {
    let run = || {
        let mut count = 0;
        for quarter in 0..4 {
            count += checked(&robustness_quarter(quarter));
        }
        count
    };
    let count = std::thread::Builder::new()
        .stack_size(1 << 20)
        .spawn(run)
        .map(|handle| match handle.join() {
            Ok(count) => count,
            Err(panic) => std::panic::resume_unwind(panic),
        })
        .unwrap_or(0);
    assert!(count > 40_000, "{count}");
}

#[test]
#[ignore = "stress"]
fn stress_the_adapters_hir_equals_the_old_builders_on_the_large_and_deep_projects() {
    let run = || {
        checked(&[
            large_corpus(),
            nest_corpus(),
            generated_corpus(),
            deep_corpus(0),
            deep_corpus(1),
            deep_corpus(2),
            deep_corpus(3),
        ])
    };
    let count = std::thread::Builder::new()
        .stack_size(1 << 20)
        .spawn(run)
        .map(|handle| match handle.join() {
            Ok(count) => count,
            Err(panic) => std::panic::resume_unwind(panic),
        })
        .unwrap_or(0);
    assert_eq!(count, 66);
}

fn string_literals(source: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut chars = source.chars().peekable();
    let mut previous = ' ';
    while let Some(c) = chars.next() {
        if c == '"' && previous != '\'' && previous != 'r' && previous != '#' {
            let mut text = String::new();
            while let Some(c) = chars.next() {
                match c {
                    '"' => break,
                    '\\' => match chars.next() {
                        Some('n') => text.push('\n'),
                        Some('t') => text.push('\t'),
                        Some('r') => text.push('\r'),
                        Some('\n') => {
                            while chars.peek().is_some_and(|c| c.is_whitespace()) {
                                chars.next();
                            }
                        }
                        Some(other) => text.push(other),
                        None => {}
                    },
                    other => text.push(other),
                }
            }
            if text.contains('\n') {
                found.push(text);
            }
            previous = '"';
            continue;
        }
        previous = c;
    }
    found
}

#[test]
fn the_adapters_hir_equals_the_old_builders_on_every_program_the_syntax_tests_write() {
    let folder = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    let mut texts = Vec::new();
    for name in [
        "typed_view.rs",
        "productions.rs",
        "recovery.rs",
        "robustness.rs",
        "lossless.rs",
        "views.rs",
    ] {
        let source = std::fs::read_to_string(folder.join(name)).unwrap_or_default();
        texts.extend(string_literals(&source));
    }
    let count = texts.len();
    let corpus = Corpus {
        name: "syntax-tests",
        vocabulary: symphonyscript_fixtures::vocabulary(),
        texts,
        bases: Vec::new(),
    };
    assert_eq!(checked(&[corpus]), count);
    assert!(count > 300, "{count}");
}
