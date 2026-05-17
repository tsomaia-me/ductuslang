use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use std::sync::atomic::AtomicI32;
use synaptic_kernel::primitives::types::AtomicBuffer;
use synaptic_kernel::primitives::simple_free_list::SimpleFreeList;
use synaptic_kernel::primitives::slot::SlotId;

fn create_mem(size: usize) -> AtomicBuffer {
    (0..size).map(|_| AtomicI32::new(0)).collect()
}

fn bench_alloc(c: &mut Criterion) {
    let mem = create_mem(1_000_000);
    let fl = SimpleFreeList::new(mem, 0, 131072);

    c.bench_function("SimpleFreeList/alloc", |b| {
        b.iter(|| black_box(fl.alloc()));
    });
}

fn bench_alloc_free_cycle(c: &mut Criterion) {
    let mem = create_mem(1_000_000);
    let fl = SimpleFreeList::new(mem, 0, 131072);

    c.bench_function("SimpleFreeList/alloc+free_cycle", |b| {
        b.iter(|| {
            let slot = fl.alloc().unwrap();
            black_box(fl.free(slot).unwrap());
        });
    });
}

fn bench_alloc_exhausted(c: &mut Criterion) {
    let mem = create_mem(4096);
    let fl = SimpleFreeList::new(mem, 0, 4);

    // Exhaust the free list
    for _ in 0..4 {
        fl.alloc().unwrap();
    }

    c.bench_function("SimpleFreeList/alloc_exhausted", |b| {
        b.iter(|| black_box(fl.alloc()));
    });
}

fn bench_batch_alloc(c: &mut Criterion) {
    let mut group = c.benchmark_group("SimpleFreeList/batch_alloc");

    for &batch in &[10, 100, 1_000] {
        group.bench_with_input(BenchmarkId::from_parameter(batch), &batch, |b, &batch| {
            b.iter_with_setup(
                || {
                    let mem = create_mem(1_000_000);
                    SimpleFreeList::new(mem, 0, 16384)
                },
                |fl| {
                    for _ in 0..batch {
                        black_box(fl.alloc().unwrap());
                    }
                },
            );
        });
    }

    group.finish();
}

fn bench_batch_free(c: &mut Criterion) {
    let mut group = c.benchmark_group("SimpleFreeList/batch_free");

    for &batch in &[10, 100, 1_000] {
        group.bench_with_input(BenchmarkId::from_parameter(batch), &batch, |b, &batch| {
            b.iter_with_setup(
                || {
                    let mem = create_mem(1_000_000);
                    let fl = SimpleFreeList::new(mem, 0, 16384);
                    let slots: Vec<SlotId> = (0..batch).map(|_| fl.alloc().unwrap()).collect();
                    (fl, slots)
                },
                |(fl, slots)| {
                    for s in slots {
                        black_box(fl.free(s).unwrap());
                    }
                },
            );
        });
    }

    group.finish();
}

fn bench_double_free_check(c: &mut Criterion) {
    let mem = create_mem(1_000_000);
    let fl = SimpleFreeList::new(mem, 0, 131072);

    // Alloc and free one slot — it's now free
    let slot = fl.alloc().unwrap();
    fl.free(slot).unwrap();

    // Benchmark the double-free detection path (bitmap check returns early)
    c.bench_function("SimpleFreeList/double_free_detect", |b| {
        b.iter(|| black_box(fl.free(slot)));
    });
}

fn bench_high_fragmentation(c: &mut Criterion) {
    c.bench_function("SimpleFreeList/fragmented_alloc_free", |b| {
        b.iter_with_setup(
            || {
                let mem = create_mem(1_000_000);
                let fl = SimpleFreeList::new(mem, 0, 4096);
                // Alloc all, free odd slots → 50% fragmented
                let slots: Vec<SlotId> = (0..4096).map(|_| fl.alloc().unwrap()).collect();
                for s in &slots {
                    if s.to_usize() % 2 == 1 {
                        fl.free(*s).unwrap();
                    }
                }
                fl
            },
            |fl| {
                // Alloc from fragmented list
                for _ in 0..100 {
                    let s = fl.alloc().unwrap();
                    black_box(s);
                    fl.free(s).unwrap();
                }
            },
        );
    });
}

criterion_group!(
    benches,
    bench_alloc,
    bench_alloc_free_cycle,
    bench_alloc_exhausted,
    bench_batch_alloc,
    bench_batch_free,
    bench_double_free_check,
    bench_high_fragmentation,
);
criterion_main!(benches);
