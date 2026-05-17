use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

mod common;

use synaptic_kernel::epoch_consumer::EpochConsumer;
use synaptic_kernel::kernel::Kernel;
use synaptic_kernel::kernel_config::KernelConfig;
use synaptic_kernel::primitives::slot::SlotId;

const NODE_META: usize = 8;
const NODE_ATTR: usize = 16;
const SYNAPSE_META: usize = 8;
const SYNAPSE_ATTR: usize = 16;

type TestKernel = Kernel<1, 1, 1>;
type TestProcessor = EpochConsumer<1, 1, 1>;

fn config(n: u32, s: u32) -> KernelConfig<1, 1, 1> {
    common::kernel_config_1_1(n, s, NODE_META, NODE_ATTR, SYNAPSE_META, SYNAPSE_ATTR)
}

// ============ Epoch Stress: Grow Under Consumer Load with Proper Ack ============
//
// Pattern across this file: the Kernel stays in the main thread; the consumer
// thread receives an `Arc::clone(&control_plane)` (Arc<ControlPlane> is Send +
// Sync) and is joined BEFORE the kernel is dropped. This guarantees the
// kernel's debug-time Drop assert sees `strong_count == 1`.

/// The critical test: main thread grows while consumer thread traverses using
/// the EpochConsumer interface (acquire_mirror + ack). Validates that
/// epoch-based reclamation prevents use-after-free.
#[test]
fn epoch_stress_grow_under_consumer_load_with_ack() {
    let mut controller = TestKernel::new(config(8, 8));
    let cp = controller.get_control_plane();

    // Seed initial data; n1 is the chain head we'll traverse from.
    let n1 = controller.insert_node(1).unwrap();
    let n2 = controller.insert_node_after(n1, 2).unwrap();
    controller.connect(n1, n2, 10).unwrap();
    controller.get_node(n1).attr_write(0, 42);
    controller.mem_write_meta(0, n1.to_i32());
    controller.publish();

    let running = Arc::new(AtomicBool::new(true));
    let running_consumer = Arc::clone(&running);

    let consumer_thread = thread::spawn(move || {
        let mut processor = TestProcessor::new(cp);
        let mut iterations = 0u64;
        let mut max_chain_len = 0usize;

        while running_consumer.load(Ordering::Relaxed) {
            let graph = processor.acquire_mirror();

            let head_slot = match SlotId::from_i32(graph.mem_read_meta(0)) {
                Some(s) => s,
                None => {
                    iterations += 1;
                    continue;
                }
            };

            let mut current = Some(graph.get_node(head_slot));
            let mut count = 0usize;
            while let Some(node) = current {
                let kind: i32 = node.get_kind();
                assert!(
                    kind >= 0 && kind < 200,
                    "corrupt kind: {} at iteration {}",
                    kind,
                    iterations
                );

                match node.get_next_ptr() {
                    Some(next_ptr) => {
                        current = Some(graph.get_node(next_ptr));
                        count += 1;
                        assert!(
                            count <= 128,
                            "chain exceeded max length — possible cycle at iteration {}",
                            iterations
                        );
                    }
                    None => break,
                }
            }

            if count > max_chain_len {
                max_chain_len = count;
            }

            iterations += 1;
            thread::yield_now();
        }

        (iterations, max_chain_len)
    });

    controller.grow(config(16, 16)).unwrap();
    controller.publish();

    let mut prev = n2;
    for i in 3..14 {
        prev = controller.insert_node_after(prev, i).unwrap();
    }
    controller.publish();

    controller.grow(config(32, 32)).unwrap();
    controller.publish();

    for i in 14..28 {
        prev = controller.insert_node_after(prev, i).unwrap();
    }
    controller.publish();

    controller.grow(config(64, 64)).unwrap();
    controller.publish();

    thread::sleep(Duration::from_millis(20));

    for _ in 0..5 {
        controller.publish();
        thread::sleep(Duration::from_millis(2));
    }

    running.store(false, Ordering::Relaxed);
    let (iterations, _max_chain) = consumer_thread.join().expect("consumer thread panicked");
    assert!(iterations > 0, "consumer thread never ran");
}

// ============ Epoch Stress: Random Operations Under Load ============

#[test]
fn epoch_stress_random_mutations_under_consumer_load() {
    let mut controller = TestKernel::new(config(16, 16));
    let cp = controller.get_control_plane();

    // Build an initial chain head[0] -> head[1] -> ... -> head[7].
    let mut node_slots = Vec::new();
    let head = controller.insert_node(0).unwrap();
    node_slots.push(head);
    let mut prev = head;
    for i in 1..8 {
        let s = controller.insert_node_after(prev, i).unwrap();
        node_slots.push(s);
        prev = s;
    }
    controller.mem_write_meta(0, head.to_i32());

    let mut synapse_slots = Vec::new();
    for i in 0..node_slots.len() - 1 {
        let s = controller
            .connect(node_slots[i], node_slots[i + 1], (i * 10) as i32)
            .unwrap();
        synapse_slots.push(s);
    }
    controller.publish();

    let running = Arc::new(AtomicBool::new(true));
    let running_consumer = Arc::clone(&running);

    let consumer_thread = thread::spawn(move || {
        let mut processor = TestProcessor::new(cp);
        let mut iterations = 0u64;

        while running_consumer.load(Ordering::Relaxed) {
            let graph = processor.acquire_mirror();

            let head_slot = match SlotId::from_i32(graph.mem_read_meta(0)) {
                Some(s) => s,
                None => {
                    iterations += 1;
                    continue;
                }
            };

            let mut current = Some(graph.get_node(head_slot));
            let mut node_count = 0usize;
            while let Some(node) = current {
                let kind: i32 = node.get_kind();
                assert!(kind >= 0, "negative kind: {}", kind);

                let mut syn_slot_opt = node.get_outgoing_synapse_head();
                let mut syn_count = 0;
                while let Some(syn_slot) = syn_slot_opt {
                    let syn = graph.get_synapse(syn_slot);
                    let _syn_kind: i32 = syn.get_kind();
                    syn_slot_opt = syn.get_outgoing_next_ptr();
                    syn_count += 1;
                    assert!(
                        syn_count <= 64,
                        "synapse chain too long — possible cycle"
                    );
                }

                match node.get_next_ptr() {
                    Some(next_ptr) => {
                        current = Some(graph.get_node(next_ptr));
                        node_count += 1;
                        assert!(node_count <= 128, "chain too long");
                    }
                    None => break,
                }
            }
            iterations += 1;
        }

        iterations
    });

    for batch in 0..20 {
        // Add nodes to the tail of our chain.
        for i in 0..3 {
            let kind = (batch * 10 + i) as i32;
            if let Some(&tail) = node_slots.last() {
                if let Ok(slot) = controller.insert_node_after(tail, kind) {
                    node_slots.push(slot);
                }
            }
        }

        if node_slots.len() > 10 {
            while let Some(s) = synapse_slots.pop() {
                let _ = controller.disconnect_synapse(s);
            }

            for _ in 0..2 {
                if node_slots.len() > 1 {
                    let slot = node_slots.pop().unwrap();
                    let _ = controller.remove_node(slot);
                }
            }
        }

        if node_slots.len() >= 2 {
            let src = node_slots[0];
            let tgt = node_slots[node_slots.len() - 1];
            if let Ok(s) = controller.connect(src, tgt, batch as i32) {
                synapse_slots.push(s);
            }
        }

        if let Some(&slot) = node_slots.first() {
            controller.get_node(slot).attr_write(0, batch as i32 * 100);
        }

        controller.publish();

        if batch == 5 {
            controller.grow(config(32, 32)).unwrap();
            controller.publish();
        }
        if batch == 12 {
            controller.grow(config(64, 64)).unwrap();
            controller.publish();
        }
    }

    thread::sleep(Duration::from_millis(10));

    for _ in 0..5 {
        controller.publish();
    }

    running.store(false, Ordering::Relaxed);
    let iterations = consumer_thread
        .join()
        .expect("consumer thread panicked during random mutations");
    assert!(iterations > 0, "consumer thread never ran");
}

// ============ Epoch Stress: Consumer Thread Acking at Varying Speeds ============

#[test]
fn epoch_stress_slow_ack_does_not_crash() {
    let mut controller = TestKernel::new(config(8, 8));
    let cp = controller.get_control_plane();

    let head = controller.insert_node(1).unwrap();
    controller.mem_write_meta(0, head.to_i32());
    controller.publish();

    let running = Arc::new(AtomicBool::new(true));
    let running_consumer = Arc::clone(&running);

    let consumer_thread = thread::spawn(move || {
        let mut processor = TestProcessor::new(cp);
        let mut iterations = 0u64;

        while running_consumer.load(Ordering::Relaxed) {
            let graph = processor.acquire_mirror();

            if let Some(head_slot) = SlotId::from_i32(graph.mem_read_meta(0)) {
                let mut current = Some(graph.get_node(head_slot));
                while let Some(node) = current {
                    match node.get_next_ptr() {
                        Some(next) => current = Some(graph.get_node(next)),
                        None => break,
                    }
                }
            }

            thread::sleep(Duration::from_millis(5));
            iterations += 1;
        }

        iterations
    });

    let grow_caps = [16, 32, 64, 128, 256];
    let mut prev = head;
    for (i, &new_cap) in grow_caps.iter().enumerate() {
        controller.grow(config(new_cap, new_cap)).unwrap();

        for j in 0..4 {
            if let Ok(slot) = controller.insert_node_after(prev, (i * 10 + j) as i32) {
                prev = slot;
            }
        }

        controller.publish();
        thread::sleep(Duration::from_millis(3));
    }

    thread::sleep(Duration::from_millis(30));

    for _ in 0..10 {
        controller.publish();
        thread::sleep(Duration::from_millis(2));
    }

    running.store(false, Ordering::Relaxed);
    let iterations = consumer_thread
        .join()
        .expect("consumer thread panicked with slow ack");
    assert!(iterations > 0, "consumer thread never ran");
}

// ============ Epoch Stress: Attribute Writes During Traversal ============

#[test]
fn epoch_stress_concurrent_attribute_writes_with_processor() {
    let controller = TestKernel::new(config(16, 16));
    let cp = controller.get_control_plane();

    let mut slots = Vec::new();
    for i in 0..8 {
        let s = controller.insert_node(i).unwrap();
        for offset in 0..16 {
            controller.get_node(s).attr_write(offset, 0);
        }
        slots.push(s);
    }

    let running = Arc::new(AtomicBool::new(true));
    let running_consumer = Arc::clone(&running);
    let slots_clone = slots.clone();

    let consumer_thread = thread::spawn(move || {
        let mut processor = TestProcessor::new(cp);
        let mut iterations = 0u64;

        while running_consumer.load(Ordering::Relaxed) {
            let graph = processor.acquire_mirror();

            for &slot in &slots_clone {
                for offset in 0..16 {
                    let _ = graph.get_node(slot).attr_read(offset);
                }
            }
            iterations += 1;
        }

        iterations
    });

    for batch in 0..500 {
        for &slot in &slots {
            for offset in 0..16 {
                controller
                    .get_node(slot)
                    .attr_write(offset, (offset as i32) * 1000 + batch);
            }
        }
    }

    thread::sleep(Duration::from_millis(5));

    running.store(false, Ordering::Relaxed);
    let iterations = consumer_thread
        .join()
        .expect("consumer thread panicked during attribute writes");
    assert!(iterations > 0, "consumer thread never ran");
}

// ============ Epoch Stress: Multiple Rapid Grows Without Consumer Ack ============

#[test]
fn epoch_stress_grows_accumulate_without_ack() {
    let mut controller = TestKernel::new(config(4, 4));

    let n1 = controller.insert_node(1).unwrap();

    // Grow 10 times rapidly WITHOUT any consumer thread acking.
    // Validates that readers_pending_deletion accumulates safely.
    for i in 1..=10 {
        let cap = 4 * (1 << i);
        if cap <= 4096 {
            controller.grow(config(cap, cap)).unwrap();
            controller.publish();
        }
    }

    {
        let mut processor = TestProcessor::new(controller.get_control_plane());
        let graph = processor.acquire_mirror();
        assert_eq!(graph.get_node(n1).get_kind(), 1);
        // processor drops at end of this scope, releasing its Arc clone.
    }

    controller.publish();

    let n2 = controller.insert_node(2).unwrap();
    assert_eq!(controller.get_node(n2).get_kind(), 2);
}
