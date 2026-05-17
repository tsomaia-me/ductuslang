use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use std::sync::atomic::AtomicI32;
use synaptic_kernel::primitives::types::AtomicBuffer;
use synaptic_kernel::primitives::ring_buffer::RingBuffer;

fn create_mem(size: usize) -> AtomicBuffer {
    (0..size).map(|_| AtomicI32::new(0)).collect()
}

fn bench_write(c: &mut Criterion) {
    let mut group = c.benchmark_group("RingBuffer/write");

    for &slot_size_label in &[1, 4, 16] {
        match slot_size_label {
            1 => {
                let mem = create_mem(1_000_000);
                let ring: RingBuffer<1> = RingBuffer::new(mem, 0, 131072);
                group.bench_function(BenchmarkId::new("STRIDE", 1), |b| {
                    b.iter(|| black_box(ring.write([0])));
                });
            }
            4 => {
                let mem = create_mem(1_000_000);
                let ring: RingBuffer<4> = RingBuffer::new(mem, 0, 131072);
                group.bench_function(BenchmarkId::new("STRIDE", 4), |b| {
                    b.iter(|| black_box(ring.write([0, 1, 2, 3])));
                });
            }
            16 => {
                let mem = create_mem(4_000_000);
                let ring: RingBuffer<16> = RingBuffer::new(mem, 0, 131072);
                group.bench_function(BenchmarkId::new("STRIDE", 16), |b| {
                    b.iter(|| black_box(ring.write([0; 16])));
                });
            }
            _ => {}
        }
    }

    group.finish();
}

fn bench_read(c: &mut Criterion) {
    let mut group = c.benchmark_group("RingBuffer/read");

    // Pre-fill and read single ops
    for &slot_size_label in &[1, 4, 16] {
        match slot_size_label {
            1 => {
                let mem = create_mem(1_000_000);
                let ring: RingBuffer<1> = RingBuffer::new(mem, 0, 131072);
                for i in 0..100_000 {
                    ring.write([i]).unwrap();
                }
                group.bench_function(BenchmarkId::new("STRIDE", 1), |b| {
                    b.iter(|| black_box(ring.read()));
                });
            }
            4 => {
                let mem = create_mem(1_000_000);
                let ring: RingBuffer<4> = RingBuffer::new(mem, 0, 131072);
                for i in 0..100_000 {
                    ring.write([i, 0, 0, 0]).unwrap();
                }
                group.bench_function(BenchmarkId::new("STRIDE", 4), |b| {
                    b.iter(|| black_box(ring.read()));
                });
            }
            16 => {
                let mem = create_mem(4_000_000);
                let ring: RingBuffer<16> = RingBuffer::new(mem, 0, 131072);
                for i in 0..100_000 {
                    ring.write([i; 16]).unwrap();
                }
                group.bench_function(BenchmarkId::new("STRIDE", 16), |b| {
                    b.iter(|| black_box(ring.read()));
                });
            }
            _ => {}
        }
    }

    group.finish();
}

fn bench_write_read_interleaved(c: &mut Criterion) {
    let mem = create_mem(4096);
    let ring: RingBuffer<4> = RingBuffer::new(mem, 0, 64);

    c.bench_function("RingBuffer/write+read_interleaved", |b| {
        b.iter(|| {
            black_box(ring.write([1, 2, 3, 4]).unwrap());
            black_box(ring.read().unwrap());
        });
    });
}

fn bench_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("RingBuffer/throughput");

    for &batch in &[100, 1_000, 10_000] {
        group.bench_with_input(BenchmarkId::new("write_batch", batch), &batch, |b, &batch| {
            b.iter_with_setup(
                || {
                    let mem = create_mem(1_000_000);
                    RingBuffer::<4>::new(mem, 0, 131072)
                },
                |ring| {
                    for i in 0..batch {
                        black_box(ring.write([i as i32, 0, 0, 0]).unwrap());
                    }
                },
            );
        });
    }

    group.finish();
}

fn bench_empty_read(c: &mut Criterion) {
    let mem = create_mem(4096);
    let ring: RingBuffer<4> = RingBuffer::new(mem, 0, 64);

    c.bench_function("RingBuffer/read_empty", |b| {
        b.iter(|| black_box(ring.read()));
    });
}

fn bench_full_write(c: &mut Criterion) {
    let mem = create_mem(4096);
    let ring: RingBuffer<4> = RingBuffer::new(mem, 0, 64);

    // Fill the buffer
    for _ in 0..64 {
        ring.write([0, 0, 0, 0]).unwrap();
    }

    c.bench_function("RingBuffer/write_full", |b| {
        b.iter(|| black_box(ring.write([0, 0, 0, 0])));
    });
}

criterion_group!(
    benches,
    bench_write,
    bench_read,
    bench_write_read_interleaved,
    bench_throughput,
    bench_empty_read,
    bench_full_write,
);
criterion_main!(benches);
