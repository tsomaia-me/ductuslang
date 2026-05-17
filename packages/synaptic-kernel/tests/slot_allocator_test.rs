use std::sync::atomic::AtomicI32;
use std::sync::Arc;
use synaptic_kernel::primitives::slot::SlotId;
use synaptic_kernel::primitives::slot_allocator::SlotAllocator;
use synaptic_kernel::primitives::types::AtomicBuffer;

fn create_allocator(capacity: u32) -> (SlotAllocator, AtomicBuffer) {
    let size = SlotAllocator::calculate_size_on_mem(capacity as usize);
    let mem: AtomicBuffer = (0..size).map(|_| AtomicI32::new(0)).collect();
    let alloc = SlotAllocator::new(Arc::clone(&mem), 0, capacity);
    (alloc, mem)
}

#[test]
fn alloc_defer_flush_lifecycle() {
    let (alloc, _mem) = create_allocator(4);
    let reader = alloc.to_staging_buffer_reader();

    let s1 = alloc.alloc().unwrap();
    assert_eq!(alloc.free_count(), 3);
    assert_eq!(alloc.alloc_count(), 1);
    assert_eq!(alloc.deferred_count(), 0);

    alloc.defer_free(s1).unwrap();
    assert_eq!(alloc.free_count(), 3); // logically deferred, technically not freed yet
    assert_eq!(alloc.alloc_count(), 1);
    assert_eq!(alloc.deferred_count(), 1);

    alloc.publish(); // advances generation, staged queue requires an ack
    reader.ack(); // simulates reader sync
    alloc.publish(); // now freed

    assert_eq!(alloc.free_count(), 4);
    assert_eq!(alloc.alloc_count(), 0);
    assert_eq!(alloc.deferred_count(), 0);
}

#[test]
fn unallocated_deferral_timebomb_is_disarmed() {
    let (alloc, _mem) = create_allocator(4);

    // Attempting to defer slot 1 which has NOT been allocated
    let res = alloc.defer_free(SlotId::new(1).unwrap());
    assert!(matches!(
        res,
        Err(synaptic_kernel::errors::slot_allocator_error::SlotAllocatorError::InvalidSlot)
    ));

    // Nothing should be deferred
    assert_eq!(alloc.deferred_count(), 0);
}

#[test]
fn double_defer_returns_error() {
    let (alloc, _mem) = create_allocator(4);
    let s1 = alloc.alloc().unwrap();

    alloc.defer_free(s1).unwrap();
    let res = alloc.defer_free(s1);

    assert!(matches!(
        res,
        Err(synaptic_kernel::errors::slot_allocator_error::SlotAllocatorError::DoubleFree)
    ));
}

#[test]
#[should_panic]
fn is_allocated_panics_if_out_of_bounds() {
    let (alloc, _mem) = create_allocator(4);
    alloc.is_allocated(SlotId::new(5).unwrap()); // panics natively via Bitmap bounds
}

#[test]
fn slot_id_zero_is_unrepresentable() {
    // SlotId is a NonZeroU32; constructing one from 0 yields None, so the
    // "defer_free of slot 0" panic is now compile-impossible.
    assert!(SlotId::new(0).is_none());
    assert!(SlotId::from_i32(0).is_none());
}

#[test]
#[should_panic]
fn defer_free_panics_if_beyond_capacity() {
    let (alloc, _mem) = create_allocator(4);
    let _ = alloc.defer_free(SlotId::new(5).unwrap());
}
