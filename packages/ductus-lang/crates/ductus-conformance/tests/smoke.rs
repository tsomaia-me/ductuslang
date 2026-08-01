//! Scaffold smoke test: proves the three crates link and the workspace test
//! harness runs.

use ductus_conformance as _;
use ductus_ir as _;
use ductus_runtime as _;

#[test]
fn workspace_links() {
    assert_eq!(1 + 1, 2);
}
