use std::collections::BTreeSet;
use std::ops::Range;

use symphonyscript_fixtures::vocabulary;

use super::super::Parser;
use super::super::cost::Cost;
use super::fault_catalogue::fault_items;
use super::generated_file::{Generated, Pool, generate};
use super::valid_items::valid_items;
use crate::diagnostics::{Code, Diagnostic};
use crate::source::Source;

const LEXER_AND_PARSER_CODES: &[Code] = &[
    Code::UnexpectedCharacter,
    Code::TabIndentation,
    Code::DedentMismatch,
    Code::ClauseColumn,
    Code::Semicolon,
    Code::InvalidIdentifier,
    Code::ReservedWord,
    Code::ReservedToken,
    Code::MisplacedCaret,
    Code::MisplacedUnderscore,
    Code::FloatDigits,
    Code::FloatTypeSuffix,
    Code::UnknownSuffix,
    Code::PrefixedSuffix,
    Code::IntegerTooLarge,
    Code::UnterminatedString,
    Code::InvalidEscape,
    Code::TopLevelSyntax,
    Code::UnexpectedIndent,
    Code::TypeSyntax,
    Code::ExpressionSyntax,
    Code::ChainedComparison,
    Code::UnparenthesizedIf,
    Code::UnparenthesizedFor,
    Code::MatchSyntax,
    Code::MatchPosition,
    Code::MixedArguments,
    Code::ConversionSyntax,
    Code::InputSyntax,
    Code::LetSyntax,
    Code::StateSyntax,
    Code::FnSyntax,
    Code::PatternSyntax,
    Code::PlacementSyntax,
    Code::TopLevelWire,
    Code::FlagSyntax,
    Code::DefaultArgSyntax,
    Code::AttributeSyntax,
    Code::InlinePlacement,
    Code::BundleSyntax,
    Code::ForSyntax,
    Code::WireSyntax,
    Code::MisplacedWire,
    Code::WhenSyntax,
];

const SEEDS: u64 = 24;

fn parse_counting(text: &str) -> (Vec<Diagnostic>, Cost) {
    let source = Source {
        path: "generated.sym".into(),
        text: text.into(),
    };
    let vocabulary = vocabulary();
    let (_, diagnostics, cost) = Parser::new(&source, &vocabulary).run_counting();
    (diagnostics, cost)
}

fn sorted(mut diagnostics: Vec<(Code, Range<u32>)>) -> Vec<(u32, u32, &'static str)> {
    diagnostics.sort_by_key(|(code, range)| (range.start, range.end, code.as_str()));
    diagnostics
        .into_iter()
        .map(|(code, range)| (range.start, range.end, code.as_str()))
        .collect()
}

fn assert_reported_once(generated: &Generated, seed: u64) -> Cost {
    let (diagnostics, cost) = parse_counting(&generated.text);
    for (span, named) in &generated.closers {
        let message = diagnostics
            .iter()
            .find(|diagnostic| diagnostic.span.range == *span)
            .map(|diagnostic| diagnostic.summary.as_str());
        assert!(
            message.is_some_and(|message| message.ends_with(named.as_str())),
            "seed {seed}: the missing closer at {span:?} says {message:?}, not {named:?}"
        );
    }
    let found = sorted(
        diagnostics
            .into_iter()
            .map(|diagnostic| (diagnostic.code, diagnostic.span.range))
            .collect(),
    );
    let expected = sorted(generated.expected.clone());
    if found != expected {
        let first = found
            .iter()
            .zip(&expected)
            .position(|(found, expected)| found != expected)
            .unwrap_or(found.len().min(expected.len()));
        let at = found
            .get(first)
            .into_iter()
            .chain(expected.get(first))
            .map(|(start, _, _)| *start)
            .min()
            .unwrap_or(0);
        let item = generated
            .items
            .partition_point(|(start, _)| *start <= at)
            .saturating_sub(1);
        let around: Vec<String> = generated
            .items
            .iter()
            .enumerate()
            .skip(item.saturating_sub(1))
            .take(3)
            .map(|(index, (start, label))| {
                let end = generated
                    .items
                    .get(index + 1)
                    .map_or(generated.text.len(), |(end, _)| *end as usize);
                format!(
                    "{start} {label}:\n{}",
                    &generated.text[*start as usize..end]
                )
            })
            .collect();
        panic!(
            "seed {seed}: found {:?}, expected {:?}, in\n{}",
            found.get(first..(first + 3).min(found.len())),
            expected.get(first..(first + 3).min(expected.len())),
            around.join("")
        );
    }
    cost
}

#[test]
fn faults_in_generated_files_are_each_reported_once_at_their_span() {
    let vocabulary = vocabulary();
    let faults = fault_items();
    let covered: BTreeSet<&str> = faults.iter().map(|item| item.code.as_str()).collect();
    let all: BTreeSet<&str> = LEXER_AND_PARSER_CODES
        .iter()
        .map(|code| code.as_str())
        .collect();
    assert_eq!(covered, all);
    let pool = Pool::new(&faults, &valid_items(&vocabulary), &vocabulary);
    for seed in 1..=SEEDS {
        assert_reported_once(&generate(seed, faults.len(), &pool), seed);
    }
}

#[test]
fn recovery_cost_grows_linearly_with_the_number_of_fault_items() {
    let vocabulary = vocabulary();
    let faults = fault_items();
    let pool = Pool::new(&faults, &valid_items(&vocabulary), &vocabulary);
    let items = 2_000;
    let half = assert_reported_once(&generate(7, items, &pool), 7);
    let full = assert_reported_once(&generate(7, items * 2, &pool), 7);
    assert!(
        full.work * 10 <= half.work * 22,
        "{} work for {items} fault items, {} for {}",
        half.work,
        full.work,
        items * 2
    );
    assert!(
        full.peak_stored_entries * 10 <= half.peak_stored_entries * 22,
        "{} stored stack entries for {items} fault items, {} for {}",
        half.peak_stored_entries,
        full.peak_stored_entries,
        items * 2
    );
}
