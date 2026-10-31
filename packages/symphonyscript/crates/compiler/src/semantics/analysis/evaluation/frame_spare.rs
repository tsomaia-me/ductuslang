use super::frame::Frame;
use super::frame_values::FrameValues;
use crate::semantics::analysis::c_val::CVal;
use crate::semantics::analysis::fast_hash::FastSet;
use crate::semantics::analysis::feed::Feed;
use crate::semantics::analysis::ids::{CallId, ScopeId};

#[derive(Default)]
pub(crate) struct FrameSpare {
    pub values: FrameValues,
    pub loops: FrameValues,
    pub faults: FastSet<u32>,
    pub templates: FastSet<ScopeId>,
    pub opened: FastSet<CallId>,
    pub args: Vec<CVal>,
    pub feeds: Vec<Feed>,
}

impl FrameSpare {
    pub fn kept(frame: Frame) -> FrameSpare {
        let mut spare = FrameSpare {
            values: frame.values,
            loops: frame.loops,
            faults: frame.faults,
            templates: frame.templates,
            opened: frame.opened,
            args: frame.args,
            feeds: frame.feeds,
        };
        spare.values.clear();
        spare.loops.clear();
        spare.faults.clear();
        spare.templates.clear();
        spare.opened.clear();
        spare.args.clear();
        spare.feeds.clear();
        spare
    }
}
