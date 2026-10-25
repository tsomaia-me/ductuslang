use crate::semantics::analysis::layout::Layout;
use crate::semantics::analysis::shape_fault::ShapeFault;

pub(crate) type Shaped = Result<Option<Layout>, Option<ShapeFault>>;
