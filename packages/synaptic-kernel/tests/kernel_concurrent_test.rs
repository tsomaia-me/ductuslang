mod common;

use synaptic_kernel::epoch_consumer::EpochConsumer;
use synaptic_kernel::kernel::Kernel;
use synaptic_kernel::kernel_config::KernelConfig;
use synaptic_kernel::primitives::slot::SlotId;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;

const NODE_META: usize = 8;
const NODE_ATTR: usize = 16;
const SYNAPSE_META: usize = 8;
const SYNAPSE_ATTR: usize = 16;

type TestKernel = Kernel<1, 1, 1>;
type TestConsumer = EpochConsumer<1, 1, 1>;

fn config() -> KernelConfig<1, 1, 1> {
    common::kernel_config_1_1(128, 256, NODE_META, NODE_ATTR, SYNAPSE_META, SYNAPSE_ATTR)
}

#[test]
fn multi_threaded_topology_fuzzer() {
    // The Kernel stays in the main thread; only an Arc<ControlPlane> clone is
    // shipped to the reader thread. The reader is joined before the kernel
    // drops, so the kernel's debug-time Drop assert sees strong_count == 1.
    let mut writer = TestKernel::new(config());
    let cp = writer.get_control_plane();

    let is_running = Arc::new(AtomicBool::new(true));
    let is_running_reader = Arc::clone(&is_running);

    let iterations = 100_000;

    // --- READER THREAD ---
    // Continuously traverses whatever entry point the writer just published,
    // proving that structural pointers never point to stale or freed memory
    // across staging-buffer generation boundaries. The reader does not know
    // the writer's chain head a priori — it walks from each node it can
    // reach via the entry slot the writer leaves in `mem_metadata[0]`.
    let reader_handle = thread::spawn(move || {
        let mut consumer = TestConsumer::new(cp);
        let mut total_iterations = 0u64;
        let mut max_nodes_seen = 0usize;

        while is_running_reader.load(Ordering::Acquire) {
            let reader = consumer.acquire_mirror();
            total_iterations += 1;

            let head_slot = match SlotId::from_i32(reader.mem_read_meta(0)) {
                Some(s) => s,
                None => continue,
            };

            let mut current = Some(reader.get_node(head_slot));
            let mut node_count = 0usize;

            while let Some(node) = current {
                node_count += 1;
                assert!(node_count <= 128, "Node loop");
                let _attr: i32 = node.get_meta(0);

                let mut syn_slot = node.get_outgoing_synapse_head();
                let mut syn_count = 0usize;
                while let Some(s) = syn_slot {
                    syn_count += 1;
                    assert!(syn_count <= 256, "Out syn loop");
                    let syn = reader.get_synapse(s);
                    // get_target_ptr returns SlotId (always non-zero by construction).
                    let _ = syn.get_target_ptr();
                    syn_slot = syn.get_outgoing_next_ptr();
                }

                let mut in_slot = node.get_incoming_synapse_head();
                let mut in_count = 0usize;
                while let Some(s) = in_slot {
                    in_count += 1;
                    assert!(in_count <= 256, "In syn loop");
                    let syn = reader.get_synapse(s);
                    // get_source_ptr returns SlotId (always non-zero by construction).
                    let _ = syn.get_source_ptr();
                    in_slot = syn.get_incoming_next_ptr();
                }

                current = node.get_next_ptr().map(|next| reader.get_node(next));
            }

            if node_count > max_nodes_seen {
                max_nodes_seen = node_count;
            }
        }

        (total_iterations, max_nodes_seen)
        // consumer drops here, releasing its Arc<ControlPlane> clone.
    });

    // --- WRITER WORK (main thread) ---
    for round in 0..iterations {
        let mut nodes = Vec::new();
        let mut synapses = Vec::new();

        // Build a fresh chain so the reader has a valid entry to walk.
        let head = match writer.insert_node(0) {
            Ok(slot) => slot,
            Err(_) => break, // capacity will eventually exhaust if reader stalls
        };
        nodes.push(head);
        writer.get_node(head).attr_write(0, round);
        writer.mem_write_meta(0, head.to_i32());
        let mut prev = head;
        for i in 1..64 {
            match writer.insert_node_after(prev, i) {
                Ok(slot) => {
                    writer.get_node(slot).attr_write(0, round);
                    nodes.push(slot);
                    prev = slot;
                }
                Err(_) => break,
            }
        }

        writer.publish();

        // Synapses: linear chain + star from head.
        if nodes.len() > 1 {
            for i in 0..nodes.len() - 1 {
                if let Ok(s) = writer.connect(nodes[i], nodes[i + 1], 99) {
                    synapses.push(s);
                }
                if let Ok(s) = writer.connect(nodes[0], nodes[i + 1], 88) {
                    synapses.push(s);
                }
            }
        }

        writer.publish();

        // Tear down half the synapses.
        for (i, &s) in synapses.iter().enumerate() {
            if i % 2 == 0 {
                let _ = writer.disconnect_synapse(s);
            }
        }

        writer.publish();

        // Tear down the chain — also marks the entry slot stale.
        writer.mem_write_meta(0, 0);
        let _ = writer.remove_chain(head);

        writer.publish();
    }

    is_running.store(false, Ordering::Release);
    let (_iters, _max_nodes) = reader_handle.join().expect("reader thread panicked");
    // writer (the kernel) drops here. Arc count is now 1.
}
