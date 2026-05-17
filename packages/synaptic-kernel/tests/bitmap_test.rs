use std::sync::atomic::AtomicI32;
use std::sync::Arc;
use std::thread;
use synaptic_kernel::primitives::bitmap::Bitmap;
use synaptic_kernel::primitives::types::AtomicBuffer;

fn create_mem(size: usize) -> AtomicBuffer {
    (0..size).map(|_| AtomicI32::new(0)).collect()
}

#[test]
fn new_creates_bitmap_all_off() {
    let mem = create_mem(100);
    let bitmap = Bitmap::new(Arc::clone(&mem), 0, 32);
    for i in 0..32 {
        assert!(bitmap.is_off(i));
        assert!(!bitmap.is_on(i));
    }
}

#[test]
fn on_and_off_toggles_correctly() {
    let mem = create_mem(100);
    let bitmap = Bitmap::new(Arc::clone(&mem), 0, 32);

    bitmap.on(5);
    assert!(bitmap.is_on(5));
    assert!(bitmap.is_off(4));
    assert!(bitmap.is_off(6));

    bitmap.off(5);
    assert!(bitmap.is_off(5));
}

#[test]
fn copy_from_transfers_bit_state() {
    let mem1 = create_mem(100);
    let b1 = Bitmap::new(Arc::clone(&mem1), 0, 32);
    b1.on(1);
    b1.on(31);

    let mem2 = create_mem(100);
    let b2 = Bitmap::new(Arc::clone(&mem2), 0, 32);
    b2.copy_from(&b1);

    assert!(b2.is_on(1));
    assert!(b2.is_on(31));
    assert!(b2.is_off(0));
}

#[test]
#[should_panic]
fn is_on_panics_out_of_bounds() {
    let mem = create_mem(100);
    let bitmap = Bitmap::new(Arc::clone(&mem), 0, 32);
    bitmap.is_on(32);
}

#[test]
#[should_panic]
fn on_panics_out_of_bounds() {
    let mem = create_mem(100);
    let bitmap = Bitmap::new(Arc::clone(&mem), 0, 32);
    bitmap.on(32);
}

#[test]
#[should_panic]
fn copy_from_panics_if_source_larger() {
    let mem1 = create_mem(100);
    let b_large = Bitmap::new(Arc::clone(&mem1), 0, 64);

    let mem2 = create_mem(100);
    let b_small = Bitmap::new(Arc::clone(&mem2), 0, 32);

    // Attempting to copy 64 bits into 32 bits capacity will natively panic due to user's fix
    b_small.copy_from(&b_large);
}

#[test]
fn bitmap_thread_stress_test() {
    let mem = create_mem(100);
    let bitmap = Arc::new(Bitmap::new(Arc::clone(&mem), 0, 32));

    let mut handles = vec![];
    for i in 0..16 {
        let bit = bitmap.clone();
        handles.push(thread::spawn(move || {
            // each thread toggles its own isolated bit repeatedly
            for _ in 0..10_000 {
                bit.on(i);
                bit.off(i);
                bit.on(i);
            }
        }));
    }

    for handle in handles {
        handle.join().unwrap();
    }

    // because we end on `on()`, all 16 bits should be exactly on
    for i in 0..16 {
        assert!(
            bitmap.is_on(i),
            "Bit {} was dropped during concurrent access",
            i
        );
    }
}
