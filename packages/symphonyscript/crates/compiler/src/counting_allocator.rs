#![allow(unsafe_code)]

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

pub(crate) struct CountingAllocator;

thread_local! {
    static ALLOCATIONS: Cell<u64> = const { Cell::new(0) };
    static LIVE: Cell<i64> = const { Cell::new(0) };
    static PEAK: Cell<i64> = const { Cell::new(0) };
}

fn bytes(size: usize) -> i64 {
    i64::try_from(size).unwrap_or(i64::MAX)
}

fn counted() {
    let _ = ALLOCATIONS.try_with(|count| count.set(count.get().saturating_add(1)));
}

fn live() -> i64 {
    LIVE.try_with(Cell::get).unwrap_or(0)
}

fn grown(size: usize) {
    let _ = LIVE.try_with(|live| {
        let now = live.get().saturating_add(bytes(size));
        live.set(now);
        let _ = PEAK.try_with(|peak| peak.set(peak.get().max(now)));
    });
}

fn shrunk(size: usize) {
    let _ = LIVE.try_with(|live| live.set(live.get().saturating_sub(bytes(size))));
}

fn apart(later: i64, earlier: i64) -> u64 {
    u64::try_from(later.saturating_sub(earlier)).unwrap_or(0)
}

impl CountingAllocator {
    pub(crate) fn begin() -> (u64, i64) {
        let now = live();
        let _ = PEAK.try_with(|peak| peak.set(now));
        (ALLOCATIONS.try_with(Cell::get).unwrap_or(0), now)
    }

    pub(crate) fn held_since((_, start): (u64, i64)) -> u64 {
        apart(live(), start)
    }

    pub(crate) fn since((allocations, start): (u64, i64)) -> (u64, u64) {
        (
            ALLOCATIONS
                .try_with(Cell::get)
                .unwrap_or(0)
                .saturating_sub(allocations),
            apart(PEAK.try_with(Cell::get).unwrap_or(start), start),
        )
    }
}

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let pointer = unsafe { System.alloc(layout) };
        if !pointer.is_null() {
            counted();
            grown(layout.size());
        }
        pointer
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let pointer = unsafe { System.alloc_zeroed(layout) };
        if !pointer.is_null() {
            counted();
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
            counted();
            if new_size >= layout.size() {
                grown(new_size - layout.size());
            } else {
                shrunk(layout.size() - new_size);
            }
        }
        moved
    }
}
