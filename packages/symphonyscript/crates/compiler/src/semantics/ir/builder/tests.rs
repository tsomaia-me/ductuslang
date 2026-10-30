use symphonyscript_fixtures::vocabulary;

use super::faults::Faults;
use super::hir_builder::HirBuilder;
use crate::semantics::Internal;
use crate::semantics::ir::{BlockId, Hir};
use crate::semantics::reporting::Report;
use crate::source::Source;
use crate::syntax::ast::Item;
use crate::syntax::parse;

#[test]
fn a_write_into_a_block_the_builder_never_allocated_is_an_internal_error() {
    let vocabulary = vocabulary();
    let source = Source {
        path: "test.sym".into(),
        text: "Clip c:\n    Note\n".into(),
    };
    let parsed = parse(&source, &vocabulary);
    let Some(Item::Placement(placement)) = parsed.file().and_then(|file| file.items().next())
    else {
        panic!("the source holds one placement");
    };
    let Some(block) = placement.body().and_then(|body| body.block()) else {
        panic!("the placement has a block");
    };
    let mut hir = Hir::default();
    let mut report = Report::default();
    let mut builder = HirBuilder {
        hir: &mut hir,
        file: 0,
        faults: Faults::new(&parsed.diagnostics, &parsed.detached, &parsed.root()),
        vocabulary: &vocabulary,
        dropped: false,
        report: &mut report,
    };
    assert_eq!(
        builder.fill(vec![(block.clone(), BlockId(0))]),
        Err(Internal::missed("block"))
    );
    let allocated = builder.empty_block();
    assert_eq!(builder.fill(vec![(block, allocated)]), Ok(()));
    assert_eq!(hir.blocks.len(), 1);
    assert_eq!(hir.blocks.first().map(|block| block.items.len()), Some(1));
}
