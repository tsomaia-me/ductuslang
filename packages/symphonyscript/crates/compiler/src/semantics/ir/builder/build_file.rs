use symphonyscript_core::Vocabulary;

use super::faults::Faults;
use super::hir_builder::HirBuilder;
use crate::semantics::ir::Hir;
use crate::syntax::Parsed;

pub(crate) fn build_file(hir: &mut Hir, path: &str, parsed: &Parsed, vocabulary: &Vocabulary) {
    let file = u32::try_from(hir.files.len()).unwrap_or(u32::MAX);
    hir.files.push(path.to_string());
    let mut builder = HirBuilder {
        hir,
        file,
        faults: Faults::new(&parsed.diagnostics),
        vocabulary,
    };
    let Some(root) = parsed.file() else {
        return;
    };
    for item in root.items() {
        builder.item(item);
    }
}
