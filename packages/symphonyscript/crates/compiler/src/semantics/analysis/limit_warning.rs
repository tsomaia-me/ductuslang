use super::engine::Engine;
use super::ids::ScopeId;
use super::limit::Limit;
use crate::diagnostics::Code;
use crate::semantics::Internal;
use crate::semantics::ir::Loc;
use crate::semantics::reporting::grouped;

impl Engine<'_> {
    pub(in crate::semantics::analysis) fn warn_limit(
        &mut self,
        limit: Limit,
        (loc, scope): (Loc, ScopeId),
        (counted, bound): (String, u64),
    ) -> Result<(), Internal> {
        if self.stop.is_some() {
            return Ok(());
        }
        let related = self
            .chain(scope)?
            .outermost_paths()
            .into_iter()
            .next()
            .unwrap_or_default();
        self.report.hinted(
            self.hir,
            Code::LimitWarning,
            loc,
            &related,
            warned(&counted, bound),
            limit.warning_help(),
        );
        Ok(())
    }
}

pub(in crate::semantics::analysis) fn warned(counted: &str, bound: u64) -> String {
    format!(
        "{counted}, the warning value of the limit of {}",
        grouped(bound)
    )
}
