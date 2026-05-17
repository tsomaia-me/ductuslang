use std::sync::atomic::AtomicI32;
use std::sync::Arc;
use synaptic_kernel::primitives::entry_store_config::EntryStoreConfig;
use synaptic_kernel::primitives::entry_store_def::{EntryStoreDef, EntryStoreId};
#[allow(unused_imports)]
use synaptic_kernel::primitives::entry_store_writer::EntryStoreWriter;
use synaptic_kernel::primitives::entry_store_writer_registry::EntryStoreWriterRegistry;
use synaptic_kernel::primitives::slot::SlotId;
use synaptic_kernel::primitives::slot_allocator::SlotAllocator;
use synaptic_kernel::primitives::triple_buffer_def::{TripleBufferDef, TripleBufferId};
use synaptic_kernel::primitives::triple_buffer_writer_registry::TripleBufferWriterRegistry;
use synaptic_kernel::primitives::types::AtomicBuffer;

const MEM_SIZE: usize = 131072; // 128K — enough for large stride combos

fn create_mem(size: usize) -> AtomicBuffer {
    (0..size).map(|_| AtomicI32::new(0)).collect()
}

/// Shorthand for creating an EntryStoreDef.
fn sdef(
    id: u16,
    tb_id: TripleBufferId,
    core: usize,
    meta: usize,
    attr: usize,
    cap: u32,
) -> EntryStoreDef {
    EntryStoreDef::new(
        EntryStoreId(id),
        tb_id,
        EntryStoreConfig {
            core_stride: core,
            meta_stride: meta,
            attr_stride: attr,
            capacity: cap,
        },
    )
}

const D: TripleBufferId = TripleBufferId::DEFAULT;

fn tb_def(id: u16, cap: usize) -> TripleBufferDef {
    TripleBufferDef {
        id: TripleBufferId(id),
        buffer_capacity: cap,
    }
}

// ============ Group 1: Construction & layout ============

#[test]
fn construct_single_store_on_default_tb() {
    let mem = create_mem(MEM_SIZE);
    let tb_defs = [tb_def(0, 1024)];
    let tb_reg = TripleBufferWriterRegistry::<1>::new(Arc::clone(&mem), tb_defs, 0, 4096);
    let defs = [sdef(0, D, 8, 0, 16, 4)];
    let reg = EntryStoreWriterRegistry::<1, 1>::new(
        Arc::clone(&mem),
        tb_reg.clone(),
        defs,
        tb_reg.mem_end_offset(),
        0,
        [0; 1],
    );
    assert_eq!(
        reg.mem_end_offset() - reg.mem_start_offset(),
        EntryStoreWriterRegistry::<1, 1>::calculate_size_on_mem(&defs)
    );
    assert_eq!(reg.get(EntryStoreId(0)).capacity(), 4);
}

#[test]
fn construct_single_store_on_user_tb() {
    let mem = create_mem(MEM_SIZE);
    let tb_defs = [tb_def(0, 1024)];
    let tb_reg = TripleBufferWriterRegistry::<1>::new(Arc::clone(&mem), tb_defs, 0, 4096);
    let defs = [sdef(0, TripleBufferId(0), 4, 4, 8, 8)];
    let reg = EntryStoreWriterRegistry::<1, 1>::new(
        Arc::clone(&mem),
        tb_reg.clone(),
        defs,
        tb_reg.mem_end_offset(),
        0,
        [0; 1],
    );
    assert_eq!(reg.get(EntryStoreId(0)).tb_start_offset(), 0);
    assert_eq!(
        EntryStoreWriterRegistry::<1, 1>::calculate_size_on_default_tb(&defs),
        0
    );
    assert_eq!(
        EntryStoreWriterRegistry::<1, 1>::calculate_size_on_tb_for(TripleBufferId(0), &defs),
        (4 + 4) * 8
    );
}

#[test]
fn construct_two_stores_contiguous_on_default_tb() {
    let mem = create_mem(MEM_SIZE);
    let tb_defs = [tb_def(0, 1024)];
    let tb_reg = TripleBufferWriterRegistry::<1>::new(Arc::clone(&mem), tb_defs, 0, 4096);
    let defs = [sdef(0, D, 4, 2, 8, 4), sdef(1, D, 8, 0, 16, 8)];
    let reg = EntryStoreWriterRegistry::<1, 2>::new(
        Arc::clone(&mem),
        tb_reg.clone(),
        defs,
        tb_reg.mem_end_offset(),
        0,
        [0; 1],
    );
    let s0 = reg.get(EntryStoreId(0));
    let s1 = reg.get(EntryStoreId(1));
    assert_eq!(s1.tb_start_offset(), s0.tb_end_offset());
    assert_eq!(s1.mem_start_offset(), s0.mem_end_offset());
}

#[test]
fn construct_two_stores_on_same_user_tb() {
    let mem = create_mem(MEM_SIZE);
    let tb_defs = [tb_def(0, 2048)];
    let tb_reg = TripleBufferWriterRegistry::<1>::new(Arc::clone(&mem), tb_defs, 0, 4096);
    let defs = [
        sdef(0, TripleBufferId(0), 4, 2, 8, 4),
        sdef(1, TripleBufferId(0), 16, 8, 32, 8),
    ];
    let reg = EntryStoreWriterRegistry::<1, 2>::new(
        Arc::clone(&mem),
        tb_reg.clone(),
        defs,
        tb_reg.mem_end_offset(),
        0,
        [0; 1],
    );
    let s0 = reg.get(EntryStoreId(0));
    let s1 = reg.get(EntryStoreId(1));
    assert_eq!(s1.tb_start_offset(), s0.tb_end_offset());
    assert_eq!(s1.mem_start_offset(), s0.mem_end_offset());
}

#[test]
fn construct_mixed_default_and_user_tb() {
    let mem = create_mem(MEM_SIZE);
    let tb_defs = [tb_def(0, 2048)];
    let tb_reg = TripleBufferWriterRegistry::<1>::new(Arc::clone(&mem), tb_defs, 0, 4096);
    let defs = [
        sdef(0, D, 8, 4, 16, 4),
        sdef(1, TripleBufferId(0), 4, 0, 8, 8),
    ];
    let reg = EntryStoreWriterRegistry::<1, 2>::new(
        Arc::clone(&mem),
        tb_reg.clone(),
        defs,
        tb_reg.mem_end_offset(),
        0,
        [0; 1],
    );
    let _ = reg;
    assert_eq!(
        EntryStoreWriterRegistry::<1, 2>::calculate_size_on_default_tb(&defs),
        defs[0].size_on_tb()
    );
    assert_eq!(
        EntryStoreWriterRegistry::<1, 2>::calculate_size_on_tb_for(TripleBufferId(0), &defs),
        defs[1].size_on_tb()
    );
}

#[test]
fn construct_with_nonzero_mem_start_offset() {
    let mem = create_mem(MEM_SIZE);
    let tb_defs = [tb_def(0, 16)];
    let tb_reg = TripleBufferWriterRegistry::<1>::new(Arc::clone(&mem), tb_defs, 0, 32);
    assert!(tb_reg.mem_end_offset() <= 256);
    let defs = [sdef(0, D, 8, 0, 16, 4)];
    let reg = EntryStoreWriterRegistry::<1, 1>::new(
        Arc::clone(&mem),
        tb_reg.clone(),
        defs,
        256,
        0,
        [0; 1],
    );
    assert_eq!(reg.mem_start_offset(), 256);
    assert_eq!(reg.get(EntryStoreId(0)).mem_start_offset(), 256);
}

#[test]
fn construct_with_nonzero_tb_start_offsets() {
    let mem = create_mem(MEM_SIZE);
    let tb_defs = [tb_def(0, 4096)];
    let tb_reg = TripleBufferWriterRegistry::<1>::new(Arc::clone(&mem), tb_defs, 0, 8192);
    let defs = [
        sdef(0, D, 8, 0, 16, 4),
        sdef(1, TripleBufferId(0), 4, 4, 8, 8),
    ];
    let reg = EntryStoreWriterRegistry::<1, 2>::new(
        Arc::clone(&mem),
        tb_reg.clone(),
        defs,
        tb_reg.mem_end_offset(),
        64,
        [32; 1],
    );
    assert_eq!(reg.get(EntryStoreId(0)).tb_start_offset(), 64);
    assert_eq!(reg.get(EntryStoreId(1)).tb_start_offset(), 32);
}

// ============ Group 2: Heterogeneous strides ============

#[test]
fn heterogeneous_strides_3_stores_layout_correctness() {
    let mem = create_mem(MEM_SIZE);
    let tb_defs = [tb_def(0, 8192)];
    let tb_reg = TripleBufferWriterRegistry::<1>::new(Arc::clone(&mem), tb_defs, 0, 16384);
    let defs = [
        sdef(0, D, 2, 1, 4, 16),
        sdef(1, D, 16, 8, 32, 4),
        sdef(2, D, 4, 0, 8, 64),
    ];
    let reg = EntryStoreWriterRegistry::<1, 3>::new(
        Arc::clone(&mem),
        tb_reg.clone(),
        defs,
        tb_reg.mem_end_offset(),
        0,
        [0; 1],
    );
    for i in 0..3usize {
        let id = EntryStoreId(i as u16);
        let st = reg.get(id);
        let cfg = defs[i].config();
        let tb_span = cfg.capacity as usize * (cfg.core_stride + cfg.meta_stride);
        assert_eq!(st.tb_end_offset() - st.tb_start_offset(), tb_span);
        let mem_span = SlotAllocator::calculate_size_on_mem(cfg.capacity as usize)
            + cfg.capacity as usize * cfg.attr_stride;
        assert_eq!(st.mem_end_offset() - st.mem_start_offset(), mem_span);
    }
    assert_eq!(
        reg.get(EntryStoreId(1)).tb_start_offset(),
        reg.get(EntryStoreId(0)).tb_end_offset()
    );
    assert_eq!(
        reg.get(EntryStoreId(2)).tb_start_offset(),
        reg.get(EntryStoreId(1)).tb_end_offset()
    );
    assert_eq!(
        reg.get(EntryStoreId(1)).mem_start_offset(),
        reg.get(EntryStoreId(0)).mem_end_offset()
    );
    assert_eq!(
        reg.get(EntryStoreId(2)).mem_start_offset(),
        reg.get(EntryStoreId(1)).mem_end_offset()
    );
}

#[test]
fn heterogeneous_strides_insert_write_read_per_store() {
    let mem = create_mem(MEM_SIZE);
    let tb_defs = [tb_def(0, 8192)];
    let tb_reg = TripleBufferWriterRegistry::<1>::new(Arc::clone(&mem), tb_defs, 0, 16384);
    let defs = [
        sdef(0, D, 2, 1, 4, 16),
        sdef(1, D, 16, 8, 32, 4),
        sdef(2, D, 4, 0, 8, 64),
    ];
    let reg = EntryStoreWriterRegistry::<1, 3>::new(
        Arc::clone(&mem),
        tb_reg.clone(),
        defs,
        tb_reg.mem_end_offset(),
        0,
        [0; 1],
    );
    for i in 0..3usize {
        let id = EntryStoreId(i as u16);
        let st = reg.get(id);
        let cfg = defs[i].config();
        let slot = st.insert().unwrap();
        let ew = st.get(slot);
        for k in 0..cfg.core_stride {
            ew.core_write(k, (100 * i + k) as i32);
        }
        for k in 0..cfg.meta_stride {
            ew.meta_write(k, (1000 * i + k) as i32);
        }
        for k in 0..cfg.attr_stride {
            ew.attr_write(k, (10000 * i + k) as i32);
        }
        for k in 0..cfg.core_stride {
            assert_eq!(ew.core_read(k), (100 * i + k) as i32);
        }
        for k in 0..cfg.meta_stride {
            assert_eq!(ew.meta_read(k), (1000 * i + k) as i32);
        }
        for k in 0..cfg.attr_stride {
            assert_eq!(ew.attr_read(k), (10000 * i + k) as i32);
        }
    }
}

#[test]
fn heterogeneous_strides_fill_to_capacity() {
    let mem = create_mem(MEM_SIZE);
    let tb_defs = [tb_def(0, 4096)];
    let tb_reg = TripleBufferWriterRegistry::<1>::new(Arc::clone(&mem), tb_defs, 0, 8192);
    let defs = [sdef(0, D, 8, 4, 16, 4), sdef(1, D, 2, 0, 4, 16)];
    let reg = EntryStoreWriterRegistry::<1, 2>::new(
        Arc::clone(&mem),
        tb_reg.clone(),
        defs,
        tb_reg.mem_end_offset(),
        0,
        [0; 1],
    );
    let mut slots0: [Option<SlotId>; 4] = [None; 4];
    let mut slots1: [Option<SlotId>; 16] = [None; 16];
    for i in 0..4 {
        slots0[i] = Some(reg.get(EntryStoreId(0)).insert().unwrap());
        reg.get(EntryStoreId(0))
            .get(slots0[i].unwrap())
            .core_write(0, (100 + i) as i32);
        reg.get(EntryStoreId(0))
            .get(slots0[i].unwrap())
            .attr_write(0, (1000 + i) as i32);
    }
    for i in 0..16 {
        slots1[i] = Some(reg.get(EntryStoreId(1)).insert().unwrap());
        reg.get(EntryStoreId(1))
            .get(slots1[i].unwrap())
            .core_write(0, (200 + i) as i32);
        reg.get(EntryStoreId(1))
            .get(slots1[i].unwrap())
            .attr_write(0, (2000 + i) as i32);
    }
    for i in 0..4 {
        let s = slots0[i].unwrap();
        assert_eq!(
            reg.get(EntryStoreId(0)).get(s).core_read(0),
            (100 + i) as i32
        );
        assert_eq!(
            reg.get(EntryStoreId(0)).get(s).attr_read(0),
            (1000 + i) as i32
        );
    }
    for i in 0..16 {
        let s = slots1[i].unwrap();
        assert_eq!(
            reg.get(EntryStoreId(1)).get(s).core_read(0),
            (200 + i) as i32
        );
        assert_eq!(
            reg.get(EntryStoreId(1)).get(s).attr_read(0),
            (2000 + i) as i32
        );
    }
}

#[test]
fn heterogeneous_strides_meta_zone_isolation() {
    let mem = create_mem(MEM_SIZE);
    let tb_defs = [tb_def(0, 2048)];
    let tb_reg = TripleBufferWriterRegistry::<1>::new(Arc::clone(&mem), tb_defs, 0, 4096);
    let defs = [sdef(0, D, 4, 8, 0, 4), sdef(1, D, 4, 2, 0, 4)];
    let reg = EntryStoreWriterRegistry::<1, 2>::new(
        Arc::clone(&mem),
        tb_reg.clone(),
        defs,
        tb_reg.mem_end_offset(),
        0,
        [0; 1],
    );
    let s0 = reg.get(EntryStoreId(0)).insert().unwrap();
    let s1 = reg.get(EntryStoreId(1)).insert().unwrap();
    for k in 0..8 {
        reg.get(EntryStoreId(0))
            .get(s0)
            .meta_write(k, (10 + k) as i32);
    }
    for k in 0..2 {
        reg.get(EntryStoreId(1))
            .get(s1)
            .meta_write(k, (99 + k) as i32);
    }
    tb_reg.get(D).publish();
    let tb_r = tb_reg.to_reader();
    tb_r.get(D).swap();
    let rr = reg.to_reader();
    for k in 0..8 {
        assert_eq!(
            rr.get(EntryStoreId(0)).get(s0).meta_read(k),
            (10 + k) as i32
        );
    }
    for k in 0..2 {
        assert_eq!(
            rr.get(EntryStoreId(1)).get(s1).meta_read(k),
            (99 + k) as i32
        );
    }
}

// ============ Group 3: ID permutation ============

#[test]
fn id_identity_permutation() {
    let mem = create_mem(MEM_SIZE);
    let tb_defs = [tb_def(0, 4096)];
    let tb_reg = TripleBufferWriterRegistry::<1>::new(Arc::clone(&mem), tb_defs, 0, 8192);
    let defs = [
        sdef(0, D, 4, 2, 8, 8),
        sdef(1, D, 8, 0, 16, 4),
        sdef(2, D, 2, 2, 4, 16),
    ];
    let reg = EntryStoreWriterRegistry::<1, 3>::new(
        Arc::clone(&mem),
        tb_reg.clone(),
        defs,
        tb_reg.mem_end_offset(),
        0,
        [0; 1],
    );
    assert!(
        reg.get(EntryStoreId(0)).mem_start_offset() < reg.get(EntryStoreId(1)).mem_start_offset()
    );
    assert!(
        reg.get(EntryStoreId(1)).mem_start_offset() < reg.get(EntryStoreId(2)).mem_start_offset()
    );
}

#[test]
fn id_reversed_permutation() {
    let mem = create_mem(MEM_SIZE);
    let tb_defs = [tb_def(0, 4096)];
    let tb_reg = TripleBufferWriterRegistry::<1>::new(Arc::clone(&mem), tb_defs, 0, 8192);
    let defs = [
        sdef(2, D, 4, 2, 8, 8),
        sdef(1, D, 8, 0, 16, 4),
        sdef(0, D, 2, 2, 4, 16),
    ];
    let reg = EntryStoreWriterRegistry::<1, 3>::new(
        Arc::clone(&mem),
        tb_reg.clone(),
        defs,
        tb_reg.mem_end_offset(),
        0,
        [0; 1],
    );
    assert!(
        reg.get(EntryStoreId(2)).mem_start_offset() < reg.get(EntryStoreId(1)).mem_start_offset()
    );
    assert!(
        reg.get(EntryStoreId(1)).mem_start_offset() < reg.get(EntryStoreId(0)).mem_start_offset()
    );
}

#[test]
fn id_shuffled_permutation_data_roundtrip() {
    let mem = create_mem(MEM_SIZE);
    let tb_defs = [tb_def(0, 4096)];
    let tb_reg = TripleBufferWriterRegistry::<1>::new(Arc::clone(&mem), tb_defs, 0, 8192);
    let defs = [
        sdef(2, D, 16, 8, 32, 4),
        sdef(0, D, 2, 1, 4, 16),
        sdef(1, D, 8, 0, 16, 8),
    ];
    let reg = EntryStoreWriterRegistry::<1, 3>::new(
        Arc::clone(&mem),
        tb_reg.clone(),
        defs,
        tb_reg.mem_end_offset(),
        0,
        [0; 1],
    );
    let mut slots: [Option<SlotId>; 3] = [None; 3];
    for i in 0..3 {
        let id = EntryStoreId([2u16, 0, 1][i]);
        let slot = reg.get(id).insert().unwrap();
        slots[i] = Some(slot);
        reg.get(id).get(slot).core_write(0, (300 + i as i32) * 17);
    }
    tb_reg.get(D).publish();
    let tb_r = tb_reg.to_reader();
    tb_r.get(D).swap();
    let rr = reg.to_reader();
    assert_eq!(
        rr.get(EntryStoreId(2)).get(slots[0].unwrap()).core_read(0),
        300 * 17
    );
    assert_eq!(
        rr.get(EntryStoreId(0)).get(slots[1].unwrap()).core_read(0),
        301 * 17
    );
    assert_eq!(
        rr.get(EntryStoreId(1)).get(slots[2].unwrap()).core_read(0),
        302 * 17
    );
}

// ============ Group 4: Multi-TB SPSC ============

#[test]
fn spsc_single_store_on_default_tb() {
    let mem = create_mem(MEM_SIZE);
    let tb_defs = [tb_def(0, 1024)];
    let tb_reg = TripleBufferWriterRegistry::<1>::new(Arc::clone(&mem), tb_defs, 0, 4096);
    let defs = [sdef(0, D, 8, 0, 16, 4)];
    let reg = EntryStoreWriterRegistry::<1, 1>::new(
        Arc::clone(&mem),
        tb_reg.clone(),
        defs,
        tb_reg.mem_end_offset(),
        0,
        [0; 1],
    );
    let st = reg.get(EntryStoreId(0));
    let slot = st.insert().unwrap();
    st.get(slot).core_write(0, 42);
    st.get(slot).attr_write(0, 99);
    let rr_early = reg.to_reader();
    assert_eq!(rr_early.get(EntryStoreId(0)).get(slot).attr_read(0), 99);
    tb_reg.get(D).publish();
    let tb_rr = tb_reg.to_reader();
    tb_rr.get(D).swap();
    let rr = reg.to_reader();
    assert_eq!(rr.get(EntryStoreId(0)).get(slot).core_read(0), 42);
}

#[test]
fn spsc_single_store_on_user_tb() {
    let mem = create_mem(MEM_SIZE);
    let tb_defs = [tb_def(0, 2048)];
    let tb_reg = TripleBufferWriterRegistry::<1>::new(Arc::clone(&mem), tb_defs, 0, 4096);
    let defs = [sdef(0, TripleBufferId(0), 4, 4, 8, 8)];
    let reg = EntryStoreWriterRegistry::<1, 1>::new(
        Arc::clone(&mem),
        tb_reg.clone(),
        defs,
        tb_reg.mem_end_offset(),
        0,
        [0; 1],
    );
    let st = reg.get(EntryStoreId(0));
    let slot = st.insert().unwrap();
    st.get(slot).core_write(0, 100);
    st.get(slot).meta_write(0, 200);
    tb_reg.get(TripleBufferId(0)).publish();
    let tb_rr = tb_reg.to_reader();
    tb_rr.get(TripleBufferId(0)).swap();
    let rr = reg.to_reader();
    assert_eq!(rr.get(EntryStoreId(0)).get(slot).core_read(0), 100);
    assert_eq!(rr.get(EntryStoreId(0)).get(slot).meta_read(0), 200);
}

#[test]
fn spsc_two_stores_on_different_tbs_independent_publish() {
    let mem = create_mem(MEM_SIZE);
    let tb_defs = [tb_def(0, 2048)];
    let tb_reg = TripleBufferWriterRegistry::<1>::new(Arc::clone(&mem), tb_defs, 0, 4096);
    let defs = [
        sdef(0, D, 8, 4, 16, 4),
        sdef(1, TripleBufferId(0), 4, 0, 8, 8),
    ];
    let reg = EntryStoreWriterRegistry::<1, 2>::new(
        Arc::clone(&mem),
        tb_reg.clone(),
        defs,
        tb_reg.mem_end_offset(),
        0,
        [0; 1],
    );
    let s0 = reg.get(EntryStoreId(0)).insert().unwrap();
    let s1 = reg.get(EntryStoreId(1)).insert().unwrap();
    reg.get(EntryStoreId(0)).get(s0).core_write(0, 7001);
    reg.get(EntryStoreId(1)).get(s1).core_write(0, 8002);
    tb_reg.get(D).publish();
    let tb_rr = tb_reg.to_reader();
    tb_rr.get(D).swap();
    let rr = reg.to_reader();
    assert_eq!(rr.get(EntryStoreId(0)).get(s0).core_read(0), 7001);
    assert_eq!(rr.get(EntryStoreId(1)).get(s1).core_read(0), 0);
    tb_reg.get(TripleBufferId(0)).publish();
    tb_rr.get(TripleBufferId(0)).swap();
    let rr2 = reg.to_reader();
    assert_eq!(rr2.get(EntryStoreId(1)).get(s1).core_read(0), 8002);
}

#[test]
fn spsc_three_stores_on_three_tbs_staggered_publish() {
    let mem = create_mem(MEM_SIZE);
    let tb_defs = [tb_def(0, 2048), tb_def(1, 2048)];
    let tb_reg = TripleBufferWriterRegistry::<2>::new(Arc::clone(&mem), tb_defs, 0, 8192);
    let defs = [
        sdef(0, D, 4, 2, 8, 4),
        sdef(1, TripleBufferId(0), 8, 0, 16, 4),
        sdef(2, TripleBufferId(1), 2, 2, 4, 8),
    ];
    let reg = EntryStoreWriterRegistry::<2, 3>::new(
        Arc::clone(&mem),
        tb_reg.clone(),
        defs,
        tb_reg.mem_end_offset(),
        0,
        [0; 2],
    );
    let s0 = reg.get(EntryStoreId(0)).insert().unwrap();
    let s1 = reg.get(EntryStoreId(1)).insert().unwrap();
    let s2 = reg.get(EntryStoreId(2)).insert().unwrap();
    reg.get(EntryStoreId(0)).get(s0).core_write(0, 10);
    reg.get(EntryStoreId(1)).get(s1).core_write(0, 20);
    reg.get(EntryStoreId(2)).get(s2).core_write(0, 30);
    tb_reg.get(D).publish();
    let tb_rr = tb_reg.to_reader();
    tb_rr.get(D).swap();
    let rr = reg.to_reader();
    assert_eq!(rr.get(EntryStoreId(0)).get(s0).core_read(0), 10);
    assert_eq!(rr.get(EntryStoreId(1)).get(s1).core_read(0), 0);
    assert_eq!(rr.get(EntryStoreId(2)).get(s2).core_read(0), 0);
    tb_reg.get(TripleBufferId(0)).publish();
    tb_rr.get(TripleBufferId(0)).swap();
    let rr2 = reg.to_reader();
    assert_eq!(rr2.get(EntryStoreId(1)).get(s1).core_read(0), 20);
    assert_eq!(rr2.get(EntryStoreId(2)).get(s2).core_read(0), 0);
    tb_reg.get(TripleBufferId(1)).publish();
    tb_rr.get(TripleBufferId(1)).swap();
    let rr3 = reg.to_reader();
    assert_eq!(rr3.get(EntryStoreId(2)).get(s2).core_read(0), 30);
}

#[test]
fn spsc_attr_plane_always_visible_regardless_of_tb() {
    let mem = create_mem(MEM_SIZE);
    let tb_defs = [tb_def(0, 2048)];
    let tb_reg = TripleBufferWriterRegistry::<1>::new(Arc::clone(&mem), tb_defs, 0, 4096);
    let defs = [
        sdef(0, D, 8, 0, 16, 4),
        sdef(1, TripleBufferId(0), 4, 4, 8, 8),
    ];
    let reg = EntryStoreWriterRegistry::<1, 2>::new(
        Arc::clone(&mem),
        tb_reg.clone(),
        defs,
        tb_reg.mem_end_offset(),
        0,
        [0; 1],
    );
    let s0 = reg.get(EntryStoreId(0)).insert().unwrap();
    let s1 = reg.get(EntryStoreId(1)).insert().unwrap();
    reg.get(EntryStoreId(0)).get(s0).attr_write(0, 501);
    reg.get(EntryStoreId(1)).get(s1).attr_write(0, 502);
    let rr = reg.to_reader();
    assert_eq!(rr.get(EntryStoreId(0)).get(s0).attr_read(0), 501);
    assert_eq!(rr.get(EntryStoreId(1)).get(s1).attr_read(0), 502);
}

#[test]
fn spsc_publish_user_tb_does_not_affect_default_tb() {
    let mem = create_mem(MEM_SIZE);
    let tb_defs = [tb_def(0, 2048)];
    let tb_reg = TripleBufferWriterRegistry::<1>::new(Arc::clone(&mem), tb_defs, 0, 4096);
    let defs = [
        sdef(0, D, 8, 4, 16, 4),
        sdef(1, TripleBufferId(0), 4, 0, 8, 8),
    ];
    let reg = EntryStoreWriterRegistry::<1, 2>::new(
        Arc::clone(&mem),
        tb_reg.clone(),
        defs,
        tb_reg.mem_end_offset(),
        0,
        [0; 1],
    );
    let s0 = reg.get(EntryStoreId(0)).insert().unwrap();
    let s1 = reg.get(EntryStoreId(1)).insert().unwrap();
    reg.get(EntryStoreId(0)).get(s0).core_write(0, 601);
    reg.get(EntryStoreId(1)).get(s1).core_write(0, 602);
    tb_reg.get(TripleBufferId(0)).publish();
    let tb_rr = tb_reg.to_reader();
    tb_rr.get(TripleBufferId(0)).swap();
    let rr = reg.to_reader();
    assert_eq!(rr.get(EntryStoreId(1)).get(s1).core_read(0), 602);
    assert_eq!(rr.get(EntryStoreId(0)).get(s0).core_read(0), 0);
}

#[test]
fn spsc_multiple_publish_cycles_same_store() {
    let mem = create_mem(MEM_SIZE);
    let tb_defs = [tb_def(0, 1024)];
    let tb_reg = TripleBufferWriterRegistry::<1>::new(Arc::clone(&mem), tb_defs, 0, 4096);
    let defs = [sdef(0, D, 8, 0, 0, 4)];
    let reg = EntryStoreWriterRegistry::<1, 1>::new(
        Arc::clone(&mem),
        tb_reg.clone(),
        defs,
        tb_reg.mem_end_offset(),
        0,
        [0; 1],
    );
    let st = reg.get(EntryStoreId(0));
    let slot = st.insert().unwrap();
    let tb_rr = tb_reg.to_reader();
    for (k, v) in [(1i32, 1i32), (2, 2), (3, 3)] {
        st.get(slot).core_write(0, k);
        tb_reg.get(D).publish();
        tb_rr.get(D).swap();
        let rr = reg.to_reader();
        assert_eq!(rr.get(EntryStoreId(0)).get(slot).core_read(0), v);
    }
}

#[test]
fn spsc_large_strides_full_data_integrity() {
    let mem = create_mem(MEM_SIZE);
    let tb_defs = [tb_def(0, 2048)];
    let tb_reg = TripleBufferWriterRegistry::<1>::new(Arc::clone(&mem), tb_defs, 0, 2048);
    let defs = [sdef(0, D, 64, 32, 64, 4)];
    let reg = EntryStoreWriterRegistry::<1, 1>::new(
        Arc::clone(&mem),
        tb_reg.clone(),
        defs,
        tb_reg.mem_end_offset(),
        0,
        [0; 1],
    );
    let st = reg.get(EntryStoreId(0));
    let slot = st.insert().unwrap();
    let ew = st.get(slot);
    for i in 0..64 {
        ew.core_write(i, (i * 10) as i32);
    }
    for i in 0..32 {
        ew.meta_write(i, (i * 20) as i32);
    }
    for i in 0..64 {
        ew.attr_write(i, (i * 30) as i32);
    }
    tb_reg.get(D).publish();
    let tb_rr = tb_reg.to_reader();
    tb_rr.get(D).swap();
    let rr = reg.to_reader();
    let er = rr.get(EntryStoreId(0)).get(slot);
    for i in 0..64 {
        assert_eq!(er.core_read(i), (i * 10) as i32);
    }
    for i in 0..32 {
        assert_eq!(er.meta_read(i), (i * 20) as i32);
    }
    for i in 0..64 {
        assert_eq!(er.attr_read(i), (i * 30) as i32);
    }
}

// ============ Group 5: to_reader() structural ============

#[test]
fn to_reader_produces_matching_offsets() {
    let mem = create_mem(MEM_SIZE);
    let tb_defs = [tb_def(0, 1024)];
    let tb_reg = TripleBufferWriterRegistry::<1>::new(Arc::clone(&mem), tb_defs, 0, 4096);
    let defs = [sdef(0, D, 8, 0, 16, 4)];
    let reg = EntryStoreWriterRegistry::<1, 1>::new(
        Arc::clone(&mem),
        tb_reg.clone(),
        defs,
        tb_reg.mem_end_offset(),
        0,
        [0; 1],
    );
    let reader_reg = reg.to_reader();
    assert_eq!(reader_reg.mem_start_offset(), reg.mem_start_offset());
    assert_eq!(reader_reg.mem_end_offset(), reg.mem_end_offset());
    assert_eq!(reader_reg.len(), reg.len());
}

#[test]
fn to_reader_preserves_id_mapping_across_permuted_ids() {
    let mem = create_mem(MEM_SIZE);
    let tb_defs = [tb_def(0, 4096)];
    let tb_reg = TripleBufferWriterRegistry::<1>::new(Arc::clone(&mem), tb_defs, 0, 8192);
    let defs = [
        sdef(2, D, 8, 0, 16, 4),
        sdef(0, D, 4, 2, 8, 8),
        sdef(1, D, 2, 2, 4, 16),
    ];
    let reg = EntryStoreWriterRegistry::<1, 3>::new(
        Arc::clone(&mem),
        tb_reg.clone(),
        defs,
        tb_reg.mem_end_offset(),
        0,
        [0; 1],
    );
    let reader_reg = reg.to_reader();
    for id in [0u16, 1, 2] {
        let eid = EntryStoreId(id);
        assert_eq!(
            reader_reg.get(eid).mem_start_offset(),
            reg.get(eid).mem_start_offset()
        );
    }
}

#[test]
fn to_reader_preserves_id_mapping_with_heterogeneous_strides() {
    let mem = create_mem(MEM_SIZE);
    let tb_defs = [tb_def(0, 8192)];
    let tb_reg = TripleBufferWriterRegistry::<1>::new(Arc::clone(&mem), tb_defs, 0, 8192);
    let defs = [
        sdef(0, D, 2, 0, 4, 16),
        sdef(1, D, 16, 8, 32, 4),
        sdef(2, D, 64, 32, 64, 2),
    ];
    let reg = EntryStoreWriterRegistry::<1, 3>::new(
        Arc::clone(&mem),
        tb_reg.clone(),
        defs,
        tb_reg.mem_end_offset(),
        0,
        [0; 1],
    );
    let reader_reg = reg.to_reader();
    for id in 0u16..3 {
        let eid = EntryStoreId(id);
        assert_eq!(
            reader_reg.get(eid).mem_start_offset(),
            reg.get(eid).mem_start_offset()
        );
    }
}

// ============ Group 6: copy_from ============

#[test]
fn copy_from_same_size_migrates_core_meta_attr() {
    let mem_src = create_mem(MEM_SIZE);
    let mem_dst = create_mem(MEM_SIZE);
    let tb_defs = [tb_def(0, 4096)];
    let tb_src = TripleBufferWriterRegistry::<1>::new(Arc::clone(&mem_src), tb_defs, 0, 8192);
    let tb_dst = TripleBufferWriterRegistry::<1>::new(Arc::clone(&mem_dst), tb_defs, 0, 8192);
    let defs = [sdef(0, D, 8, 4, 16, 4), sdef(1, D, 4, 0, 8, 8)];
    let source = EntryStoreWriterRegistry::<1, 2>::new(
        Arc::clone(&mem_src),
        tb_src.clone(),
        defs,
        tb_src.mem_end_offset(),
        0,
        [0; 1],
    );
    let dest = EntryStoreWriterRegistry::<1, 2>::new(
        Arc::clone(&mem_dst),
        tb_dst.clone(),
        defs,
        tb_dst.mem_end_offset(),
        0,
        [0; 1],
    );
    let s00 = source.get(EntryStoreId(0)).insert().unwrap();
    let s01 = source.get(EntryStoreId(0)).insert().unwrap();
    let s10 = source.get(EntryStoreId(1)).insert().unwrap();
    let s11 = source.get(EntryStoreId(1)).insert().unwrap();
    source.get(EntryStoreId(0)).get(s00).core_write(0, 11);
    source.get(EntryStoreId(0)).get(s00).meta_write(0, 12);
    source.get(EntryStoreId(0)).get(s00).attr_write(0, 13);
    source.get(EntryStoreId(0)).get(s01).core_write(0, 21);
    source.get(EntryStoreId(1)).get(s10).core_write(0, 31);
    source.get(EntryStoreId(1)).get(s10).attr_write(0, 32);
    source.get(EntryStoreId(1)).get(s11).core_write(0, 41);
    dest.copy_from(&source);
    assert_eq!(dest.get(EntryStoreId(0)).get(s00).core_read(0), 11);
    assert_eq!(dest.get(EntryStoreId(0)).get(s00).meta_read(0), 12);
    assert_eq!(dest.get(EntryStoreId(0)).get(s00).attr_read(0), 13);
    assert_eq!(dest.get(EntryStoreId(0)).get(s01).core_read(0), 21);
    assert_eq!(dest.get(EntryStoreId(1)).get(s10).core_read(0), 31);
    assert_eq!(dest.get(EntryStoreId(1)).get(s10).attr_read(0), 32);
    assert_eq!(dest.get(EntryStoreId(1)).get(s11).core_read(0), 41);
}

#[test]
fn copy_from_smaller_to_larger_registry() {
    let mem_src = create_mem(MEM_SIZE);
    let mem_dst = create_mem(MEM_SIZE);
    let tb_defs = [tb_def(0, 2048)];
    let tb_src = TripleBufferWriterRegistry::<1>::new(Arc::clone(&mem_src), tb_defs, 0, 4096);
    let tb_dst = TripleBufferWriterRegistry::<1>::new(Arc::clone(&mem_dst), tb_defs, 0, 4096);
    let defs_src = [sdef(0, D, 8, 0, 16, 4)];
    let defs_dst = [sdef(0, D, 8, 0, 16, 4), sdef(1, D, 4, 4, 8, 8)];
    let source = EntryStoreWriterRegistry::<1, 1>::new(
        Arc::clone(&mem_src),
        tb_src.clone(),
        defs_src,
        tb_src.mem_end_offset(),
        0,
        [0; 1],
    );
    let dest = EntryStoreWriterRegistry::<1, 2>::new(
        Arc::clone(&mem_dst),
        tb_dst.clone(),
        defs_dst,
        tb_dst.mem_end_offset(),
        0,
        [0; 1],
    );
    let slot = source.get(EntryStoreId(0)).insert().unwrap();
    source.get(EntryStoreId(0)).get(slot).core_write(0, 555);
    dest.copy_from(&source);
    assert_eq!(dest.get(EntryStoreId(0)).get(slot).core_read(0), 555);
    assert_eq!(dest.get(EntryStoreId(1)).len(), 0);
}

#[test]
fn copy_from_preserves_id_mapping_with_permuted_defs() {
    let mem_src = create_mem(MEM_SIZE);
    let mem_dst = create_mem(MEM_SIZE);
    let tb_defs = [tb_def(0, 2048)];
    let tb_src = TripleBufferWriterRegistry::<1>::new(Arc::clone(&mem_src), tb_defs, 0, 4096);
    let tb_dst = TripleBufferWriterRegistry::<1>::new(Arc::clone(&mem_dst), tb_defs, 0, 4096);
    let defs_src = [sdef(1, D, 8, 0, 16, 4), sdef(0, D, 4, 4, 8, 8)];
    let defs_dst = [sdef(0, D, 4, 4, 8, 8), sdef(1, D, 8, 0, 16, 4)];
    let source = EntryStoreWriterRegistry::<1, 2>::new(
        Arc::clone(&mem_src),
        tb_src.clone(),
        defs_src,
        tb_src.mem_end_offset(),
        0,
        [0; 1],
    );
    let dest = EntryStoreWriterRegistry::<1, 2>::new(
        Arc::clone(&mem_dst),
        tb_dst.clone(),
        defs_dst,
        tb_dst.mem_end_offset(),
        0,
        [0; 1],
    );
    let slot = source.get(EntryStoreId(0)).insert().unwrap();
    source.get(EntryStoreId(0)).get(slot).core_write(0, 4242);
    dest.copy_from(&source);
    assert_eq!(dest.get(EntryStoreId(0)).get(slot).core_read(0), 4242);
}

#[test]
fn copy_from_with_heterogeneous_strides() {
    let mem_src = create_mem(MEM_SIZE);
    let mem_dst = create_mem(MEM_SIZE);
    let tb_defs = [tb_def(0, 8192)];
    let tb_src = TripleBufferWriterRegistry::<1>::new(Arc::clone(&mem_src), tb_defs, 0, 16384);
    let tb_dst = TripleBufferWriterRegistry::<1>::new(Arc::clone(&mem_dst), tb_defs, 0, 16384);
    let defs = [sdef(0, D, 4, 2, 8, 4), sdef(1, D, 8, 0, 16, 8)];
    let source = EntryStoreWriterRegistry::<1, 2>::new(
        Arc::clone(&mem_src),
        tb_src.clone(),
        defs,
        tb_src.mem_end_offset(),
        0,
        [0; 1],
    );
    let dest = EntryStoreWriterRegistry::<1, 2>::new(
        Arc::clone(&mem_dst),
        tb_dst.clone(),
        defs,
        tb_dst.mem_end_offset(),
        0,
        [0; 1],
    );
    let mut slots0: [Option<SlotId>; 4] = [None; 4];
    let mut slots1: [Option<SlotId>; 8] = [None; 8];
    for i in 0..4 {
        slots0[i] = Some(source.get(EntryStoreId(0)).insert().unwrap());
        source
            .get(EntryStoreId(0))
            .get(slots0[i].unwrap())
            .core_write(0, i as i32);
    }
    for i in 0..8 {
        slots1[i] = Some(source.get(EntryStoreId(1)).insert().unwrap());
        source
            .get(EntryStoreId(1))
            .get(slots1[i].unwrap())
            .core_write(0, (100 + i) as i32);
    }
    dest.copy_from(&source);
    for i in 0..4 {
        assert_eq!(
            dest.get(EntryStoreId(0))
                .get(slots0[i].unwrap())
                .core_read(0),
            i as i32
        );
    }
    for i in 0..8 {
        assert_eq!(
            dest.get(EntryStoreId(1))
                .get(slots1[i].unwrap())
                .core_read(0),
            (100 + i) as i32
        );
    }
}

#[test]
fn copy_from_with_different_tb_assignments() {
    let mem_src = create_mem(MEM_SIZE);
    let mem_dst = create_mem(MEM_SIZE);
    let tb_defs = [tb_def(0, 4096)];
    let tb_src = TripleBufferWriterRegistry::<1>::new(Arc::clone(&mem_src), tb_defs, 0, 8192);
    let tb_dst = TripleBufferWriterRegistry::<1>::new(Arc::clone(&mem_dst), tb_defs, 0, 8192);
    let defs = [
        sdef(0, D, 8, 0, 16, 4),
        sdef(1, TripleBufferId(0), 4, 4, 8, 8),
    ];
    let source = EntryStoreWriterRegistry::<1, 2>::new(
        Arc::clone(&mem_src),
        tb_src.clone(),
        defs,
        tb_src.mem_end_offset(),
        0,
        [0; 1],
    );
    let dest = EntryStoreWriterRegistry::<1, 2>::new(
        Arc::clone(&mem_dst),
        tb_dst.clone(),
        defs,
        tb_dst.mem_end_offset(),
        0,
        [0; 1],
    );
    let s0 = source.get(EntryStoreId(0)).insert().unwrap();
    let s1 = source.get(EntryStoreId(1)).insert().unwrap();
    source.get(EntryStoreId(0)).get(s0).core_write(0, 77);
    source.get(EntryStoreId(1)).get(s1).core_write(0, 88);
    dest.copy_from(&source);
    assert_eq!(dest.get(EntryStoreId(0)).get(s0).core_read(0), 77);
    assert_eq!(dest.get(EntryStoreId(1)).get(s1).core_read(0), 88);
}

// ============ Group 7: Size calculations ============

#[test]
fn calculate_size_on_mem_single_store() {
    let defs = [sdef(0, D, 8, 0, 16, 4)];
    assert_eq!(
        EntryStoreWriterRegistry::<1, 1>::calculate_size_on_mem(&defs),
        defs[0].size_on_mem()
    );
}

#[test]
fn calculate_size_on_mem_sums_heterogeneous_stores() {
    let defs = [
        sdef(0, D, 4, 2, 8, 4),
        sdef(1, D, 16, 8, 32, 8),
        sdef(2, D, 2, 0, 4, 16),
    ];
    let sum = defs[0].size_on_mem() + defs[1].size_on_mem() + defs[2].size_on_mem();
    assert_eq!(
        EntryStoreWriterRegistry::<1, 3>::calculate_size_on_mem(&defs),
        sum
    );
}

#[test]
fn calculate_size_on_tb_filters_by_tb_id() {
    let defs = [
        sdef(0, D, 8, 0, 16, 4),
        sdef(1, D, 4, 4, 8, 8),
        sdef(2, TripleBufferId(0), 2, 2, 4, 8),
    ];
    assert_eq!(
        EntryStoreWriterRegistry::<1, 3>::calculate_size_on_default_tb(&defs),
        defs[0].size_on_tb() + defs[1].size_on_tb()
    );
    assert_eq!(
        EntryStoreWriterRegistry::<1, 3>::calculate_size_on_tb_for(TripleBufferId(0), &defs),
        defs[2].size_on_tb()
    );
    assert_eq!(
        EntryStoreWriterRegistry::<1, 3>::calculate_size_on_tb_for(TripleBufferId(1), &defs),
        0
    );
}

// ============ Group 8: Debug panics ============

#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "duplicate id")]
fn duplicate_ids_panic() {
    let mem = create_mem(MEM_SIZE);
    let tb_defs = [tb_def(0, 1024)];
    let tb_reg = TripleBufferWriterRegistry::<1>::new(Arc::clone(&mem), tb_defs, 0, 4096);
    let defs = [sdef(0, D, 8, 0, 16, 4), sdef(0, D, 4, 4, 8, 8)];
    let _ = EntryStoreWriterRegistry::<1, 2>::new(
        Arc::clone(&mem),
        tb_reg.clone(),
        defs,
        tb_reg.mem_end_offset(),
        0,
        [0; 1],
    );
}

#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "out of bounds")]
fn out_of_range_id_panics_at_construction() {
    let mem = create_mem(MEM_SIZE);
    let tb_defs = [tb_def(0, 1024)];
    let tb_reg = TripleBufferWriterRegistry::<1>::new(Arc::clone(&mem), tb_defs, 0, 4096);
    let defs = [sdef(0, D, 8, 0, 16, 4), sdef(5, D, 4, 4, 8, 8)];
    let _ = EntryStoreWriterRegistry::<1, 2>::new(
        Arc::clone(&mem),
        tb_reg.clone(),
        defs,
        tb_reg.mem_end_offset(),
        0,
        [0; 1],
    );
}

#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "out of bounds")]
fn get_out_of_range_id_panics() {
    let mem = create_mem(MEM_SIZE);
    let tb_defs = [tb_def(0, 1024)];
    let tb_reg = TripleBufferWriterRegistry::<1>::new(Arc::clone(&mem), tb_defs, 0, 4096);
    let defs = [sdef(0, D, 8, 0, 16, 4)];
    let reg = EntryStoreWriterRegistry::<1, 1>::new(
        Arc::clone(&mem),
        tb_reg.clone(),
        defs,
        tb_reg.mem_end_offset(),
        0,
        [0; 1],
    );
    let _ = reg.get(EntryStoreId(1));
}

#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "out of Triple Buffer")]
fn default_tb_overflow_panics() {
    let mem = create_mem(MEM_SIZE * 4);
    let tb_defs = [tb_def(0, 4096)];
    let tb_reg = TripleBufferWriterRegistry::<1>::new(Arc::clone(&mem), tb_defs, 0, 1024);
    let defs = [sdef(0, D, 512, 0, 16, 128)];
    let _ = EntryStoreWriterRegistry::<1, 1>::new(
        Arc::clone(&mem),
        tb_reg.clone(),
        defs,
        tb_reg.mem_end_offset(),
        0,
        [0; 1],
    );
}

// ============ Group 9: Stress ============

#[test]
fn stress_fill_all_stores_to_capacity_then_verify() {
    let mem = create_mem(MEM_SIZE * 4);
    let tb_defs = [tb_def(0, 32768), tb_def(1, 32768)];
    let tb_reg = TripleBufferWriterRegistry::<2>::new(Arc::clone(&mem), tb_defs, 0, 65536);
    let defs = [
        sdef(0, D, 4, 2, 8, 32),
        sdef(1, D, 8, 0, 16, 16),
        sdef(2, TripleBufferId(0), 2, 2, 4, 64),
        sdef(3, TripleBufferId(1), 16, 8, 32, 8),
    ];
    let reg = EntryStoreWriterRegistry::<2, 4>::new(
        Arc::clone(&mem),
        tb_reg.clone(),
        defs,
        tb_reg.mem_end_offset(),
        0,
        [0; 2],
    );
    let mut slots: [[Option<SlotId>; 128]; 4] = [[None; 128]; 4];
    let caps = [32usize, 16, 64, 8];
    for si in 0..4usize {
        let id = EntryStoreId(si as u16);
        for j in 0..caps[si] {
            slots[si][j] = Some(reg.get(id).insert().unwrap());
            let v = (si * 100000 + j * 17 + 3) as i32;
            reg.get(id).get(slots[si][j].unwrap()).core_write(0, v);
            reg.get(id)
                .get(slots[si][j].unwrap())
                .attr_write(0, v + 9000);
        }
    }
    tb_reg.get(D).publish();
    tb_reg.get(TripleBufferId(0)).publish();
    tb_reg.get(TripleBufferId(1)).publish();
    let tb_rr = tb_reg.to_reader();
    tb_rr.get(D).swap();
    tb_rr.get(TripleBufferId(0)).swap();
    tb_rr.get(TripleBufferId(1)).swap();
    let rr = reg.to_reader();
    for si in 0..4usize {
        let id = EntryStoreId(si as u16);
        for j in 0..caps[si] {
            let v = (si * 100000 + j * 17 + 3) as i32;
            let s = slots[si][j].unwrap();
            assert_eq!(rr.get(id).get(s).core_read(0), v);
            assert_eq!(rr.get(id).get(s).attr_read(0), v + 9000);
        }
    }
}

#[test]
fn stress_interleaved_insert_write_across_stores() {
    let mem = create_mem(MEM_SIZE);
    let tb_defs = [tb_def(0, 4096)];
    let tb_reg = TripleBufferWriterRegistry::<1>::new(Arc::clone(&mem), tb_defs, 0, 8192);
    let defs = [
        sdef(0, D, 4, 2, 8, 8),
        sdef(1, D, 8, 0, 16, 8),
        sdef(2, D, 2, 2, 4, 8),
    ];
    let reg = EntryStoreWriterRegistry::<1, 3>::new(
        Arc::clone(&mem),
        tb_reg.clone(),
        defs,
        tb_reg.mem_end_offset(),
        0,
        [0; 1],
    );
    let mut slots: [[Option<SlotId>; 8]; 3] = [[None; 8]; 3];
    for round in 0..8 {
        slots[0][round] = Some(reg.get(EntryStoreId(0)).insert().unwrap());
        slots[1][round] = Some(reg.get(EntryStoreId(1)).insert().unwrap());
        slots[2][round] = Some(reg.get(EntryStoreId(2)).insert().unwrap());
    }
    for round in 0..8 {
        reg.get(EntryStoreId(0))
            .get(slots[0][round].unwrap())
            .core_write(0, (round * 10 + 1) as i32);
        reg.get(EntryStoreId(1))
            .get(slots[1][round].unwrap())
            .core_write(0, (round * 10 + 2) as i32);
        reg.get(EntryStoreId(2))
            .get(slots[2][round].unwrap())
            .core_write(0, (round * 10 + 3) as i32);
    }
    tb_reg.get(D).publish();
    let tb_rr = tb_reg.to_reader();
    tb_rr.get(D).swap();
    let rr = reg.to_reader();
    for round in 0..8 {
        assert_eq!(
            rr.get(EntryStoreId(0))
                .get(slots[0][round].unwrap())
                .core_read(0),
            (round * 10 + 1) as i32
        );
        assert_eq!(
            rr.get(EntryStoreId(1))
                .get(slots[1][round].unwrap())
                .core_read(0),
            (round * 10 + 2) as i32
        );
        assert_eq!(
            rr.get(EntryStoreId(2))
                .get(slots[2][round].unwrap())
                .core_read(0),
            (round * 10 + 3) as i32
        );
    }
}

#[test]
fn stress_remove_reuse_across_stores() {
    let mem = create_mem(MEM_SIZE);
    let tb_defs = [tb_def(0, 4096)];
    let tb_reg = TripleBufferWriterRegistry::<1>::new(Arc::clone(&mem), tb_defs, 0, 8192);
    let defs = [sdef(0, D, 8, 0, 16, 4), sdef(1, D, 4, 4, 8, 4)];
    let reg = EntryStoreWriterRegistry::<1, 2>::new(
        Arc::clone(&mem),
        tb_reg.clone(),
        defs,
        tb_reg.mem_end_offset(),
        0,
        [0; 1],
    );
    let st0 = reg.get(EntryStoreId(0));
    let st1 = reg.get(EntryStoreId(1));
    let mut slots: [Option<SlotId>; 4] = [None; 4];
    for i in 0..4 {
        slots[i] = Some(st0.insert().unwrap());
        st0.get(slots[i].unwrap()).core_write(0, (100 + i) as i32);
        st0.get(slots[i].unwrap()).attr_write(0, (200 + i) as i32);
    }
    let s1 = st1.insert().unwrap();
    st1.get(s1).core_write(0, 999);
    st1.get(s1).attr_write(0, 888);
    let removed = slots[2].unwrap();
    st0.remove(removed).unwrap();
    let reader_ack = st0.to_reader();
    st0.publish();
    reader_ack.ack_generation();
    st0.publish();
    let s_new = st0.insert().unwrap();
    assert_eq!(s_new, removed);
    st0.get(s_new).core_write(0, 7777);
    st0.get(s_new).attr_write(0, 6666);
    tb_reg.get(D).publish();
    let tb_rr = tb_reg.to_reader();
    tb_rr.get(D).swap();
    let rr = reg.to_reader();
    assert_eq!(rr.get(EntryStoreId(0)).get(s_new).core_read(0), 7777);
    assert_eq!(rr.get(EntryStoreId(0)).get(s_new).attr_read(0), 6666);
    assert_eq!(rr.get(EntryStoreId(1)).get(s1).core_read(0), 999);
    assert_eq!(rr.get(EntryStoreId(1)).get(s1).attr_read(0), 888);
}
