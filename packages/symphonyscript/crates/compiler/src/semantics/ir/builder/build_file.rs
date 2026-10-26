use symphonyscript_core::Vocabulary;

use super::bodies::{error_names, skipped_block, skipped_names};
use super::faults::Faults;
use super::hir_builder::HirBuilder;
use crate::semantics::Internal;
use crate::semantics::ir::{Hir, UnknownNames};
use crate::syntax::ast::{AstNode, Item};
use crate::syntax::{Parsed, SyntaxKind};

pub(crate) fn build_file(
    hir: &mut Hir,
    path: &str,
    parsed: &Parsed,
    vocabulary: &Vocabulary,
) -> Result<(), Internal> {
    let file = u32::try_from(hir.files.len()).unwrap_or(u32::MAX);
    hir.files.push(path.to_string());
    let Some(root) = parsed.file() else {
        return Ok(());
    };
    let mut builder = HirBuilder {
        hir,
        file,
        faults: Faults::new(&parsed.diagnostics, &parsed.detached, root.syntax()),
        vocabulary,
        dropped: false,
    };
    let mut previous = None;
    let mut skipped = Vec::new();
    for child in root.syntax().children() {
        if child.kind() == SyntaxKind::ErrorNode {
            match previous.take() {
                Some(placement) if skipped_block(&child) => {
                    builder.skip_block_of(placement, skipped_names(&child))?;
                }
                _ => skipped.extend(skipped_names(&child)),
            }
        } else if let Some(item) = Item::cast(child.clone()) {
            skipped.extend(error_names(&child));
            previous = builder.item(item)?;
        } else {
            previous = None;
        }
    }
    builder.hir.skipped_top.join(&UnknownNames::of(skipped));
    Ok(())
}
