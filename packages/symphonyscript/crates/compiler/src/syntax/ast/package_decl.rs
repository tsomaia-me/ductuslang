use super::record_lit::RecordLit;
use super::support::child;

ast_node!(PackageDecl);

impl<'p> PackageDecl<'p> {
    pub fn record(&self) -> Option<RecordLit<'p>> {
        child(&self.0)
    }
}
