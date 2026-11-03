use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use symphonyscript_core::Vocabulary;
use symphonyscript_fixtures::{
    Project, Rows, control_project, covered, deep_projects, document_examples, dsp_project,
    large_project, lighting_project, music_project, spec_examples, vocabulary,
};

use super::Parsed;
use crate::semantics::ir::{Hir, build_file};
use crate::source::Source;

#[path = "../../tests/robustness_corpus/mod.rs"]
#[allow(dead_code, clippy::duplicate_mod)]
mod robustness_corpus;

pub(crate) struct Corpus {
    pub name: &'static str,
    pub vocabulary: Vocabulary,
    pub texts: Vec<String>,
    pub bases: Vec<(u64, usize)>,
}

impl Corpus {
    fn new(name: &'static str, texts: Vec<String>) -> Self {
        Self {
            name,
            vocabulary: vocabulary(),
            texts,
            bases: Vec::new(),
        }
    }
}

fn lesson_texts() -> Vec<String> {
    let folder = Path::new(env!("CARGO_MANIFEST_DIR")).join("lessons");
    let mut paths: Vec<PathBuf> = std::fs::read_dir(&folder)
        .expect("the lessons folder reads")
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.extension().is_some_and(|extension| extension == "md"))
        .collect();
    paths.sort();
    paths
        .iter()
        .flat_map(|path| {
            let text = std::fs::read_to_string(path).expect("a lesson reads");
            document_examples(&text)
        })
        .filter(|example| example.tag.starts_with("sym"))
        .map(|example| example.text)
        .collect()
}

fn project_texts(projects: impl IntoIterator<Item = Project>) -> Vec<String> {
    projects
        .into_iter()
        .flat_map(|project| project.files)
        .map(|file| file.text)
        .collect()
}

fn lexed_spans(text: &str, vocabulary: &Vocabulary) -> Vec<((usize, usize), usize)> {
    super::lex(&source(text), vocabulary)
        .tokens
        .iter()
        .filter_map(|token| {
            let start = usize::try_from(token.range.start).ok()?;
            let end = usize::try_from(token.range.end).ok()?;
            Some(((start, end), token.kind as usize))
        })
        .collect()
}

fn robustness(keep: impl Fn(usize, usize, usize) -> bool, name: &'static str) -> Vec<Corpus> {
    robustness_corpus::corpora()
        .into_iter()
        .enumerate()
        .map(|(corpus, (vocabulary, texts))| {
            let mut mutants = Vec::new();
            let mut bases = Vec::new();
            for text in &texts {
                let before = mutants.len();
                for (span, kind) in lexed_spans(text, &vocabulary) {
                    for mutation in 0..robustness_corpus::MUTATIONS {
                        if keep(corpus, kind, mutation)
                            && let Some(mutant) = robustness_corpus::mutant(text, span, mutation)
                        {
                            mutants.push(mutant);
                        }
                    }
                }
                bases.push((digest(text.as_bytes()), mutants.len() - before));
            }
            Corpus {
                name,
                vocabulary,
                texts: mutants,
                bases,
            }
        })
        .collect()
}

pub(crate) fn covered_mutants(rows: &Rows) -> Vec<Corpus> {
    robustness(
        |corpus, kind, mutation| rows.take(&[corpus, kind, mutation]),
        "robustness-covered",
    )
}

pub(crate) fn source(text: &str) -> Source {
    Source {
        path: "main.sym".into(),
        text: text.into(),
    }
}

fn hir_of(parsed: &Parsed, vocabulary: &Vocabulary) -> Result<Hir, String> {
    let mut hir = Hir::default();
    build_file(&mut hir, "main.sym", parsed, vocabulary)
        .map_err(|internal| format!("{internal:?}"))?;
    Ok(hir)
}

fn digest(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x0100_0000_01b3)
    })
}

fn without_vocabulary_tokens(debug: &str) -> String {
    const MARK: &str = "VocabularyToken(";
    let mut kept = String::with_capacity(debug.len());
    let mut rest = debug;
    while let Some(at) = rest.find(MARK) {
        kept.push_str(&rest[..at + MARK.len()]);
        rest = rest[at + MARK.len()..].trim_start_matches(|c: char| c.is_ascii_digit());
    }
    kept.push_str(rest);
    kept
}

fn without_cut_marks(hir: &Hir) -> Hir {
    let mut kept = hir.clone();
    for expr in &mut kept.exprs {
        if let crate::semantics::ir::ExprKind::Call(_, args) = &mut expr.kind
            && let crate::semantics::ir::Args::Cut(inner) = args
        {
            let whole = std::mem::replace(
                inner.as_mut(),
                crate::semantics::ir::Args::Positional(Vec::new()),
            );
            *args = whole;
        }
    }
    kept
}

fn golden_line(text: &str, hir: &Hir) -> String {
    let hir = &without_cut_marks(hir);
    let placeholders: Vec<String> = hir
        .placeholders
        .iter()
        .map(|ident| format!("{ident:?}"))
        .collect();
    format!(
        "{:016x} {:016x} {} {:016x}",
        digest(text.as_bytes()),
        digest(without_vocabulary_tokens(&format!("{hir:?}")).as_bytes()),
        placeholders.len(),
        digest(placeholders.join("\n").as_bytes()),
    )
}
fn recorded(text: &str, corpus: &Corpus) -> Result<String, String> {
    let parsed = super::parse(&source(text), &corpus.vocabulary);
    let hir = hir_of(&parsed, &corpus.vocabulary)?;
    Ok(golden_line(text, &hir))
}

fn golden_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("front_end_goldens")
        .join(format!("{name}.txt"))
}

fn golden(name: &str) -> BTreeMap<String, String> {
    std::fs::read_to_string(golden_path(name))
        .unwrap_or_default()
        .lines()
        .filter_map(|line| {
            let (program, rest) = line.split_once(' ')?;
            Some((program.to_string(), rest.to_string()))
        })
        .collect()
}

fn entries(corpus: &Corpus, failures: &mut Vec<String>) -> Vec<(String, String)> {
    let mut lines = Vec::new();
    for text in &corpus.texts {
        match recorded(text, corpus) {
            Ok(line) => {
                let (program, rest) = line.split_once(' ').unwrap_or_default();
                lines.push((program.to_string(), rest.to_string()));
            }
            Err(found) => failures.push(crate::pinned_row::failed_counts(
                text,
                &[],
                format!("{}: {found}", corpus.name),
            )),
        }
    }
    if corpus.bases.is_empty() {
        return lines;
    }
    let mut grouped = Vec::new();
    let mut rest = lines.as_slice();
    for (index, (base, count)) in corpus.bases.iter().enumerate() {
        let (group, after) = rest.split_at((*count).min(rest.len()));
        rest = after;
        let joined: String = group
            .iter()
            .map(|(program, line)| format!("{program} {line}\n"))
            .collect();
        grouped.push((
            format!("{index:05}-{base:016x}"),
            format!("{count} {:016x}", digest(joined.as_bytes())),
        ));
    }
    grouped
}

fn checked(corpora: &[Corpus]) -> usize {
    let mut failures = Vec::new();
    let mut checked = 0;
    for corpus in corpora {
        let expected = golden(corpus.name);
        checked += corpus.texts.len();
        for (key, value) in entries(corpus, &mut failures) {
            match expected.get(&key) {
                Some(recorded) if *recorded == value => {}
                recorded => failures.push(format!(
                    "{}: {key} {value} against the golden {recorded:?}",
                    corpus.name
                )),
            }
        }
    }
    crate::pinned_row::assert_no_failure(&failures);
    checked
}

pub(crate) fn spec_and_lesson_corpora() -> [Corpus; 2] {
    let examples = spec_examples()
        .into_iter()
        .map(|example| example.text)
        .collect();
    [
        Corpus::new("spec", examples),
        Corpus::new("lessons", lesson_texts()),
    ]
}

pub(crate) fn fixture_corpus() -> Corpus {
    let texts = project_texts([
        music_project(),
        dsp_project(),
        lighting_project(),
        control_project(),
    ]);
    Corpus::new("fixtures", texts)
}

pub(crate) fn parser_test_corpus() -> Corpus {
    Corpus::new(
        "parser-tests",
        super::parser::tests::item_programs(&vocabulary()),
    )
}

pub(crate) fn generated_corpus() -> Corpus {
    Corpus::new(
        "parser-generated",
        super::parser::tests::generated_programs(&vocabulary()),
    )
}

pub(crate) fn large_corpus() -> Corpus {
    Corpus::new("large-project", project_texts([large_project()]))
}

pub(crate) fn deep_corpus(project: usize) -> Corpus {
    let texts = project_texts(deep_projects().into_iter().skip(project).take(1));
    Corpus::new(DEEP_NAMES[project], texts)
}

const DEEP_NAMES: [&str; 4] = [
    "deep-project-1",
    "deep-project-2",
    "deep-project-3",
    "deep-project-4",
];

pub(crate) fn nest_corpus() -> Corpus {
    Corpus::new("parser-nests", super::parser::tests::nest_programs(2_000))
}

const QUARTERS: [&str; 4] = [
    "robustness-first",
    "robustness-second",
    "robustness-third",
    "robustness-fourth",
];

pub(crate) fn robustness_quarter(quarter: usize) -> Vec<Corpus> {
    let name = QUARTERS[quarter];
    robustness(|_, _, _| true, name)
        .into_iter()
        .map(|mut corpus| {
            let bases = std::mem::take(&mut corpus.bases);
            let mut kept = Vec::new();
            let mut texts = std::mem::take(&mut corpus.texts).into_iter();
            for (index, (base, count)) in bases.into_iter().enumerate() {
                let group: Vec<String> = texts.by_ref().take(count).collect();
                if index % QUARTERS.len() == quarter {
                    corpus.bases.push((base, count));
                    kept.extend(group);
                }
            }
            corpus.texts = kept;
            corpus
        })
        .collect()
}

#[test]
fn the_front_end_keeps_its_recorded_hir_and_placeholders_on_every_spec_and_lesson_example() {
    assert_eq!(checked(&spec_and_lesson_corpora()), 404);
}

#[test]
fn the_front_end_keeps_its_recorded_hir_and_placeholders_on_the_fixture_projects() {
    assert_eq!(checked(&[fixture_corpus()]), 7);
}

#[test]
fn the_front_end_keeps_its_recorded_hir_and_placeholders_on_the_parser_tests_programs() {
    assert_eq!(checked(&[parser_test_corpus()]), 730);
}

#[test]
fn the_front_end_keeps_its_recorded_hir_and_placeholders_on_the_covered_robustness_mutants() {
    assert_eq!(checked(&covered(covered_mutants)), 1_355);
}

#[test]
#[ignore = "stress"]
fn stress_the_front_end_keeps_its_recorded_hir_and_placeholders_on_the_first_quarter_of_the_robustness_corpus()
 {
    assert!(checked(&robustness_quarter(0)) > 10_000);
}

#[test]
#[ignore = "stress"]
fn stress_the_front_end_keeps_its_recorded_hir_and_placeholders_on_the_second_quarter_of_the_robustness_corpus()
 {
    assert!(checked(&robustness_quarter(1)) > 10_000);
}

#[test]
#[ignore = "stress"]
fn stress_the_front_end_keeps_its_recorded_hir_and_placeholders_on_the_third_quarter_of_the_robustness_corpus()
 {
    assert!(checked(&robustness_quarter(2)) > 10_000);
}

#[test]
#[ignore = "stress"]
fn stress_the_front_end_keeps_its_recorded_hir_and_placeholders_on_the_fourth_quarter_of_the_robustness_corpus()
 {
    assert!(checked(&robustness_quarter(3)) > 10_000);
}

#[test]
#[ignore = "stress"]
fn stress_the_front_end_keeps_its_recorded_hir_and_placeholders_on_the_large_project_the_nests_and_the_generated_files()
 {
    assert_eq!(
        checked(&[large_corpus(), nest_corpus(), generated_corpus()]),
        62
    );
}

#[test]
#[ignore = "stress"]
fn stress_the_front_end_keeps_its_recorded_hir_and_placeholders_on_the_deep_expressions_and_bodies()
{
    assert_eq!(checked(&[deep_corpus(0), deep_corpus(1)]), 2);
}

#[test]
#[ignore = "stress"]
fn stress_the_front_end_keeps_its_recorded_hir_and_placeholders_on_the_deep_array_type_and_calls() {
    assert_eq!(checked(&[deep_corpus(2), deep_corpus(3)]), 2);
}
