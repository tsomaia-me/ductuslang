use crate::epoch_mirror::EpochMirror;
use std::sync::atomic::{AtomicI32, AtomicPtr, Ordering};

/// Lock-free control plane for initial delivery and later hot-swapping of
/// the active epoch.
///
/// Owns the current `Box<EpochMirror>` and provides atomic access to it.
/// Acts as the sole source of truth indicating which `EpochMirror` instance the
/// consumer thread should traverse. Orchestrates the safe delivery of the initial epoch, as well
/// as hot-swapping to new epoch instances when the kernel reallocates due to `grow()`.
///
/// # Mechanism
/// The kernel initializes this with a `Box<EpochMirror>` via `new()`.
/// When `grow()` occurs, the kernel calls `swap_epoch()` with the new reader.
/// `swap_epoch()` atomically swaps the internal pointer, advances the writer generation,
/// and returns the old `(old_reader, deletion_gen)` - the old reader paired with its
/// generation stamp for deferred deletion.
/// The kernel holds the pair in a deletion queue and frees it only once the consumer's
/// acknowledged generation reaches the stamp.
///
/// On the consumer side, `acquire_mirror()` acknowledges the current generation **before**
/// loading the epoch pointer, ensuring the consumer's ack never exceeds the generation
/// of the epoch it actually receives.
///
/// # Threading
/// Wait-free SPSC synchronization.
/// - `swap_epoch()`: `AcqRel` on the `AtomicPtr` swap; `Release` on the writer generation
///   increment (producer-only, follows the `swap()`).
/// - `acquire_mirror()`: `Release` on the ack store; `Acquire` on the `AtomicPtr` load.
///   The internal writer generation read uses `Acquire` (synchronizes against producer's
///   `Release` in `swap_epoch()`).
/// - `get_reader_ack_generation()`: `Acquire` (synchronizes against
///   consumer's `Release` in `acquire_mirror()`).
///
/// # Constraints
/// - `acquire_mirror()`: Consumer thread only.
/// - `swap_epoch()`: Producer thread only.
/// - `get_reader_ack_generation()`: Producer thread only (Reads consumer's ack).
#[repr(C)]
pub struct ControlPlane<const TB_COUNT: usize, const STORE_COUNT: usize, const LUT_COUNT: usize> {
    mirror_ptr: AtomicPtr<EpochMirror<TB_COUNT, STORE_COUNT, LUT_COUNT>>,
    writer_generation: AtomicI32,
    reader_ack_generation: AtomicI32,
}

impl<const TB_COUNT: usize, const STORE_COUNT: usize, const LUT_COUNT: usize>
    ControlPlane<TB_COUNT, STORE_COUNT, LUT_COUNT>
{
    pub fn new(mirror: Box<EpochMirror<TB_COUNT, STORE_COUNT, LUT_COUNT>>) -> Self {
        ControlPlane {
            mirror_ptr: AtomicPtr::new(Box::into_raw(mirror)),
            writer_generation: AtomicI32::new(0),
            reader_ack_generation: AtomicI32::new(0),
        }
    }

    pub(crate) fn acquire_mirror(&self) -> &EpochMirror<TB_COUNT, STORE_COUNT, LUT_COUNT> {
        self.ack();

        let epoch_ptr = self.mirror_ptr.load(Ordering::Acquire);

        // SAFETY: The pointer is always valid, because it's managed by Kernel's Box lifecycle
        // and generation-gated deferred deletion.
        unsafe { &*epoch_ptr }
    }

    pub fn swap_epoch(
        &self,
        new_epoch: Box<EpochMirror<TB_COUNT, STORE_COUNT, LUT_COUNT>>,
    ) -> (Box<EpochMirror<TB_COUNT, STORE_COUNT, LUT_COUNT>>, i32) {
        let new_epoch_ptr = Box::into_raw(new_epoch);
        let old_epoch_ptr = self.mirror_ptr.swap(new_epoch_ptr, Ordering::AcqRel);
        // Two separate atomic operations (pointer swap + generation bump),
        // not one combined atomic. Loom-verified (see tests/loom_control_plane.rs).
        // The AcqRel on the swap above pairs with the Acquire load in
        // `acquire_mirror()`. The Release on the fetch_add below pairs with
        // the Acquire load of `writer_generation` in `ack()`. The two pairs
        // together guarantee: any consumer that observes the bumped generation
        // must also see the new mirror_ptr. These should not be combined, orderings
        // should not be weakened or reordered.
        let prev_gen = self.writer_generation.fetch_add(1, Ordering::Release);

        // SAFETY: old_epoch_ptr was originally created by Box::into_raw() in a prior
        // swap_epoch() or ControlPlane::new(). The atomic swap guarantees exclusive access -
        // no other thread holds this epoch after the swap().
        let old_epoch = unsafe { Box::from_raw(old_epoch_ptr) };

        (old_epoch, prev_gen + 1)
    }

    pub fn get_writer_generation(&self) -> i32 {
        self.writer_generation.load(Ordering::Acquire)
    }

    pub fn get_reader_ack_generation(&self) -> i32 {
        self.reader_ack_generation.load(Ordering::Acquire)
    }

    fn ack(&self) {
        let writer_generation = self.writer_generation.load(Ordering::Acquire);
        self.reader_ack_generation
            .store(writer_generation, Ordering::Release)
    }
}

impl<const TB_COUNT: usize, const STORE_COUNT: usize, const LUT_COUNT: usize> Drop
    for ControlPlane<TB_COUNT, STORE_COUNT, LUT_COUNT>
{
    fn drop(&mut self) {
        // SAFETY: The pointer was created by Box::into_raw() in a prior
        // swap_epoch() or ControlPlane::new(). `&mut self` guarantees exclusive access.
        // No concurrent load is possible.
        unsafe {
            drop(Box::from_raw(self.mirror_ptr.load(Ordering::Relaxed)));
        }
    }
}
