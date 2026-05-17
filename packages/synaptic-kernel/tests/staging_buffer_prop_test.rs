use proptest::prelude::*;
use std::sync::atomic::AtomicI32;
use std::sync::Arc;
use synaptic_kernel::primitives::slot::SlotId;
use synaptic_kernel::primitives::staging_buffer_reader::StagingBufferReader;
use synaptic_kernel::primitives::staging_buffer_writer::StagingBufferWriter;
use synaptic_kernel::primitives::types::AtomicBuffer;

const STAGING_CAPACITY: u32 = 1024;

fn create_staging(capacity: u32) -> (StagingBufferWriter, StagingBufferReader, AtomicBuffer) {
    let size = StagingBufferWriter::calculate_size_on_mem(capacity as usize);
    let mem: AtomicBuffer = (0..size).map(|_| AtomicI32::new(0)).collect();
    let buffer = StagingBufferWriter::new(Arc::clone(&mem), 0, capacity);
    let reader = buffer.to_reader();
    (buffer, reader, mem)
}

#[derive(Debug, Clone)]
enum SpscOp {
    Push(SlotId),
    Publish,
    ReaderAck,
    Drain,
}

struct Oracle {
    writer_generation: i32,
    reader_ack_generation: i32,
    buffer_state: Vec<(SlotId, i32)>, // (item, generation_stamp)
    drained_history: Vec<SlotId>,
}

impl Oracle {
    fn new() -> Self {
        Self {
            writer_generation: 1, // Starts at 1 natively in StagingBuffer
            reader_ack_generation: 0,
            buffer_state: Vec::new(),
            drained_history: Vec::new(),
        }
    }

    fn push(&mut self, item: SlotId) {
        if self.buffer_state.len() < STAGING_CAPACITY as usize {
            self.buffer_state.push((item, self.writer_generation));
        }
    }

    fn publish(&mut self) {
        self.writer_generation += 1;
    }

    fn reader_ack(&mut self) {
        // Reader acknowledges strictly up to the generation prior to current writer generation.
        // It reads writer_generation and explicitly sets ack to writer_generation - 1.
        self.reader_ack_generation = self.writer_generation - 1;
    }

    fn drain(&mut self) -> Vec<SlotId> {
        let mut drained = Vec::new();
        let mut kept = Vec::new();

        for &(item, stamp) in &self.buffer_state {
            // Signed-wrapping "stamp <= ack" — matches staging buffer's drain gate.
            if stamp.wrapping_sub(self.reader_ack_generation) <= 0 {
                drained.push(item);
                self.drained_history.push(item);
            } else {
                kept.push((item, stamp));
            }
        }

        self.buffer_state = kept;
        drained
    }
}

proptest! {
    #[test]
    fn staging_buffer_spsc_generation_fuzz(
        ops in prop::collection::vec(
            prop_oneof![
                // SlotId is NonZeroU32, so we sample 1..=1000.
                4 => (1u32..=1000).prop_map(|n| SpscOp::Push(SlotId::new(n).unwrap())),
                2 => Just(SpscOp::Publish),
                2 => Just(SpscOp::ReaderAck),
                3 => Just(SpscOp::Drain),
            ],
            1..1000 // sequence size
        )
    ) {
        let (buf, reader, _mem) = create_staging(STAGING_CAPACITY);
        let mut oracle = Oracle::new();

        for op in ops {
            match op {
                SpscOp::Push(item) => {
                    if buf.len() < STAGING_CAPACITY as usize {
                        buf.push(item).unwrap();
                        oracle.push(item);
                    }
                }
                SpscOp::Publish => {
                    buf.publish();
                    oracle.publish();

                    assert_eq!(buf.writer_generation(), oracle.writer_generation);
                }
                SpscOp::ReaderAck => {
                    reader.ack();
                    oracle.reader_ack();

                    assert_eq!(buf.reader_ack_generation(), oracle.reader_ack_generation);
                }
                SpscOp::Drain => {
                    let actual_drained: Vec<SlotId> = buf.drain().collect();
                    let oracle_drained = oracle.drain();

                    assert_eq!(actual_drained.len(), oracle_drained.len(), "Drain count mismatch");
                    assert_eq!(actual_drained, oracle_drained, "Drained items mismatch");
                    assert_eq!(buf.len(), oracle.buffer_state.len(), "Remaining buffer len mismatch");
                }
            }
        }

        // Final invariant check: after we publish and ack everything,
        // the remaining buffer should completely drain.
        buf.publish();
        oracle.publish();

        reader.ack();
        oracle.reader_ack();

        let final_actual_drained: Vec<SlotId> = buf.drain().collect();
        let final_oracle_drained = oracle.drain();

        assert_eq!(final_actual_drained, final_oracle_drained);
        assert_eq!(buf.len(), 0, "Buffer should be entirely empty after final sync");
    }
}
