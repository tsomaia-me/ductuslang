use crate::diagnostics::Code;
use crate::semantics::Internal;
use crate::semantics::ir::{Hir, Loc};
use crate::semantics::reporting::Report;

pub(crate) fn internal_error(hir: &Hir, report: &mut Report, internal: Internal) {
    let first_source = if hir.files.len() > hir.prelude.files {
        hir.prelude.files
    } else {
        0
    };
    let loc = Loc {
        file: u32::try_from(first_source).unwrap_or(0),
        start: 0,
        end: 0,
    };
    report.error(
        hir,
        Code::InternalError,
        loc,
        format!(
            "internal compiler error: a lookup of a {} missed; this is a defect of the compiler",
            internal.missed
        ),
    );
}
