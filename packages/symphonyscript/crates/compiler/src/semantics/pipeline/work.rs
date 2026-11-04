use super::loop_run::LoopRun;

pub(crate) enum Work {
    Items(u32, u32, u32),
    Members(u32, u32, u32),
    Loop(LoopRun),
}
