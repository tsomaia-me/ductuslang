use std::collections::{BTreeMap, BTreeSet};

use symphonyscript_core::{Limits, Vocabulary};

use super::program::program;
use crate::diagnostics::{Code, Diagnostic};
use crate::semantics::collection::Variants;
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
    diagnostics: Vec<Diagnostic>,
) -> Vec<Diagnostic> {
    let hir = analysis.hir;
    let mut fresh: BTreeMap<String, String> = BTreeMap::new();
    let mut counter = 0;
    let mut held: BTreeSet<Key> = BTreeSet::new();
    for id in placeholders {
        let Ok(expr) = hir.expr(*id) else {
            return diagnostics;
        };
        let ExprKind::Name(name) = &expr.kind else {
            return diagnostics;
        };
        let at = span(hir, expr.loc);
        held.insert((Code::UnknownName, at.file, at.range.start, at.range.end));
        if !fresh.contains_key(name) {
            let twin = unused_name(analysis, &mut counter, name);
            fresh.insert(name.clone(), twin);
        }
    }
    let path = twin_path(hir);
    let mut runs = Vec::new();
    for declaration in declarations(analysis.vocabulary, hir) {
        let Some(found) = twin_run(analysis, placeholders, &fresh, &path, &declaration) else {
            return diagnostics;
        };
        runs.push(found);
    }
    let Some((first, rest)) = runs.split_first() else {
        return diagnostics;
    };
    let mut common: BTreeSet<Key> = first.iter().map(key).collect();
    for run in rest {
        let keys: BTreeSet<Key> = run.iter().map(key).collect();
        common.retain(|found| keys.contains(found));
    }
    let original: BTreeSet<Key> = diagnostics.iter().map(key).collect();
    let mut kept: Vec<Diagnostic> = diagnostics
        .into_iter()
        .filter(|diagnostic| common.contains(&key(diagnostic)) || held.contains(&key(diagnostic)))
        .collect();
    let renamed: Vec<(String, String)> = fresh
        .iter()
        .map(|(name, twin)| (twin.clone(), name.clone()))
        .collect();
    kept.extend(
        first
            .iter()
            .filter(|diagnostic| {
                common.contains(&key(diagnostic)) && !original.contains(&key(diagnostic))
            })
            .map(|diagnostic| restored(diagnostic, &renamed, &path)),
    );
    kept.extend(
        runs.iter()
            .flatten()
            .filter(|diagnostic| diagnostic.code == Code::InternalError)
            .map(|diagnostic| restored(diagnostic, &renamed, &path)),
    );
    kept
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
) -> Option<Vec<Diagnostic>> {
    let mut twin = analysis.hir.clone();
    for id in placeholders {
        let expr = twin.exprs.get_mut(id.index())?;
        let ExprKind::Name(name) = &expr.kind else {
            return None;
        };
        expr.kind = ExprKind::Name(fresh.get(name)?.clone());
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
        return None;
    }
    build_file(&mut twin, path, &parsed, analysis.vocabulary).ok()?;
    let mut report = Report::default();
    program(
        &twin,
        analysis.vocabulary,
        analysis.variants,
        analysis.limits,
        &mut report,
        |_| (),
    );
    Some(
        report
            .finish(&twin.files)
            .into_iter()
            .filter(|diagnostic| diagnostic.span.file != path)
            .collect(),
    )
}

fn declarations(vocabulary: &Vocabulary, hir: &Hir) -> Vec<(&'static str, String)> {
    let mut found = vec![
        ("input", ": i32 = 0".to_string()),
        ("let", " = 1".to_string()),
        ("input", ": f64 = 0.5".to_string()),
        ("let", " = 0.5".to_string()),
        ("input", " = false".to_string()),
        ("let", " = false".to_string()),
        ("let", " = \"a\"".to_string()),
        ("input", ": i32[..4] = []".to_string()),
        ("let", " = [1]".to_string()),
    ];
    if let Some(variant) = vocabulary.enums().first().and_then(|decl| {
        decl.variants
            .first()
            .map(|variant| format!(" = {}::{variant}", decl.name))
    }) {
        found.push(("input", variant.clone()));
        found.push(("let", variant));
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

fn unused_name(analysis: &Analysis<'_>, counter: &mut usize, name: &str) -> String {
    loop {
        let candidate = format!("{MARK}{counter}{name}");
        *counter += 1;
        if !analysis
            .texts
            .iter()
            .any(|text| text.contains(candidate.as_str()))
        {
            return candidate;
        }
    }
}

fn twin_path(hir: &Hir) -> String {
    let mut path = format!("{}{MARK}", char::MAX);
    while hir.files.contains(&path) {
        path.push(MARK);
    }
    path
}

fn restored(diagnostic: &Diagnostic, renamed: &[(String, String)], path: &str) -> Diagnostic {
    let rename = |text: &str| {
        renamed.iter().fold(text.to_string(), |text, (twin, name)| {
            text.replace(twin, name)
        })
    };
    let mut restored = diagnostic.clone();
    restored.summary = rename(&restored.summary);
    restored.help = restored.help.as_deref().map(rename);
    restored.related.retain(|related| related.span.file != path);
    for related in &mut restored.related {
        related.label = rename(&related.label);
    }
    restored
}
