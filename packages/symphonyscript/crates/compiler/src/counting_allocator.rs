#![allow(unsafe_code)]

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicU64, Ordering};

pub(crate) struct CountingAllocator;

static ALLOCATIONS: AtomicU64 = AtomicU64::new(0);
static LIVE: AtomicU64 = AtomicU64::new(0);
static PEAK: AtomicU64 = AtomicU64::new(0);

fn bytes(size: usize) -> u64 {
    u64::try_from(size).unwrap_or(u64::MAX)
}

fn grown(size: usize) {
    let live = LIVE.fetch_add(bytes(size), Ordering::Relaxed) + bytes(size);
    PEAK.fetch_max(live, Ordering::Relaxed);
}

fn shrunk(size: usize) {
    LIVE.fetch_sub(bytes(size), Ordering::Relaxed);
}

impl CountingAllocator {
    pub(crate) fn begin() -> (u64, u64) {
        let live = LIVE.load(Ordering::Relaxed);
        PEAK.store(live, Ordering::Relaxed);
        (ALLOCATIONS.load(Ordering::Relaxed), live)
    }

    pub(crate) fn held_since((_, live): (u64, u64)) -> u64 {
        LIVE.load(Ordering::Relaxed).saturating_sub(live)
    }

    pub(crate) fn since((allocations, live): (u64, u64)) -> (u64, u64) {
        (
            ALLOCATIONS
                .load(Ordering::Relaxed)
                .saturating_sub(allocations),
            PEAK.load(Ordering::Relaxed).saturating_sub(live),
        )
    }
}

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let pointer = unsafe { System.alloc(layout) };
        if !pointer.is_null() {
            ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
            grown(layout.size());
        }
        pointer
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let pointer = unsafe { System.alloc_zeroed(layout) };
        if !pointer.is_null() {
            ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
            grown(layout.size());
        }
        pointer
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        unsafe { System.dealloc(pointer, layout) };
        shrunk(layout.size());
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let moved = unsafe { System.realloc(pointer, layout, new_size) };
        if !moved.is_null() {
            ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
            if new_size >= layout.size() {
                grown(new_size - layout.size());
            } else {
                shrunk(layout.size() - new_size);
            }
        }
        moved
    }
}
