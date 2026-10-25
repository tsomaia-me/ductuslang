use symphonyscript_core::Vocabulary;

use super::bodies::{skipped_block, skipped_names};
use super::faults::Faults;
use super::hir_builder::HirBuilder;
use crate::semantics::Internal;
use crate::semantics::ir::Hir;
use crate::syntax::Parsed;
use crate::syntax::ast::{AstNode, Item};

pub(crate) fn build_file(
    hir: &mut Hir,
    path: &str,
    parsed: &Parsed,
    vocabulary: &Vocabulary,
) -> Result<(), Internal> {
    let file = u32::try_from(hir.files.len()).unwrap_or(u32::MAX);
    hir.files.push(path.to_string());
    let mut builder = HirBuilder {
        hir,
        file,
        faults: Faults::new(&parsed.diagnostics, &parsed.detached),
        vocabulary,
        dropped: false,
    };
    let Some(root) = parsed.file() else {
        return Ok(());
    };
    let mut previous = None;
    for child in root.syntax().children() {
        if skipped_block(&child) {
            if let Some(placement) = previous.take() {
                builder.skip_block_of(placement, skipped_names(&child))?;
            }
        } else if let Some(item) = Item::cast(child) {
            previous = builder.item(item)?;
        } else {
            previous = None;
        }
    }
    Ok(())
}
