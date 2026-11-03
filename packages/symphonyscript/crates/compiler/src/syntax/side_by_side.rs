use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use symphonyscript_core::Vocabulary;
use symphonyscript_fixtures::{
    Project, Rows, control_project, covered, deep_projects, document_examples, dsp_project,
    large_project, lighting_project, music_project, spec_examples, vocabulary,
};

use super::rowan_oracle;
use super::token_columns::TokenColumns;
use super::tree_columns::TreeColumns;
use super::{ElementView, Parsed, Walk};
use crate::semantics::ir::{Hir, build_file};
use crate::source::Source;

#[path = "../../tests/robustness_corpus/mod.rs"]
#[allow(dead_code, clippy::duplicate_mod)]
mod robustness_corpus;

const SHALLOW: usize = 256;
const STRIDE: usize = 997;

type Element = (bool, u16, u32, u32);

#[derive(Clone, Copy, PartialEq, Eq)]
enum Offsets {
    EveryByte,
    TokenEdges,
    TokenStarts,
}

#[derive(PartialEq, Eq, Debug, Default)]
struct Walks {
    preorder: Vec<(bool, Element)>,
    children: Vec<Vec<Element>>,
    elements: Vec<Vec<Element>>,
    parents: Vec<Option<Element>>,
    ancestors: Vec<Vec<Element>>,
    edges: Vec<(Option<Element>, Option<Element>)>,
    at_offset: Vec<(u32, Vec<Element>)>,
    forward: Vec<Element>,
    backward: Vec<Element>,
    text: String,
}

macro_rules! node {
    ($node:expr) => {{
        let range = $node.range();
        (true, $node.kind() as u16, range.start, range.end)
    }};
}

macro_rules! token {
    ($token:expr) => {{
        let range = $token.range();
        (false, $token.kind() as u16, range.start, range.end)
    }};
}

macro_rules! walks {
    ($root:expr, $element:ident, $walk:ident, $offsets:expr) => {{
        let root = $root;
        let mut walks = Walks::default();
        let mut nodes = Vec::new();
        let mut depth = 0usize;
        let mut deepest = 0usize;
        for event in root.preorder() {
            match event {
                $walk::Enter($element::Node(node)) => {
                    walks.preorder.push((true, node!(node)));
                    nodes.push(node);
                    depth += 1;
                    deepest = deepest.max(depth);
                }
                $walk::Enter($element::Token(token)) => walks.preorder.push((true, token!(token))),
                $walk::Leave($element::Node(node)) => {
                    walks.preorder.push((false, node!(node)));
                    depth -= 1;
                }
                $walk::Leave($element::Token(token)) => walks.preorder.push((false, token!(token))),
            }
        }
        let stride = if deepest <= SHALLOW { 1 } else { STRIDE };
        for (index, node) in nodes.iter().enumerate() {
            walks
                .children
                .push(node.children().map(|child| node!(child)).collect());
            walks.elements.push(
                node.children_with_tokens()
                    .map(|element| match element {
                        $element::Node(child) => node!(child),
                        $element::Token(token) => token!(token),
                    })
                    .collect(),
            );
            walks
                .parents
                .push(node.parent().map(|parent| node!(parent)));
            if index % stride == 0 {
                walks
                    .ancestors
                    .push(node.ancestors().map(|ancestor| node!(ancestor)).collect());
                walks.edges.push((
                    node.first_token().map(|token| token!(token)),
                    node.last_token().map(|token| token!(token)),
                ));
            }
        }
        for offset in $offsets.iter().step_by(stride) {
            walks.at_offset.push((
                *offset,
                root.token_at_offset(*offset)
                    .iter()
                    .map(|token| token!(token))
                    .collect(),
            ));
        }
        let mut next = root.first_token();
        while let Some(token) = next {
            walks.forward.push(token!(token));
            next = token.next_token();
        }
        let mut previous = root.last_token();
        while let Some(token) = previous {
            walks.backward.push(token!(token));
            previous = token.prev_token();
        }
        walks.text = root.text();
        walks
    }};
}

struct Corpus {
    name: &'static str,
    vocabulary: Vocabulary,
    texts: Vec<String>,
    offsets: Offsets,
    bases: Vec<(u64, usize)>,
}

impl Corpus {
    fn new(name: &'static str, texts: Vec<String>, offsets: Offsets) -> Self {
        Self {
            name,
            vocabulary: vocabulary(),
            texts,
            offsets,
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
                offsets: Offsets::TokenStarts,
                bases,
            }
        })
        .collect()
}

fn covered_mutants(rows: &Rows) -> Vec<Corpus> {
    robustness(
        |corpus, kind, mutation| rows.take(&[corpus, kind, mutation]),
        "robustness-covered",
    )
}

fn source(text: &str) -> Source {
    Source {
        path: "main.sym".into(),
        text: text.into(),
    }
}

fn converted(oracle: &rowan_oracle::Parsed, text: &str) -> Parsed {
    let mut tokens = TokenColumns::default();
    let mut tree = TreeColumns::default();
    let mut open: Vec<(u32, u32)> = Vec::new();
    let count = |columns: &TokenColumns| u32::try_from(columns.len()).unwrap_or(u32::MAX);
    for event in oracle.root().preorder() {
        match event {
            rowan_oracle::Walk::Enter(rowan_oracle::ElementView::Node(_)) => {
                open.push((
                    count(&tokens),
                    u32::try_from(tree.len()).unwrap_or(u32::MAX),
                ));
            }
            rowan_oracle::Walk::Leave(rowan_oracle::ElementView::Node(node)) => {
                let (first, first_desc) = open.pop().unwrap_or_default();
                tree.push(node.kind(), (first, count(&tokens)), first_desc);
            }
            rowan_oracle::Walk::Enter(rowan_oracle::ElementView::Token(token)) => {
                if !token.kind().is_trivia() {
                    tokens.sig.push(count(&tokens));
                }
                tokens.kinds.push(token.kind());
                tokens.starts.push(token.range().start);
            }
            rowan_oracle::Walk::Leave(rowan_oracle::ElementView::Token(_)) => {}
        }
    }
    tokens
        .starts
        .push(u32::try_from(text.len()).unwrap_or(u32::MAX));
    Parsed::new(
        text.to_string(),
        tokens,
        tree,
        oracle.diagnostics.clone(),
        oracle.detached.clone(),
    )
}

fn offsets(text: &str, parsed: &Parsed, mode: Offsets) -> Vec<u32> {
    let end = u32::try_from(text.len()).unwrap_or(u32::MAX);
    match mode {
        Offsets::EveryByte => (0..=end + 1).collect(),
        Offsets::TokenEdges => {
            let mut found: Vec<u32> = parsed
                .tokens()
                .starts
                .windows(2)
                .flat_map(|pair| {
                    let (start, stop) = (pair[0], pair[1]);
                    [start, start + (stop - start) / 2, stop]
                })
                .collect();
            found.push(end + 1);
            found.dedup();
            found
        }
        Offsets::TokenStarts => parsed.tokens().starts.clone(),
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

fn golden_line(text: &str, hir: &Hir) -> String {
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

fn mismatch(text: &str, corpus: &Corpus) -> Result<String, String> {
    let vocabulary = &corpus.vocabulary;
    let source = source(text);
    let lexed = super::lex(&source, vocabulary);
    let oracle_lexed = rowan_oracle::lex(&source, vocabulary);
    if lexed != oracle_lexed {
        return Err(format!(
            "lexed {}, diagnostics {:?} against {:?}",
            first_difference(&lexed.tokens, &oracle_lexed.tokens),
            lexed.diagnostics,
            oracle_lexed.diagnostics
        ));
    }
    let parsed = super::parse(&source, vocabulary);
    let oracle = rowan_oracle::parse(&source, vocabulary);
    if parsed.diagnostics != oracle.diagnostics {
        return Err(format!(
            "diagnostics {:?}\n  oracle {:?}",
            parsed.diagnostics, oracle.diagnostics
        ));
    }
    if parsed.detached != oracle.detached {
        return Err(format!(
            "detached {:?}\n  oracle {:?}",
            parsed.detached, oracle.detached
        ));
    }
    let offsets = offsets(text, &parsed, corpus.offsets);
    let new = walks!(parsed.root(), ElementView, Walk, offsets);
    let old = {
        use rowan_oracle::{ElementView, Walk};
        walks!(oracle.root(), ElementView, Walk, offsets)
    };
    if let Some(found) = walk_mismatch(&new, &old) {
        return Err(found);
    }
    let old_parsed = converted(&oracle, text);
    if old_parsed != parsed {
        return Err(String::from("the columns differ from the oracle's tree"));
    }
    let hir = hir_of(&parsed, vocabulary)?;
    let old_hir = hir_of(&old_parsed, vocabulary)?;
    if hir != old_hir {
        return Err(format!("hir {hir:?}\n  oracle {old_hir:?}"));
    }
    Ok(golden_line(text, &hir))
}

fn first_difference<T: PartialEq + std::fmt::Debug>(new: &[T], old: &[T]) -> String {
    let at = new
        .iter()
        .zip(old)
        .position(|(left, right)| left != right)
        .unwrap_or(new.len().min(old.len()));
    format!(
        "at {at} of {} and {}: {:?} against {:?}",
        new.len(),
        old.len(),
        new.get(at),
        old.get(at)
    )
}

fn walk_mismatch(new: &Walks, old: &Walks) -> Option<String> {
    let parts: [(&str, bool, String); 10] = [
        (
            "preorder",
            new.preorder == old.preorder,
            first_difference(&new.preorder, &old.preorder),
        ),
        (
            "children",
            new.children == old.children,
            first_difference(&new.children, &old.children),
        ),
        (
            "children_with_tokens",
            new.elements == old.elements,
            first_difference(&new.elements, &old.elements),
        ),
        (
            "parent",
            new.parents == old.parents,
            first_difference(&new.parents, &old.parents),
        ),
        (
            "ancestors",
            new.ancestors == old.ancestors,
            first_difference(&new.ancestors, &old.ancestors),
        ),
        (
            "first and last token",
            new.edges == old.edges,
            first_difference(&new.edges, &old.edges),
        ),
        (
            "token_at_offset",
            new.at_offset == old.at_offset,
            first_difference(&new.at_offset, &old.at_offset),
        ),
        (
            "next_token",
            new.forward == old.forward,
            first_difference(&new.forward, &old.forward),
        ),
        (
            "prev_token",
            new.backward == old.backward,
            first_difference(&new.backward, &old.backward),
        ),
        (
            "text",
            new.text == old.text,
            first_difference(new.text.as_bytes(), old.text.as_bytes()),
        ),
    ];
    parts
        .into_iter()
        .find(|(_, equal, _)| !equal)
        .map(|(name, _, difference)| format!("{name} {difference}"))
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
        match mismatch(text, corpus) {
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

fn spec_and_lesson_corpora() -> [Corpus; 2] {
    let examples = spec_examples()
        .into_iter()
        .map(|example| example.text)
        .collect();
    [
        Corpus::new("spec", examples, Offsets::EveryByte),
        Corpus::new("lessons", lesson_texts(), Offsets::EveryByte),
    ]
}

fn fixture_corpus() -> Corpus {
    let texts = project_texts([
        music_project(),
        dsp_project(),
        lighting_project(),
        control_project(),
    ]);
    Corpus::new("fixtures", texts, Offsets::TokenEdges)
}

fn parser_test_corpus() -> Corpus {
    Corpus::new(
        "parser-tests",
        super::parser::tests::item_programs(&vocabulary()),
        Offsets::TokenEdges,
    )
}

fn generated_corpus() -> Corpus {
    Corpus::new(
        "parser-generated",
        super::parser::tests::generated_programs(&vocabulary()),
        Offsets::TokenStarts,
    )
}

fn large_corpus() -> Corpus {
    Corpus::new(
        "large-project",
        project_texts([large_project()]),
        Offsets::TokenEdges,
    )
}

fn deep_corpus(project: usize) -> Corpus {
    let texts = project_texts(deep_projects().into_iter().skip(project).take(1));
    Corpus::new(DEEP_NAMES[project], texts, Offsets::TokenEdges)
}

const DEEP_NAMES: [&str; 4] = [
    "deep-project-1",
    "deep-project-2",
    "deep-project-3",
    "deep-project-4",
];

fn nest_corpus() -> Corpus {
    Corpus::new(
        "parser-nests",
        super::parser::tests::nest_programs(2_000),
        Offsets::TokenEdges,
    )
}

const QUARTERS: [&str; 4] = [
    "robustness-first",
    "robustness-second",
    "robustness-third",
    "robustness-fourth",
];

fn robustness_quarter(quarter: usize) -> Vec<Corpus> {
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
fn the_front_end_matches_the_rowan_oracle_on_every_spec_and_lesson_example() {
    assert_eq!(checked(&spec_and_lesson_corpora()), 404);
}

#[test]
fn the_front_end_matches_the_rowan_oracle_on_the_fixture_projects() {
    assert_eq!(checked(&[fixture_corpus()]), 7);
}

#[test]
fn the_front_end_matches_the_rowan_oracle_on_the_parser_tests_programs() {
    assert_eq!(checked(&[parser_test_corpus()]), 730);
}

#[test]
fn the_front_end_matches_the_rowan_oracle_on_the_covered_robustness_mutants() {
    assert_eq!(checked(&covered(covered_mutants)), 1_355);
}

#[test]
#[ignore = "stress"]
fn stress_the_front_end_matches_the_rowan_oracle_on_the_first_quarter_of_the_robustness_corpus() {
    assert!(checked(&robustness_quarter(0)) > 10_000);
}

#[test]
#[ignore = "stress"]
fn stress_the_front_end_matches_the_rowan_oracle_on_the_second_quarter_of_the_robustness_corpus() {
    assert!(checked(&robustness_quarter(1)) > 10_000);
}

#[test]
#[ignore = "stress"]
fn stress_the_front_end_matches_the_rowan_oracle_on_the_third_quarter_of_the_robustness_corpus() {
    assert!(checked(&robustness_quarter(2)) > 10_000);
}

#[test]
#[ignore = "stress"]
fn stress_the_front_end_matches_the_rowan_oracle_on_the_fourth_quarter_of_the_robustness_corpus() {
    assert!(checked(&robustness_quarter(3)) > 10_000);
}

#[test]
#[ignore = "stress"]
fn stress_the_front_end_matches_the_rowan_oracle_on_the_large_project_the_nests_and_the_generated_files()
 {
    assert_eq!(
        checked(&[large_corpus(), nest_corpus(), generated_corpus()]),
        62
    );
}

#[test]
#[ignore = "stress"]
fn stress_the_front_end_matches_the_rowan_oracle_on_the_deep_expressions_and_bodies() {
    assert_eq!(checked(&[deep_corpus(0), deep_corpus(1)]), 2);
}

#[test]
#[ignore = "stress"]
fn stress_the_front_end_matches_the_rowan_oracle_on_the_deep_array_type_and_calls() {
    assert_eq!(checked(&[deep_corpus(2), deep_corpus(3)]), 2);
}
