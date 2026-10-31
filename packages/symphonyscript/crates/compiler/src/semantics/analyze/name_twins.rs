use std::collections::{BTreeMap, BTreeSet};

use symphonyscript_core::{Limits, Vocabulary};

use super::program::program;
use super::twin_program::TwinProgram;
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

struct Twins {
    fresh: BTreeMap<String, String>,
    originals: BTreeMap<String, String>,
    held: BTreeSet<Key>,
    path: String,
}

pub(crate) fn name_twins(
    analysis: &Analysis<'_>,
    placeholders: &[ExprId],
    diagnostics: &[Diagnostic],
) -> Result<Vec<Diagnostic>, Internal> {
    let twins = Twins::of(analysis, placeholders)?;
    let (declarations, referenced) = declarations(analysis.vocabulary, analysis.hir);
    let Some(twin) = TwinProgram::of(analysis.hir, placeholders, &referenced)? else {
        return whole_program(analysis, placeholders, diagnostics);
    };
    let mut report = Report::default();
    program(
        &twin.hir,
        analysis.vocabulary,
        analysis.variants,
        analysis.limits,
        &mut report,
        |_| (),
    );
    let null = report.finish(&twin.hir.files);
    twins.kept(analysis, &twin, (diagnostics, &null), &declarations)
}

#[cfg(test)]
pub(crate) fn whole_program_twins(
    analysis: &Analysis<'_>,
    placeholders: &[ExprId],
    diagnostics: &[Diagnostic],
) -> Result<Vec<Diagnostic>, Internal> {
    whole_program(analysis, placeholders, diagnostics)
}

fn whole_program(
    analysis: &Analysis<'_>,
    placeholders: &[ExprId],
    diagnostics: &[Diagnostic],
) -> Result<Vec<Diagnostic>, Internal> {
    let twins = Twins::of(analysis, placeholders)?;
    let (declarations, _) = declarations(analysis.vocabulary, analysis.hir);
    let whole = TwinProgram {
        hir: analysis.hir.clone(),
        placeholders: placeholders.to_vec(),
    };
    twins.kept(analysis, &whole, (diagnostics, diagnostics), &declarations)
}

impl Twins {
    fn of(analysis: &Analysis<'_>, placeholders: &[ExprId]) -> Result<Twins, Internal> {
        let hir = analysis.hir;
        let taken = marked_counters(&analysis.texts);
        let mut twins = Twins {
            fresh: BTreeMap::new(),
            originals: BTreeMap::new(),
            held: BTreeSet::new(),
            path: twin_path(hir),
        };
        let mut counter = 0;
        for id in placeholders {
            let expr = hir.expr(*id)?;
            let ExprKind::Name(name) = &expr.kind else {
                return Err(Internal::missed("name placeholder"));
            };
            let at = span(hir, expr.loc);
            twins
                .held
                .insert((Code::UnknownName, at.file, at.range.start, at.range.end));
            if !twins.fresh.contains_key(name) {
                let marked = unused_counter(&taken, &mut counter);
                twins
                    .fresh
                    .insert(name.clone(), format!("{MARK}{marked}{name}"));
                twins.originals.insert(marked, name.clone());
            }
        }
        Ok(twins)
    }

    fn kept(
        &self,
        analysis: &Analysis<'_>,
        twin: &TwinProgram,
        (diagnostics, null): (&[Diagnostic], &[Diagnostic]),
        declarations: &[(&str, String)],
    ) -> Result<Vec<Diagnostic>, Internal> {
        let mut sources = Vec::new();
        for (keyword, rest) in declarations {
            let source = Source {
                path: self.path.clone(),
                text: self
                    .fresh
                    .values()
                    .map(|name| format!("{keyword} {name}{rest}\n"))
                    .collect(),
            };
            let parsed = parse(&source, analysis.vocabulary);
            if !parsed.diagnostics.is_empty() {
                return Ok(diagnostics.to_vec());
            }
            sources.push(parsed);
        }
        let original: BTreeSet<Key> = diagnostics.iter().map(key).collect();
        let outside: BTreeSet<Key> = null.iter().map(key).collect();
        let mut runs = Vec::new();
        for parsed in &sources {
            runs.push(self.twin_run(analysis, twin, parsed)?);
        }
        let Some((first, rest)) = runs.split_first() else {
            return Ok(diagnostics.to_vec());
        };
        let mut common: BTreeSet<Key> = first.iter().map(key).collect();
        for run in rest {
            let keys: BTreeSet<Key> = run.iter().map(key).collect();
            common.retain(|found| keys.contains(found));
        }
        let mut kept: Vec<Diagnostic> = diagnostics
            .iter()
            .filter(|diagnostic| {
                let found = key(diagnostic);
                common.contains(&found) || self.held.contains(&found) || !outside.contains(&found)
            })
            .cloned()
            .collect();
        kept.extend(
            first
                .iter()
                .filter(|diagnostic| {
                    let found = key(diagnostic);
                    common.contains(&found)
                        && !original.contains(&found)
                        && !outside.contains(&found)
                })
                .map(|diagnostic| restored(diagnostic, &self.originals, &self.path)),
        );
        kept.extend(
            runs.iter()
                .flatten()
                .filter(|diagnostic| diagnostic.code == Code::InternalError)
                .map(|diagnostic| restored(diagnostic, &self.originals, &self.path)),
        );
        Ok(kept)
    }

    fn twin_run(
        &self,
        analysis: &Analysis<'_>,
        program_to_twin: &TwinProgram,
        parsed: &crate::syntax::Parsed,
    ) -> Result<Vec<Diagnostic>, Internal> {
        let mut twin = program_to_twin.hir.clone();
        for id in &program_to_twin.placeholders {
            let expr = twin
                .exprs
                .get_mut(id.index())
                .ok_or(Internal::missed("name placeholder"))?;
            let ExprKind::Name(name) = &expr.kind else {
                return Err(Internal::missed("name placeholder"));
            };
            let twin_name = self.fresh.get(name).ok_or(Internal::missed("twin name"))?;
            expr.kind = ExprKind::Name(twin_name.clone());
        }
        build_file(&mut twin, &self.path, parsed, analysis.vocabulary)?;
        let mut report = Report::default();
        program(
            &twin,
            analysis.vocabulary,
            analysis.variants,
            analysis.limits,
            &mut report,
            |_| (),
        );
        Ok(report
            .finish(&twin.files)
            .into_iter()
            .filter(|diagnostic| diagnostic.span.file != self.path)
            .collect())
    }
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

type Declarations = (Vec<(&'static str, String)>, Vec<String>);

fn declarations(vocabulary: &Vocabulary, hir: &Hir) -> Declarations {
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
    let mut referenced = Vec::new();
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
            referenced.push(name.text.clone());
        }
    }
    (found, referenced)
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
