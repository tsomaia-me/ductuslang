use symphonyscript_core::Vocabulary;

use crate::diagnostics::{Diagnostic, Severity};
use crate::failed::Failed;
use crate::semantics::analyze;
use crate::semantics::ir::{Hir, build_file};
use crate::semantics::reporting::Report;
use crate::source::Source;
use crate::syntax::parse;

pub struct Compiler {
    vocabulary: Vocabulary,
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
            build_file(&mut hir, &file.path, &parsed, vocab);
        }
        hir.seal_prelude();
        analyze::prelude(&hir, vocab, &mut report);
        let diagnostics = report.finish();
        if diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity() == Severity::Error)
        {
            return Err(Failed { diagnostics });
        }
        Ok(Compiler {
            vocabulary: vocab.clone(),
            prelude: hir,
        })
    }

    pub fn check(&self, sources: &[Source]) -> Vec<Diagnostic> {
        let mut hir = self.prelude.clone();
        let mut report = Report::default();
        let mut ordered: Vec<&Source> = sources.iter().collect();
        ordered.sort_by(|a, b| a.path.as_bytes().cmp(b.path.as_bytes()));
        for source in ordered {
            let parsed = parse(source, &self.vocabulary);
            report
                .diagnostics
                .extend(parsed.diagnostics.iter().cloned());
            build_file(&mut hir, &source.path, &parsed, &self.vocabulary);
        }
        analyze::program(&hir, &self.vocabulary, &mut report);
        report.finish()
    }
}
