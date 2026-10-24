#![cfg_attr(
    not(test),
    deny(
        clippy::indexing_slicing,
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic
    )
)]

use symphonyscript_core::Vocabulary;

use crate::diagnostics::{Diagnostic, Severity};
use crate::failed::Failed;
use crate::semantics::analysis::Engine;
use crate::semantics::analyze;
use crate::semantics::collection::Variants;
use crate::semantics::ir::{Hir, build_file};
use crate::semantics::reporting::Report;
use crate::source::Source;
use crate::syntax::parse;

pub struct Compiler {
    vocabulary: Vocabulary,
    variants: Variants,
    prelude: Hir,
}

impl Compiler {
    pub fn new(vocab: &Vocabulary) -> Result<Compiler, Failed> {
        let mut hir = Hir::default();
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
            if let Err(internal) = build_file(&mut hir, &file.path, &parsed, vocab) {
                analyze::internal_error(&hir, &mut report, internal);
                return Err(Failed {
                    diagnostics: report.finish(&hir.files),
                });
            }
        }
        hir.seal_prelude();
        let variants = Variants::of(vocab);
        analyze::prelude(&hir, vocab, &variants, &mut report);
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
        })
    }

    pub fn check(&self, sources: &[Source]) -> Vec<Diagnostic> {
        self.analyzed(sources, |_| ()).0
    }

    pub(crate) fn analyzed<R>(
        &self,
        sources: &[Source],
        inspect: impl FnOnce(&Engine<'_>) -> R,
    ) -> (Vec<Diagnostic>, Option<R>) {
        let mut hir = self.prelude.clone();
        let mut report = Report::default();
        let mut ordered: Vec<&Source> = sources.iter().collect();
        ordered.sort_by(|a, b| a.path.as_bytes().cmp(b.path.as_bytes()));
        for source in ordered {
            let parsed = parse(source, &self.vocabulary);
            report
                .diagnostics
                .extend(parsed.diagnostics.iter().cloned());
            if let Err(internal) = build_file(&mut hir, &source.path, &parsed, &self.vocabulary) {
                analyze::internal_error(&hir, &mut report, internal);
                return (report.finish(&hir.files), None);
            }
        }
        let inspected =
            analyze::program(&hir, &self.vocabulary, &self.variants, &mut report, inspect);
        (report.finish(&hir.files), inspected)
    }
}
