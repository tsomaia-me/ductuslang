use std::collections::{BTreeMap, BTreeSet};

use symphonyscript_core::{Limits, Vocabulary};

use super::program::program;
use crate::diagnostics::{Code, Diagnostic};
use crate::semantics::Internal;
use crate::semantics::collection::Variants;
use crate::semantics::counted_walk::CountedWalk;
use crate::semantics::ir::{ExprId, ExprKind, Hir, Top, build_file};
use crate::semantics::reporting::{Report, span};
use crate::source::Source;
use crate::syntax::parse;

const MARK: char = 'ǂ';

pub(crate) struct Analysis<'a> {
    pub hir: &'a Hir,
    pub vocabulary: &'a Vocabulary,
    pub variants: &'a Variants,
    pub limits: Limits,
    pub texts: Vec<&'a str>,
}

pub(crate) fn name_twins(
    analysis: &Analysis<'_>,
    placeholders: &[ExprId],
    diagnostics: &[Diagnostic],
) -> Result<Vec<Diagnostic>, Internal> {
    let hir = analysis.hir;
    let taken = marked_counters(&analysis.texts);
    let mut originals: BTreeMap<String, String> = BTreeMap::new();
    let mut fresh: BTreeMap<String, String> = BTreeMap::new();
    let mut counter = 0;
    let mut held: BTreeSet<Key> = BTreeSet::new();
    for id in placeholders {
        let expr = hir.expr(*id)?;
        let ExprKind::Name(name) = &expr.kind else {
            return Err(Internal::missed("name placeholder"));
        };
        let at = span(hir, expr.loc);
        held.insert((Code::UnknownName, at.file, at.range.start, at.range.end));
        if !fresh.contains_key(name) {
            let marked = unused_counter(&taken, &mut counter);
            fresh.insert(name.clone(), format!("{MARK}{marked}{name}"));
            originals.insert(marked, name.clone());
        }
    }
    let path = twin_path(hir);
    let mut runs = Vec::new();
    for declaration in declarations(analysis.vocabulary, hir) {
        let Some(found) = twin_run(analysis, placeholders, &fresh, &path, &declaration)? else {
            return Ok(diagnostics.to_vec());
        };
        runs.push(found);
    }
    let Some((first, rest)) = runs.split_first() else {
        return Ok(diagnostics.to_vec());
    };
    let mut common: BTreeSet<Key> = first.iter().map(key).collect();
    for run in rest {
        let keys: BTreeSet<Key> = run.iter().map(key).collect();
        common.retain(|found| keys.contains(found));
    }
    let original: BTreeSet<Key> = diagnostics.iter().map(key).collect();
    let mut kept: Vec<Diagnostic> = diagnostics
        .iter()
        .filter(|diagnostic| common.contains(&key(diagnostic)) || held.contains(&key(diagnostic)))
        .cloned()
        .collect();
    kept.extend(
        first
            .iter()
            .filter(|diagnostic| {
                common.contains(&key(diagnostic)) && !original.contains(&key(diagnostic))
            })
            .map(|diagnostic| restored(diagnostic, &originals, &path)),
    );
    kept.extend(
        runs.iter()
            .flatten()
            .filter(|diagnostic| diagnostic.code == Code::InternalError)
            .map(|diagnostic| restored(diagnostic, &originals, &path)),
    );
    Ok(kept)
}

type Key = (Code, String, u32, u32);

fn key(diagnostic: &Diagnostic) -> Key {
    (
        diagnostic.code,
        diagnostic.span.file.clone(),
        diagnostic.span.range.start,
        diagnostic.span.range.end,
    )
}

fn twin_run(
    analysis: &Analysis<'_>,
    placeholders: &[ExprId],
    fresh: &BTreeMap<String, String>,
    path: &str,
    (keyword, rest): &(&str, String),
) -> Result<Option<Vec<Diagnostic>>, Internal> {
    let mut twin = analysis.hir.clone();
    for id in placeholders {
        let expr = twin
            .exprs
            .get_mut(id.index())
            .ok_or(Internal::missed("name placeholder"))?;
        let ExprKind::Name(name) = &expr.kind else {
            return Err(Internal::missed("name placeholder"));
        };
        let twin_name = fresh.get(name).ok_or(Internal::missed("twin name"))?;
        expr.kind = ExprKind::Name(twin_name.clone());
    }
    let text: String = fresh
        .values()
        .map(|name| format!("{keyword} {name}{rest}\n"))
        .collect();
    let source = Source {
        path: path.to_string(),
        text,
    };
    let parsed = parse(&source, analysis.vocabulary);
    if !parsed.diagnostics.is_empty() {
        return Ok(None);
    }
    build_file(&mut twin, path, &parsed, analysis.vocabulary)?;
    let mut report = Report::default();
    program(
        &twin,
        analysis.vocabulary,
        analysis.variants,
        analysis.limits,
        &mut report,
        |_| (),
    );
    Ok(Some(
        report
            .finish(&twin.files)
            .into_iter()
            .filter(|diagnostic| diagnostic.span.file != path)
            .collect(),
    ))
}

fn declarations(vocabulary: &Vocabulary, hir: &Hir) -> Vec<(&'static str, String)> {
    let mut found = vec![
        ("state", "(0): i32".to_string()),
        ("let", " = 1".to_string()),
        ("state", "(0.5): f64".to_string()),
        ("let", " = 0.5".to_string()),
        ("state", "(false)".to_string()),
        ("let", " = false".to_string()),
        ("let", " = \"a\"".to_string()),
        ("state", "([]): i32[..4]".to_string()),
        ("let", " = [1]".to_string()),
    ];
    if let Some(variant) = vocabulary.enums().first().and_then(|decl| {
        decl.variants
            .first()
            .map(|variant| format!("{}::{variant}", decl.name))
    }) {
        found.push(("state", format!("({variant})")));
        found.push(("let", format!(" = {variant}")));
    }
    let mut kinds = BTreeSet::new();
    for (position, top) in hir.top.iter().enumerate() {
        let Top::Placement(id) = top else {
            continue;
        };
        let Ok(placement) = hir.placement(*id) else {
            continue;
        };
        if hir.is_prelude_top(position) {
            continue;
        }
        if let (Some(kind), Some(name)) = (&placement.kind, &placement.name)
            && kinds.insert(kind.text.clone())
        {
            found.push(("let", format!(" = {}", name.text)));
        }
    }
    found
}

fn marked_counters(texts: &[&str]) -> BTreeSet<String> {
    let mut taken = BTreeSet::new();
    for text in texts {
        for (at, _) in text.match_indices(MARK) {
            CountedWalk::NameTwinsUnusedName.step();
            let rest = text.get(at + MARK.len_utf8()..).unwrap_or_default();
            let digits = rest.len() - rest.trim_start_matches(|c: char| c.is_ascii_digit()).len();
            taken.insert(rest.get(..digits).unwrap_or_default().to_string());
        }
    }
    taken
}

fn unused_counter(taken: &BTreeSet<String>, counter: &mut usize) -> String {
    loop {
        CountedWalk::NameTwinsUnusedName.step();
        let candidate = counter.to_string();
        *counter += 1;
        if !taken.contains(&candidate) {
            return candidate;
        }
    }
}

fn renamed(text: &str, originals: &BTreeMap<String, String>) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find(MARK) {
        CountedWalk::NameTwinsRestored.step();
        out.push_str(rest.get(..at).unwrap_or_default());
        let after = rest.get(at + MARK.len_utf8()..).unwrap_or_default();
        let digits = after.len() - after.trim_start_matches(|c: char| c.is_ascii_digit()).len();
        let counter = after.get(..digits).unwrap_or_default();
        let named = after.get(digits..).unwrap_or_default();
        match originals.get(counter) {
            Some(name) if named.starts_with(name.as_str()) => {
                out.push_str(name);
                rest = named.get(name.len()..).unwrap_or_default();
            }
            Some(_) | None => {
                out.push(MARK);
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

fn twin_path(hir: &Hir) -> String {
    let mut path = format!("{}{MARK}", char::MAX);
    while hir.files.contains(&path) {
        path.push(MARK);
    }
    path
}

fn restored(
    diagnostic: &Diagnostic,
    originals: &BTreeMap<String, String>,
    path: &str,
) -> Diagnostic {
    let mut restored = diagnostic.clone();
    restored.summary = renamed(&restored.summary, originals);
    restored.help = restored
        .help
        .as_deref()
        .map(|help| renamed(help, originals));
    restored.related.retain(|related| related.span.file != path);
    for related in &mut restored.related {
        related.label = renamed(&related.label, originals);
    }
    restored
}
