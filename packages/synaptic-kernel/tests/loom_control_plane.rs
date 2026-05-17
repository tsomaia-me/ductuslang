//! Loom model of the ControlPlane epoch-swap handshake.
//!
//! Re-implements the protocol from `src/control_plane.rs` against
//! `loom::sync::atomic`. This file does NOT use the kernel's real types —
//! it tests a model of the protocol, not the production code.
//!
//! Real protocol summary:
//!
//!   - Producer `swap_epoch(new)`:
//!       1. `mirror_ptr.swap(new, AcqRel)` returning `old_ptr`.
//!       2. `writer_generation.fetch_add(1, Release)` returning `prev_gen`.
//!       Returns `(old_box, prev_gen + 1)`. The kernel pushes the pair
//!       onto `readers_pending_deletion` and frees `old_box` only once
//!       `reader_ack_generation >= prev_gen + 1`.
//!
//!   - Consumer `acquire_mirror`:
//!       1. `ack()`:
//!           a. `let w = writer_generation.load(Acquire)`.
//!           b. `reader_ack_generation.store(w, Release)`.
//!       2. `mirror_ptr.load(Acquire)` to obtain the current mirror.
//!
//! Invariants under test:
//!
//!   1. **Ack ≤ received-mirror generation.** The consumer's ack store
//!      must never name a generation strictly greater than the mirror it
//!      ends up loading. Phrased as `acked_value <= G(loaded_ptr)`.
//!      Violation would mean the producer could free a mirror the
//!      consumer is about to use.
//!   2. **No generation skip.** `swap_epoch` returns a strictly
//!      increasing generation each call (single-thread property).
//!   3. **Once gen N+1 is observable, the new pointer is observable.**
//!      The Acquire on `mirror_ptr.load` must sync with the `AcqRel` of
//!      `mirror_ptr.swap` whenever the consumer's read of
//!      `writer_generation` already saw the post-swap value.

#![cfg(feature = "loom_tests")]

use loom::sync::atomic::{AtomicI32, AtomicPtr, Ordering};
use loom::sync::Arc;
use loom::thread;
use std::sync::atomic::{AtomicBool, Ordering as StdOrdering};

/// Tracks "consumer observed the post-swap state at least once across
/// all loom-explored interleavings." Without this, a model where the
/// consumer always saw the pre-swap state would pass the safety
/// invariants vacuously.
static SAW_NEW_MIRROR: AtomicBool = AtomicBool::new(false);
static SAW_GEN_BUMP: AtomicBool = AtomicBool::new(false);

/// Stand-in for `Box<EpochMirror>`. Each instance carries the generation
/// it was installed at — we leak these on purpose; loom models don't run
/// destructors and we only care about the synchronization, not memory.
#[repr(C)]
struct Mirror {
    generation: i32,
}

fn install_mirror(generation: i32) -> *mut Mirror {
    Box::into_raw(Box::new(Mirror { generation }))
}

/// Model of `ControlPlane`. Holds an `AtomicPtr` over Mirror plus the two
/// generation counters.
struct ControlPlane {
    mirror_ptr: AtomicPtr<Mirror>,
    writer_generation: AtomicI32,
    reader_ack_generation: AtomicI32,
}

impl ControlPlane {
    fn new(initial_mirror: *mut Mirror) -> Self {
        ControlPlane {
            mirror_ptr: AtomicPtr::new(initial_mirror),
            writer_generation: AtomicI32::new(0),
            reader_ack_generation: AtomicI32::new(0),
        }
    }

    /// Producer-side. Mirrors the real `swap_epoch`.
    fn swap_epoch(&self, new: *mut Mirror) -> (*mut Mirror, i32) {
        let old = self.mirror_ptr.swap(new, Ordering::AcqRel);
        let prev = self.writer_generation.fetch_add(1, Ordering::Release);
        (old, prev + 1)
    }

    /// Consumer-side. Mirrors the real `acquire_mirror` exactly:
    /// ack first, then load. Returns `(ack_value, loaded_ptr)`.
    fn acquire_mirror(&self) -> (i32, *mut Mirror) {
        // ack(): writer_gen.load(Acquire) → reader_ack_gen.store(Release).
        let w = self.writer_generation.load(Ordering::Acquire);
        self.reader_ack_generation.store(w, Ordering::Release);

        // Then load the mirror.
        let p = self.mirror_ptr.load(Ordering::Acquire);
        (w, p)
    }
}

#[test]
fn loom_control_plane_ack_never_exceeds_received_mirror_generation() {
    // Single producer swap, single consumer acquire. Every loom
    // interleaving must satisfy: consumer's ack value <= the generation
    // of the mirror it received.
    SAW_NEW_MIRROR.store(false, StdOrdering::Relaxed);
    loom::model(|| {
        let m0 = install_mirror(0);
        let m1 = install_mirror(1);
        let cp = Arc::new(ControlPlane::new(m0));

        let producer_cp = Arc::clone(&cp);
        let producer = thread::spawn(move || {
            let (_old, _gen) = producer_cp.swap_epoch(m1);
        });

        let consumer_cp = Arc::clone(&cp);
        let consumer = thread::spawn(move || {
            let (ack, p) = consumer_cp.acquire_mirror();
            // SAFETY: in this loom model the mirrors are leaked — both
            // m0 and m1 stay valid. The deref reads only the generation
            // field (no concurrent writers since each Mirror is created
            // before being installed and never mutated after).
            let g = unsafe { (*p).generation };
            assert!(
                ack <= g,
                "consumer acked gen {} but received mirror at gen {}",
                ack,
                g
            );
            if g == 1 {
                SAW_NEW_MIRROR.store(true, StdOrdering::Relaxed);
            }
        });

        producer.join().unwrap();
        consumer.join().unwrap();
    });
    assert!(
        SAW_NEW_MIRROR.load(StdOrdering::Relaxed),
        "no loom interleaving observed the post-swap mirror — protocol stuck pre-swap"
    );
}

#[test]
fn loom_control_plane_acquire_after_visible_gen_bump_sees_new_ptr() {
    // Stronger property derived from the AcqRel on swap and the Acquire
    // on the consumer's mirror_ptr load: if the consumer's
    // writer_generation read sees the post-swap value (1), then the
    // subsequent mirror_ptr load MUST see the new pointer (gen 1, not
    // the original gen 0). Acquire-Release ordering pairs the two.
    SAW_GEN_BUMP.store(false, StdOrdering::Relaxed);
    loom::model(|| {
        let m0 = install_mirror(0);
        let m1 = install_mirror(1);
        let cp = Arc::new(ControlPlane::new(m0));

        let producer_cp = Arc::clone(&cp);
        let producer = thread::spawn(move || {
            let (_old, _gen) = producer_cp.swap_epoch(m1);
        });

        let consumer_cp = Arc::clone(&cp);
        let consumer = thread::spawn(move || {
            let (ack, p) = consumer_cp.acquire_mirror();
            let g = unsafe { (*p).generation };
            // If consumer saw the bumped writer_generation (ack==1),
            // it must also see the new mirror.
            if ack == 1 {
                SAW_GEN_BUMP.store(true, StdOrdering::Relaxed);
                assert_eq!(
                    g, 1,
                    "ack=1 but mirror generation={} — Acquire on mirror_ptr.load failed to sync with AcqRel on swap"
                ,g);
            }
        });

        producer.join().unwrap();
        consumer.join().unwrap();
    });
    assert!(
        SAW_GEN_BUMP.load(StdOrdering::Relaxed),
        "no loom interleaving observed ack==1 (gen-bump-already-visible path)"
    );
}

#[test]
fn loom_control_plane_swap_returns_strictly_increasing_generations() {
    // Single-thread sanity: two consecutive swap_epoch calls must return
    // different, strictly increasing generations. Loom can't reorder a
    // single thread but this still exercises the fetch_add semantics.
    loom::model(|| {
        let m0 = install_mirror(0);
        let m1 = install_mirror(1);
        let m2 = install_mirror(2);
        let cp = Arc::new(ControlPlane::new(m0));

        let (_old1, gen1) = cp.swap_epoch(m1);
        let (_old2, gen2) = cp.swap_epoch(m2);

        assert_eq!(gen1, 1);
        assert_eq!(gen2, 2);
        assert!(gen1 < gen2);
    });
}

#[test]
fn loom_control_plane_initial_acquire_returns_initial_mirror() {
    // No producer thread: the consumer's first acquire_mirror returns
    // the initial mirror at generation 0 with ack 0. Sanity for the
    // initial-state path — distinct from the swap interleavings above.
    loom::model(|| {
        let m0 = install_mirror(0);
        let cp = Arc::new(ControlPlane::new(m0));

        let consumer_cp = Arc::clone(&cp);
        let consumer = thread::spawn(move || {
            let (ack, p) = consumer_cp.acquire_mirror();
            let g = unsafe { (*p).generation };
            assert_eq!(ack, 0);
            assert_eq!(g, 0);
        });

        consumer.join().unwrap();
    });
}
