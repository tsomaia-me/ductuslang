use super::core::Core;
use super::core_value::CoreValue;
use super::memo_arg::MemoArg;
use super::memo_entry::MemoEntry;
use super::memo_key::MemoKey;
use super::recording::Recording;
use crate::diagnostics::Diagnostic;
use crate::semantics::counted_walk::CountedWalk;
use crate::semantics::operators::EvalTy;

impl Core<'_> {
    pub fn memo_key(
        &self,
        body: u32,
        ty: EvalTy,
        args: &[CoreValue],
        unit: Option<(symphonyscript_core::FamilyId, usize, u32)>,
    ) -> Option<MemoKey> {
        if self.stop.is_some() {
            return None;
        }
        let args = args.iter().map(MemoArg::of).collect::<Option<Vec<_>>>()?;
        Some(MemoKey {
            body,
            ty,
            flags: [self.fitting, self.owning, self.unchecked],
            args,
            unit,
        })
    }

    pub fn memo_hit(&mut self, key: &MemoKey, call: u32) -> Option<CoreValue> {
        CountedWalk::CallMemo.step();
        let entry = self.call_memo.get(key)?;
        let after = self.fold_steps.checked_add(entry.steps)?;
        let warning = self.limits.expansion_steps_warning;
        if after > self.limits.expansion_steps || (self.fold_steps <= warning && after > warning) {
            return None;
        }
        let value = entry.value.clone();
        let faults = entry.faults.clone();
        self.fold_steps = after;
        self.push_call(call);
        for fault in faults {
            let fault = self.chained(fault);
            self.push_drawn(fault);
        }
        self.eval_chain.pop();
        Some(value)
    }

    pub fn begin_recording(&mut self, key: MemoKey, frames: usize) {
        self.recordings.push(Recording {
            key,
            frames,
            depth: self.eval_chain.len(),
            steps: self.fold_steps,
            faults: Vec::new(),
            storable: true,
        });
    }

    pub fn end_recording(&mut self, frames: usize, value: &CoreValue) {
        if self
            .recordings
            .last()
            .is_none_or(|recording| recording.frames != frames)
        {
            return;
        }
        let Some(recording) = self.recordings.pop() else {
            return;
        };
        if !recording.storable || self.stop.is_some() {
            return;
        }
        self.call_memo.insert(
            recording.key,
            MemoEntry {
                value: value.clone(),
                faults: recording.faults,
                steps: self.fold_steps.saturating_sub(recording.steps),
            },
        );
    }

    pub fn record(&mut self, diagnostic: &Diagnostic) {
        for recording in &mut self.recordings {
            let mut kept = diagnostic.clone();
            let keep = kept.related.len().saturating_sub(recording.depth + 1);
            kept.related.truncate(keep);
            recording.faults.push(kept);
        }
    }

    pub fn unmemoized(&mut self) {
        for recording in self.recordings.iter_mut().rev() {
            if !recording.storable {
                break;
            }
            recording.storable = false;
        }
    }
}
