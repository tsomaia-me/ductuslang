//! Loom model of the staging-buffer generation handshake.
//!
//! Re-implements the protocol from
//! `src/primitives/staging_buffer_writer.rs` + `staging_buffer_reader.rs`
//! against `loom::sync::atomic`. This file does NOT use the kernel's real
//! types — it tests a model of the protocol, not the production code.
//!
//! Real protocol summary:
//!
//!   - `writer_generation` starts at 1, `reader_ack_generation` at 0.
//!   - Producer `push(slot)`: writes `(slot, current_writer_generation)`
//!     into a ring buffer (producer-only).
//!   - Producer `publish()`: `writer_generation.fetch_add(1, Relaxed)`.
//!   - Consumer `ack()`: stores `writer_generation - 1` into
//!     `reader_ack_generation` with `Release`.
//!   - Producer `drain()`: reads `reader_ack_generation` with `Acquire`,
//!     yields entries whose stamped generation `<=` ack via wrapping
//!     comparison.
//!
//! Invariants under test:
//!
//!   1. An entry pushed at generation G is never drained until the
//!      reader has acked some generation `>= G`.
//!   2. When acked at `>= G`, drained entries pushed at `<= G` come out
//!      in FIFO push order.
//!   3. The `wrapping_sub` comparison stays correct under all reachable
//!      loom interleavings.

#![cfg(feature = "loom_tests")]

use loom::sync::atomic::{AtomicI32, Ordering};
use loom::sync::Arc;
use loom::thread;
use std::cell::RefCell;
use std::sync::atomic::{AtomicBool, Ordering as StdOrdering};

/// Loom permits std atomics for tracking outside the model-under-test.
/// These flags accumulate across every loom-explored interleaving for the
/// life of a single `loom::model` invocation, then we assert the model
/// reached a "positive" state at least once. Without this, a model where
/// `drain()` always returned nothing (because we mishandled the sync)
/// would pass silently.
static SAW_DRAIN: AtomicBool = AtomicBool::new(false);

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
struct Entry {
    slot: i32,
    generation: i32,
}

/// SPSC staging buffer model. The cross-thread atomics are the two
/// generation counters; the ring buffer state is producer-only and lives
/// inside a `RefCell` in the producer thread, so it doesn't need
/// loom-typed wrappers.
struct StagingBuffer {
    writer_gen: AtomicI32,
    reader_ack_gen: AtomicI32,
}

impl StagingBuffer {
    fn new() -> Self {
        StagingBuffer {
            writer_gen: AtomicI32::new(1),
            reader_ack_gen: AtomicI32::new(0),
        }
    }

    /// Producer-side: snapshot the current writer generation for stamping
    /// a freshly-pushed entry. Relaxed mirrors the real kernel.
    fn current_writer_gen(&self) -> i32 {
        self.writer_gen.load(Ordering::Relaxed)
    }

    /// Producer-side: advance the writer generation. Relaxed mirrors the
    /// real `publish()`. The handshake's release semantics live on the
    /// reader's `ack()` — see `read_acked_gen`.
    fn publish(&self) {
        self.writer_gen.fetch_add(1, Ordering::Relaxed);
    }

    /// Consumer-side: ack `writer_gen - 1` with Release, exactly as
    /// `StagingBufferReader::ack` does.
    fn ack(&self) {
        let w = self.writer_gen.load(Ordering::Relaxed);
        self.reader_ack_gen.store(w - 1, Ordering::Release);
    }

    /// Producer-side: read the ack with Acquire (matches the real
    /// `StagingBufferWriter::reader_ack_generation`).
    fn read_acked_gen(&self) -> i32 {
        self.reader_ack_gen.load(Ordering::Acquire)
    }
}

/// Drain the producer-local ring against an ack snapshot. Returns the
/// drained entries in FIFO push order. Mirrors the real
/// `StagingBufferWriterIterator::next` predicate
/// `entry.generation.wrapping_sub(ack) > 0  =>  block`.
fn drain(ring: &mut Vec<Entry>, ack: i32) -> Vec<Entry> {
    let mut out = Vec::new();
    while let Some(e) = ring.first().copied() {
        if e.generation.wrapping_sub(ack) > 0 {
            break;
        }
        ring.remove(0);
        out.push(e);
    }
    out
}

#[test]
fn loom_staging_buffer_drain_blocks_on_unacked_generations() {
    // Producer pushes one entry at gen 1, publishes (gen->2), pushes one
    // at gen 2, publishes (gen->3). Consumer does `ack()` at some
    // unconstrained point, possibly never observing a particular gen.
    //
    // Invariant: any entry the producer drains has `entry.generation <= ack_gen`
    // at the moment of drain. Anything else means the protocol exposed
    // an entry the consumer hasn't agreed is safe to reclaim.
    SAW_DRAIN.store(false, StdOrdering::Relaxed);
    loom::model(|| {
        let buf = Arc::new(StagingBuffer::new());

        let producer_buf = Arc::clone(&buf);
        let producer = thread::spawn(move || {
            // Producer-local ring of pending entries.
            let ring: RefCell<Vec<Entry>> = RefCell::new(Vec::new());

            // push at gen 1
            let g = producer_buf.current_writer_gen();
            ring.borrow_mut().push(Entry { slot: 100, generation: g });
            producer_buf.publish();

            // drain mid-protocol — may or may not yield
            {
                let ack = producer_buf.read_acked_gen();
                let drained = drain(&mut ring.borrow_mut(), ack);
                for e in drained {
                    assert!(
                        e.generation.wrapping_sub(ack) <= 0,
                        "drained entry gen {} > ack {}",
                        e.generation,
                        ack
                    );
                    SAW_DRAIN.store(true, StdOrdering::Relaxed);
                }
            }

            // push at gen 2
            let g = producer_buf.current_writer_gen();
            ring.borrow_mut().push(Entry { slot: 200, generation: g });
            producer_buf.publish();

            // final drain after both publishes
            {
                let ack = producer_buf.read_acked_gen();
                let drained = drain(&mut ring.borrow_mut(), ack);
                for e in drained {
                    assert!(
                        e.generation.wrapping_sub(ack) <= 0,
                        "drained entry gen {} > ack {}",
                        e.generation,
                        ack
                    );
                    SAW_DRAIN.store(true, StdOrdering::Relaxed);
                }
            }
        });

        let consumer_buf = Arc::clone(&buf);
        let consumer = thread::spawn(move || {
            consumer_buf.ack();
        });

        producer.join().unwrap();
        consumer.join().unwrap();
    });
    assert!(
        SAW_DRAIN.load(StdOrdering::Relaxed),
        "no loom interleaving drained any entry — protocol model is stuck"
    );
}

#[test]
fn loom_staging_buffer_drain_yields_fifo_after_full_ack() {
    // Stronger sequencing: consumer ack happens AFTER producer publishes
    // both entries (we sequence by joining the producer-publish phase
    // first via a barrier-equivalent dance — actually we keep the
    // concurrent shape and rely on loom to also explore the order where
    // ack lands after both publishes).
    //
    // When the ack snapshot the producer reads is `>= 2`, both entries
    // (gen 1 and gen 2) MUST drain in push order: 100 then 200. Any
    // partial ack (1 or 0) drains a prefix. A wrapping-arithmetic bug
    // would surface as out-of-order or missing entries.
    SAW_DRAIN.store(false, StdOrdering::Relaxed);
    loom::model(|| {
        let buf = Arc::new(StagingBuffer::new());

        let producer_buf = Arc::clone(&buf);
        let producer = thread::spawn(move || {
            let ring: RefCell<Vec<Entry>> = RefCell::new(Vec::new());

            let g1 = producer_buf.current_writer_gen();
            ring.borrow_mut().push(Entry { slot: 100, generation: g1 });
            producer_buf.publish();

            let g2 = producer_buf.current_writer_gen();
            ring.borrow_mut().push(Entry { slot: 200, generation: g2 });
            producer_buf.publish();

            let ack = producer_buf.read_acked_gen();
            let drained = drain(&mut ring.borrow_mut(), ack);

            // Whatever subset drained must be a contiguous FIFO prefix:
            // first 100 (gen=1), then 200 (gen=2).
            let expected_full = [
                Entry { slot: 100, generation: 1 },
                Entry { slot: 200, generation: 2 },
            ];
            for (i, e) in drained.iter().enumerate() {
                assert_eq!(*e, expected_full[i], "FIFO order broken at index {}", i);
                SAW_DRAIN.store(true, StdOrdering::Relaxed);
            }
        });

        let consumer_buf = Arc::clone(&buf);
        let consumer = thread::spawn(move || {
            consumer_buf.ack();
        });

        producer.join().unwrap();
        consumer.join().unwrap();
    });
    assert!(
        SAW_DRAIN.load(StdOrdering::Relaxed),
        "no loom interleaving observed a drain — model not exercising the path"
    );
}

#[test]
fn loom_staging_buffer_unacked_entry_is_invisible() {
    // Setup: producer has pushed at gen 1 and not yet published. The
    // consumer's ack of `writer_gen - 1` evaluates to 0 (since
    // writer_gen is still 1). Any drain in this window must yield
    // nothing — entry's gen is 1, ack is 0, `1.wrapping_sub(0) > 0` is
    // true so the iterator returns None.
    loom::model(|| {
        let buf = Arc::new(StagingBuffer::new());

        let producer_buf = Arc::clone(&buf);
        let producer = thread::spawn(move || {
            let ring: RefCell<Vec<Entry>> = RefCell::new(Vec::new());

            let g = producer_buf.current_writer_gen(); // 1
            ring.borrow_mut().push(Entry { slot: 100, generation: g });

            // Drain WITHOUT publish. The reader can ack at most
            // writer_gen - 1 = 0. Drain must return nothing.
            let ack = producer_buf.read_acked_gen();
            let drained = drain(&mut ring.borrow_mut(), ack);
            assert!(
                drained.is_empty(),
                "drained {} entries before publish (ack={})",
                drained.len(),
                ack
            );
        });

        let consumer_buf = Arc::clone(&buf);
        let consumer = thread::spawn(move || {
            // Consumer might race in here and ack — the maximum ack
            // value is still writer_gen - 1 = 0 since no publish has
            // happened yet.
            consumer_buf.ack();
        });

        producer.join().unwrap();
        consumer.join().unwrap();
    });
}
