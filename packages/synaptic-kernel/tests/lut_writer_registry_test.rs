//! Tests for [`LutWriterRegistry`] layout, `get`, `to_reader`, and `copy_from`.

use std::sync::atomic::AtomicI32;
use std::sync::Arc;
use synaptic_kernel::primitives::entry_store_config::EntryStoreConfig;
use synaptic_kernel::primitives::entry_store_def::{EntryStoreDef, EntryStoreId};
use synaptic_kernel::primitives::entry_store_writer_registry::EntryStoreWriterRegistry;
use synaptic_kernel::primitives::lut_def::{LutDef, LutId};
use synaptic_kernel::primitives::lut_writer_registry::LutWriterRegistry;
use synaptic_kernel::primitives::triple_buffer_def::{TripleBufferDef, TripleBufferId};
use synaptic_kernel::primitives::triple_buffer_writer_registry::TripleBufferWriterRegistry;
use synaptic_kernel::primitives::types::AtomicBuffer;

const MEM_SIZE: usize = 131072;
const D: TripleBufferId = TripleBufferId::DEFAULT;

fn create_mem(size: usize) -> AtomicBuffer {
    (0..size).map(|_| AtomicI32::new(0)).collect()
}

fn tb_def(id: u16, cap: usize) -> TripleBufferDef {
    TripleBufferDef {
        id: TripleBufferId(id),
        buffer_capacity: cap,
    }
}

fn ldef(id: u16, tb_id: TripleBufferId, size: usize) -> LutDef {
    LutDef::new(LutId(id), tb_id, size)
}

#[test]
fn single_lut_construction() {
    let mem = create_mem(MEM_SIZE);
    let tb_defs = [tb_def(0, 1024)];
    let tb_reg = TripleBufferWriterRegistry::<1>::new(Arc::clone(&mem), tb_defs, 0, 4096);
    let defs = [ldef(0, D, 16)];
    let reg = LutWriterRegistry::<1, 1>::new(tb_reg, defs, 0, [0; 1]);
    assert_eq!(reg.get(LutId(0)).len(), 16);
}

#[test]
fn multiple_luts_non_overlapping_on_default_tb() {
    let mem = create_mem(MEM_SIZE);
    let tb_defs = [tb_def(0, 2048)];
    let tb_reg = TripleBufferWriterRegistry::<1>::new(Arc::clone(&mem), tb_defs, 0, 4096);
    let defs = [ldef(0, D, 5), ldef(1, D, 7)];
    let reg = LutWriterRegistry::<1, 2>::new(tb_reg, defs, 0, [0; 1]);
    let a = reg.get(LutId(0));
    let b = reg.get(LutId(1));
    assert_eq!(b.tb_start_offset(), a.tb_end_offset());
    assert!(a.tb_end_offset() <= b.tb_end_offset());
}

#[test]
fn heterogeneous_sizes_layout() {
    let mem = create_mem(MEM_SIZE);
    let tb_defs = [tb_def(0, 4096)];
    let tb_reg = TripleBufferWriterRegistry::<1>::new(Arc::clone(&mem), tb_defs, 0, 8192);
    let defs = [ldef(0, D, 3), ldef(1, D, 100), ldef(2, D, 1)];
    let reg = LutWriterRegistry::<1, 3>::new(tb_reg, defs, 10, [0; 1]);
    let s0 = reg.get(LutId(0)).tb_start_offset();
    let s1 = reg.get(LutId(1)).tb_start_offset();
    let s2 = reg.get(LutId(2)).tb_start_offset();
    assert_eq!(s0, 10);
    assert_eq!(s1, s0 + 3);
    assert_eq!(s2, s1 + 100);
}

#[test]
fn id_permutation_maps_get_correctly() {
    let mem = create_mem(MEM_SIZE);
    let tb_defs = [tb_def(0, 2048)];
    let tb_reg = TripleBufferWriterRegistry::<1>::new(Arc::clone(&mem), tb_defs, 0, 4096);
    let defs = [ldef(1, D, 4), ldef(0, D, 6)];
    let reg = LutWriterRegistry::<1, 2>::new(tb_reg, defs, 0, [0; 1]);
    reg.get(LutId(0)).write(0, 111);
    reg.get(LutId(1)).write(0, 222);
    assert_eq!(reg.get(LutId(0)).read(0), 111);
    assert_eq!(reg.get(LutId(1)).read(0), 222);
    assert_eq!(reg.get(LutId(0)).tb_start_offset(), 4);
    assert_eq!(reg.get(LutId(1)).tb_start_offset(), 0);
}

#[test]
fn lut_on_default_tb_writes_visible_after_publish_swap() {
    let mem = create_mem(MEM_SIZE);
    let tb_defs = [tb_def(0, 1024)];
    let tb_reg = TripleBufferWriterRegistry::<1>::new(Arc::clone(&mem), tb_defs, 0, 4096);
    let defs = [ldef(0, D, 4)];
    let reg = LutWriterRegistry::<1, 1>::new(tb_reg.clone(), defs, 0, [0; 1]);
    reg.get(LutId(0)).write_all(&[5, 6, 7, 8]);
    let w = tb_reg.get(D).clone();
    w.publish();
    let r = w.to_reader();
    assert!(r.swap());
    let read_reg = reg.to_reader();
    assert_eq!(read_reg.get(LutId(0)).read(0), 5);
    assert_eq!(read_reg.get(LutId(0)).read(3), 8);
}

#[test]
fn lut_on_user_tb_writes_visible_after_publish_tb_swap() {
    let mem = create_mem(MEM_SIZE);
    let tb_defs = [tb_def(0, 2048)];
    let tb_reg = TripleBufferWriterRegistry::<1>::new(Arc::clone(&mem), tb_defs, 0, 4096);
    let defs = [ldef(0, TripleBufferId(0), 8)];
    let reg = LutWriterRegistry::<1, 1>::new(tb_reg.clone(), defs, 0, [0; 1]);
    reg.get(LutId(0)).write(3, 4242);
    let uw = tb_reg.get(TripleBufferId(0)).clone();
    uw.publish();
    let ur = uw.to_reader();
    assert!(ur.swap());
    let read_reg = reg.to_reader();
    assert_eq!(read_reg.get(LutId(0)).read(3), 4242);
}

#[test]
fn mixed_tb_assignment_default_and_user() {
    let mem = create_mem(MEM_SIZE);
    let tb_defs = [tb_def(0, 2048)];
    let tb_reg = TripleBufferWriterRegistry::<1>::new(Arc::clone(&mem), tb_defs, 0, 4096);
    let defs = [ldef(0, D, 3), ldef(1, TripleBufferId(0), 5)];
    let reg = LutWriterRegistry::<1, 2>::new(tb_reg.clone(), defs, 0, [0; 1]);
    let on_default = reg.get(LutId(0));
    let on_user = reg.get(LutId(1));
    assert_eq!(on_default.tb_start_offset(), 0);
    assert_eq!(on_user.tb_start_offset(), 0);
    on_default.write(0, 1);
    on_user.write(0, 2);
    assert_eq!(on_default.read(0), 1);
    assert_eq!(on_user.read(0), 2);
}

#[test]
fn calculate_size_on_default_tb_matches_runtime_layout() {
    let mem = create_mem(MEM_SIZE);
    let tb_defs = [tb_def(0, 4096)];
    let tb_reg = TripleBufferWriterRegistry::<1>::new(Arc::clone(&mem), tb_defs, 0, 8192);
    let defs = [ldef(0, D, 11), ldef(1, D, 22)];
    let start = 7usize;
    let reg = LutWriterRegistry::<1, 2>::new(tb_reg, defs, start, [0; 1]);
    let calc = LutWriterRegistry::<1, 2>::calculate_size_on_default_tb(&defs);
    assert_eq!(calc, 11 + 22);
    assert_eq!(
        reg.default_tb_end_offset() - reg.default_tb_start_offset(),
        calc
    );
}

#[test]
fn calculate_size_on_tb_for_user_tb() {
    let defs = [ldef(0, D, 3), ldef(1, TripleBufferId(0), 9)];
    assert_eq!(
        LutWriterRegistry::<1, 2>::calculate_size_on_tb_for(TripleBufferId(0), &defs),
        9
    );
    assert_eq!(
        LutWriterRegistry::<1, 2>::calculate_size_on_tb_for(D, &defs),
        3
    );
}

#[test]
fn default_tb_start_end_offsets_match_construction() {
    let mem = create_mem(MEM_SIZE);
    let tb_defs = [tb_def(0, 1024)];
    let tb_reg = TripleBufferWriterRegistry::<1>::new(Arc::clone(&mem), tb_defs, 0, 4096);
    let defs = [ldef(0, D, 4), ldef(1, D, 6)];
    let start = 50usize;
    let reg = LutWriterRegistry::<1, 2>::new(tb_reg, defs, start, [0; 1]);
    assert_eq!(reg.default_tb_start_offset(), start);
    assert_eq!(reg.default_tb_end_offset(), start + 4 + 6);
}

#[test]
fn extra_tb_start_end_offsets_match_construction() {
    let mem = create_mem(MEM_SIZE);
    let tb_defs = [tb_def(0, 1024)];
    let tb_reg = TripleBufferWriterRegistry::<1>::new(Arc::clone(&mem), tb_defs, 0, 4096);
    let defs = [
        ldef(0, TripleBufferId(0), 10),
        ldef(1, TripleBufferId(0), 20),
    ];
    let extra = [15usize; 1];
    let reg = LutWriterRegistry::<1, 2>::new(tb_reg, defs, 0, extra);
    assert_eq!(reg.extra_tb_start_offsets(), [15]);
    assert_eq!(reg.extra_tb_end_offsets(), [15 + 10 + 20]);
}

#[test]
fn to_reader_produces_valid_registry() {
    let mem = create_mem(MEM_SIZE);
    let tb_defs = [tb_def(0, 1024)];
    let tb_reg = TripleBufferWriterRegistry::<1>::new(Arc::clone(&mem), tb_defs, 0, 4096);
    let defs = [ldef(0, D, 2)];
    let reg = LutWriterRegistry::<1, 1>::new(tb_reg.clone(), defs, 0, [0; 1]);
    reg.get(LutId(0)).write(1, -5);
    let w = tb_reg.get(D).clone();
    w.publish();
    assert!(w.to_reader().swap());
    let rr = reg.to_reader();
    assert_eq!(rr.get(LutId(0)).read(1), -5);
}

#[test]
fn copy_from_preserves_all_luts() {
    let mem = create_mem(MEM_SIZE);
    let tb_defs = [tb_def(0, 2048)];
    let tb_reg = TripleBufferWriterRegistry::<1>::new(Arc::clone(&mem), tb_defs, 0, 4096);
    let defs = [ldef(0, D, 3), ldef(1, D, 2)];
    let src = LutWriterRegistry::<1, 2>::new(tb_reg.clone(), defs, 0, [0; 1]);
    src.get(LutId(0)).write_all(&[1, 2, 3]);
    src.get(LutId(1)).write_all(&[40, 41]);
    let dst_defs = [ldef(0, D, 3), ldef(1, D, 2)];
    let dst = LutWriterRegistry::<1, 2>::new(tb_reg, dst_defs, 100, [0; 1]);
    dst.copy_from(&src);
    assert_eq!(dst.get(LutId(0)).read(0), 1);
    assert_eq!(dst.get(LutId(0)).read(2), 3);
    assert_eq!(dst.get(LutId(1)).read(1), 41);
}

#[test]
fn copy_from_larger_dest_copies_source_prefix_only() {
    let mem = create_mem(MEM_SIZE);
    let tb_defs = [tb_def(0, 4096)];
    let tb_reg = TripleBufferWriterRegistry::<1>::new(Arc::clone(&mem), tb_defs, 0, 8192);
    let src_defs = [ldef(0, D, 2), ldef(1, D, 3)];
    let src = LutWriterRegistry::<1, 2>::new(tb_reg.clone(), src_defs, 0, [0; 1]);
    src.get(LutId(0)).write_all(&[10, 20]);
    src.get(LutId(1)).write_all(&[100, 200, 300]);
    let dst_defs = [ldef(0, D, 8), ldef(1, D, 8)];
    let dst = LutWriterRegistry::<1, 2>::new(tb_reg, dst_defs, 200, [0; 1]);
    for i in 2..8 {
        dst.get(LutId(0)).write(i, -1);
    }
    for i in 3..8 {
        dst.get(LutId(1)).write(i, -2);
    }
    dst.copy_from(&src);
    assert_eq!(dst.get(LutId(0)).read(0), 10);
    assert_eq!(dst.get(LutId(0)).read(1), 20);
    for i in 2..8 {
        assert_eq!(dst.get(LutId(0)).read(i), -1);
    }
    assert_eq!(dst.get(LutId(1)).read(0), 100);
    assert_eq!(dst.get(LutId(1)).read(2), 300);
    for i in 3..8 {
        assert_eq!(dst.get(LutId(1)).read(i), -2);
    }
}

#[test]
fn write_read_through_registry() {
    let mem = create_mem(MEM_SIZE);
    let tb_defs = [tb_def(0, 512)];
    let tb_reg = TripleBufferWriterRegistry::<1>::new(Arc::clone(&mem), tb_defs, 0, 1024);
    let defs = [ldef(0, D, 8)];
    let reg = LutWriterRegistry::<1, 1>::new(tb_reg, defs, 0, [0; 1]);
    reg.get(LutId(0)).write(5, 999);
    assert_eq!(reg.get(LutId(0)).read(5), 999);
}

#[test]
fn write_all_read_all_through_registry() {
    let mem = create_mem(MEM_SIZE);
    let tb_defs = [tb_def(0, 512)];
    let tb_reg = TripleBufferWriterRegistry::<1>::new(Arc::clone(&mem), tb_defs, 0, 1024);
    let defs = [ldef(0, D, 5)];
    let reg = LutWriterRegistry::<1, 1>::new(tb_reg, defs, 0, [0; 1]);
    reg.get(LutId(0)).write_all(&[9, 8, 7, 6, 5]);
    let mut buf = [0i32; 5];
    reg.get(LutId(0)).read_all(&mut buf);
    assert_eq!(buf, [9, 8, 7, 6, 5]);
}

#[test]
fn luts_after_entry_stores_on_same_tb_no_corruption() {
    let mem = create_mem(MEM_SIZE);
    let tb_defs = [tb_def(0, 4096)];
    let tb_reg = TripleBufferWriterRegistry::<1>::new(Arc::clone(&mem), tb_defs, 0, 8192);
    let store_defs = [EntryStoreDef::new(
        EntryStoreId(0),
        D,
        EntryStoreConfig {
            core_stride: 4,
            meta_stride: 0,
            attr_stride: 8,
            capacity: 2,
        },
    )];
    let store_reg = EntryStoreWriterRegistry::<1, 1>::new(
        Arc::clone(&mem),
        tb_reg.clone(),
        store_defs,
        tb_reg.mem_end_offset(),
        0,
        [0; 1],
    );
    let lut_defs = [ldef(0, D, 16)];
    let lut_reg = LutWriterRegistry::<1, 1>::new(
        tb_reg.clone(),
        lut_defs,
        store_reg.default_tb_end_offset(),
        store_reg.extra_tb_end_offsets(),
    );
    assert_eq!(
        store_reg.default_tb_end_offset(),
        lut_reg.default_tb_start_offset()
    );
    let slot = store_reg.get(EntryStoreId(0)).insert().unwrap();
    store_reg
        .get(EntryStoreId(0))
        .get(slot)
        .core_write(0, 0x5a5a);
    lut_reg
        .get(LutId(0))
        .write_all(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]);
    assert_eq!(
        store_reg.get(EntryStoreId(0)).get(slot).core_read(0),
        0x5a5a
    );
    for i in 0..16 {
        assert_eq!(lut_reg.get(LutId(0)).read(i), (i + 1) as i32);
    }
}

#[test]
fn chained_tb_offsets_store_end_equals_lut_start() {
    let mem = create_mem(MEM_SIZE);
    let tb_defs = [tb_def(0, 2048)];
    let tb_reg = TripleBufferWriterRegistry::<1>::new(Arc::clone(&mem), tb_defs, 0, 4096);
    let store_defs = [EntryStoreDef::new(
        EntryStoreId(0),
        D,
        EntryStoreConfig {
            core_stride: 8,
            meta_stride: 0,
            attr_stride: 16,
            capacity: 4,
        },
    )];
    let store_reg = EntryStoreWriterRegistry::<1, 1>::new(
        Arc::clone(&mem),
        tb_reg.clone(),
        store_defs,
        tb_reg.mem_end_offset(),
        0,
        [0; 1],
    );
    let lut_defs = [ldef(0, D, 32)];
    let lut_reg = LutWriterRegistry::<1, 1>::new(
        tb_reg,
        lut_defs,
        store_reg.default_tb_end_offset(),
        store_reg.extra_tb_end_offsets(),
    );
    assert_eq!(
        store_reg.default_tb_end_offset(),
        lut_reg.default_tb_start_offset()
    );
}

#[cfg(debug_assertions)]
mod debug_checks {
    use super::*;

    #[test]
    #[should_panic]
    fn get_out_of_bounds_panics() {
        let mem = create_mem(MEM_SIZE);
        let tb_defs = [tb_def(0, 512)];
        let tb_reg = TripleBufferWriterRegistry::<1>::new(Arc::clone(&mem), tb_defs, 0, 1024);
        let defs = [ldef(0, D, 2), ldef(1, D, 2)];
        let reg = LutWriterRegistry::<1, 2>::new(tb_reg, defs, 0, [0; 1]);
        let _ = reg.get(LutId(2));
    }

    #[test]
    #[should_panic]
    fn duplicate_id_panics_at_construction() {
        let mem = create_mem(MEM_SIZE);
        let tb_defs = [tb_def(0, 512)];
        let tb_reg = TripleBufferWriterRegistry::<1>::new(Arc::clone(&mem), tb_defs, 0, 1024);
        let defs = [ldef(0, D, 2), ldef(0, D, 2)];
        let _reg = LutWriterRegistry::<1, 2>::new(tb_reg, defs, 0, [0; 1]);
    }
}
