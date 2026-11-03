#![cfg_attr(
    not(test),
    deny(
        clippy::indexing_slicing,
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic
    )
)]

use symphonyscript_core::{LimitOverrides, Limits, Span, Vocabulary};

use crate::diagnostics::{Code, Diagnostic, Severity};
use crate::failed::Failed;
use crate::selector::Selector;
use crate::semantics::Internal;
use crate::semantics::analysis::Engine;
use crate::semantics::analyze;
use crate::semantics::collection::Variants;
use crate::semantics::ir::{ExprId, FlatIr, Hir, adapt_file_mapped, build_flat_file};
use crate::semantics::pipeline;
use crate::semantics::reporting::Report;
use crate::source::Source;
use crate::syntax::parse;
use std::sync::atomic::{AtomicU64, Ordering};

#[cfg(test)]
thread_local! {
    pub(crate) static BEFORE_TWINS: std::cell::Cell<fn()> = const { std::cell::Cell::new(|| ()) };
}

pub struct Compiler {
    vocabulary: Vocabulary,
    variants: Variants,
    prelude: Hir,
    prelude_ir: FlatIr,
    limits: Limits,
    selector: Selector,
    taken: AtomicU64,
}

impl Compiler {
    pub fn new(vocab: &Vocabulary) -> Result<Compiler, Failed> {
        Self::with_limits(vocab, LimitOverrides::default())
    }

    pub fn with_limits(vocab: &Vocabulary, overrides: LimitOverrides) -> Result<Compiler, Failed> {
        let limits = overrides.applied_to(Limits::default());
        let mut hir = Hir::default();
        let mut ir = FlatIr::default();
        let mut report = Report::default();
        let mut files: Vec<Source> = vocab
            .prelude()
            .iter()
            .map(|file| Source {
                path: file.path.clone(),
                text: file.text.clone(),
            })
            .collect();
        files.sort_by(|a, b| a.path.as_bytes().cmp(b.path.as_bytes()));
        for file in &files {
            let parsed = parse(file, vocab);
            report
                .diagnostics
                .extend(parsed.diagnostics.iter().cloned());
            if let Err(internal) = lowered(&mut ir, &mut hir, &file.path, &parsed, vocab) {
                analyze::internal_error(&hir, &mut report, internal);
                return Err(Failed {
                    diagnostics: report.finish(&hir.files),
                });
            }
        }
        hir.seal_prelude();
        ir.freeze();
        let variants = match Variants::of(vocab) {
            Ok(variants) => variants,
            Err(internal) => {
                analyze::internal_error(&hir, &mut report, internal);
                return Err(Failed {
                    diagnostics: report.finish(&hir.files),
                });
            }
        };
        analyze::prelude(&hir, vocab, &variants, limits, &mut report);
        let diagnostics = report.finish(&hir.files);
        if diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity() == Severity::Error)
        {
            return Err(Failed { diagnostics });
        }
        Ok(Compiler {
            vocabulary: vocab.clone(),
            variants,
            prelude: hir,
            prelude_ir: ir,
            limits,
            selector: Selector::Default,
            taken: AtomicU64::new(0),
        })
    }

    pub fn select(&mut self, selector: Selector) {
        self.selector = selector;
    }

    #[cfg(test)]
    pub(crate) fn on_the_old_engine(mut self) -> Compiler {
        self.selector = Selector::OldEngine;
        self
    }

    pub fn taken_by_new_core(&self) -> u64 {
        self.taken.load(Ordering::Relaxed)
    }

    pub fn check(&self, sources: &[Source]) -> Vec<Diagnostic> {
        self.analyzed_by(sources, |_| (), analyze::name_twins, Route::Selected)
            .unwrap_or_default()
            .0
    }

    #[cfg(test)]
    pub(crate) fn analyzed<R>(
        &self,
        sources: &[Source],
        inspect: impl FnOnce(&Engine<'_>) -> R,
    ) -> (Vec<Diagnostic>, Option<R>) {
        self.analyzed_by(sources, inspect, analyze::name_twins, Route::Selected)
            .unwrap_or_default()
    }

    #[cfg(test)]
    pub(crate) fn checked_by_whole_program(&self, sources: &[Source]) -> Vec<Diagnostic> {
        self.analyzed_by(
            sources,
            |_| (),
            analyze::whole_program_twins,
            Route::Selected,
        )
        .unwrap_or_default()
        .0
    }

    #[cfg(test)]
    pub(crate) fn checked_by_new_core(&self, sources: &[Source]) -> Option<Vec<Diagnostic>> {
        self.analyzed_by(sources, |_| (), analyze::name_twins, Route::NewCoreOnly)
            .map(|(diagnostics, _)| diagnostics)
    }

    fn analyzed_by<R>(
        &self,
        sources: &[Source],
        inspect: impl FnOnce(&Engine<'_>) -> R,
        twins: impl FnOnce(
            &analyze::Analysis<'_>,
            &[ExprId],
            &[Diagnostic],
        ) -> Result<Vec<Diagnostic>, Internal>,
        route: Route,
    ) -> Option<(Vec<Diagnostic>, Option<R>)> {
        let mut hir = self.prelude.clone();
        let mut ir = FlatIr::after(&self.prelude_ir);
        let mut report = Report::default();
        let mut exprs_of: Vec<(u32, ExprId)> = Vec::new();
        let mut ordered: Vec<&Source> = sources.iter().collect();
        ordered.sort_by(|a, b| a.path.as_bytes().cmp(b.path.as_bytes()));
        for source in ordered {
            if self
                .vocabulary
                .prelude()
                .iter()
                .any(|file| file.path == source.path)
            {
                report.diagnostics.push(Diagnostic::new(
                    Code::PreludePath,
                    Span {
                        file: source.path.clone(),
                        range: 0..0,
                    },
                    format!(
                        "the source file `{}` stands at the path of a prelude file, so it is not read",
                        source.path
                    ),
                ));
                continue;
            }
            let parsed = parse(source, &self.vocabulary);
            report
                .diagnostics
                .extend(parsed.diagnostics.iter().cloned());
            match lowered(&mut ir, &mut hir, &source.path, &parsed, &self.vocabulary) {
                Ok(mapped) => exprs_of.extend(mapped),
                Err(internal) => {
                    analyze::internal_error(&hir, &mut report, internal);
                    return Some((report.finish(&hir.files), None));
                }
            }
        }
        let front = report.diagnostics.len();
        let taken = if route == Route::Selected && self.selector == Selector::OldEngine {
            None
        } else {
            match pipeline::analyze(&pipeline::Inputs {
                prelude: &self.prelude_ir,
                ir: &ir,
                vocabulary: &self.vocabulary,
                variants: &self.variants,
                limits: self.limits,
                front_end_faults: front > 0,
            }) {
                Ok(run) => run,
                Err(internal) => {
                    analyze::internal_error(&hir, &mut report, internal);
                    return Some((report.finish(&hir.files), None));
                }
            }
        };
        let (inspected, placeholders) = match taken {
            Some(run) => {
                self.taken.fetch_add(1, Ordering::Relaxed);
                report.diagnostics.extend(run.diagnostics);
                let mapped: std::collections::HashMap<u32, ExprId> =
                    exprs_of.iter().copied().collect();
                let placeholders = run
                    .placeholders
                    .iter()
                    .filter_map(|node| mapped.get(node).copied())
                    .collect::<Vec<ExprId>>();
                (None, placeholders)
            }
            None if route == Route::NewCoreOnly => return None,
            None => analyze::program(
                &hir,
                &self.vocabulary,
                &self.variants,
                self.limits,
                &mut report,
                inspect,
            ),
        };
        if !placeholders.is_empty() {
            let analysis = analyze::Analysis {
                hir: &hir,
                vocabulary: &self.vocabulary,
                variants: &self.variants,
                limits: self.limits,
                texts: sources
                    .iter()
                    .map(|source| source.text.as_str())
                    .chain(
                        self.vocabulary
                            .prelude()
                            .iter()
                            .map(|file| file.text.as_str()),
                    )
                    .collect(),
            };
            let found = report.diagnostics.split_off(front);
            #[cfg(test)]
            BEFORE_TWINS.with(|hook| hook.get()());
            match twins(&analysis, &placeholders, &found) {
                Ok(kept) => report.diagnostics.extend(kept),
                Err(internal) => {
                    report.diagnostics.extend(found);
                    analyze::internal_error(&hir, &mut report, internal);
                }
            }
        }
        Some((report.finish(&hir.files), inspected))
    }
}

fn lowered(
    ir: &mut FlatIr,
    hir: &mut Hir,
    path: &str,
    parsed: &crate::syntax::Parsed,
    vocabulary: &Vocabulary,
) -> Result<Vec<(u32, ExprId)>, Internal> {
    let file = ir.files.len();
    build_flat_file(ir, path, parsed, vocabulary)?;
    adapt_file_mapped(hir, ir, file)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Route {
    Selected,
    NewCoreOnly,
}
