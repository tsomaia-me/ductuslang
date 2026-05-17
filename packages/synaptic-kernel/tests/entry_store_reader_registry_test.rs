use std::sync::atomic::AtomicI32;
use std::sync::Arc;
use synaptic_kernel::primitives::entry_store_config::EntryStoreConfig;
use synaptic_kernel::primitives::entry_store_def::{EntryStoreDef, EntryStoreId};
use synaptic_kernel::primitives::entry_store_reader_registry::EntryStoreReaderRegistry;
#[allow(unused_imports)]
use synaptic_kernel::primitives::entry_store_writer::EntryStoreWriter;
use synaptic_kernel::primitives::entry_store_writer_registry::EntryStoreWriterRegistry;
use synaptic_kernel::primitives::triple_buffer_def::{TripleBufferDef, TripleBufferId};
use synaptic_kernel::primitives::triple_buffer_writer_registry::TripleBufferWriterRegistry;
use synaptic_kernel::primitives::types::AtomicBuffer;

const MEM_SIZE: usize = 65536;

fn create_mem(size: usize) -> AtomicBuffer {
    (0..size).map(|_| AtomicI32::new(0)).collect()
}

fn sdef(id: u16, core: usize, meta: usize, attr: usize, cap: u32) -> EntryStoreDef {
    EntryStoreDef::new(
        EntryStoreId(id),
        TripleBufferId::DEFAULT,
        EntryStoreConfig {
            core_stride: core,
            meta_stride: meta,
            attr_stride: attr,
            capacity: cap,
        },
    )
}

fn tb_def(id: u16, cap: usize) -> TripleBufferDef {
    TripleBufferDef {
        id: TripleBufferId(id),
        buffer_capacity: cap,
    }
}

/// Constructs a writer registry (no reader yet). All stores on DEFAULT TB for simplicity.
fn make_writer_registry<const STORE_COUNT: usize>(
    defs: [EntryStoreDef; STORE_COUNT],
) -> (
    TripleBufferWriterRegistry<1>,
    EntryStoreWriterRegistry<1, STORE_COUNT>,
) {
    let mem = create_mem(MEM_SIZE);
    let tb_defs = [tb_def(0, 1024)];
    let tb_reg = TripleBufferWriterRegistry::<1>::new(Arc::clone(&mem), tb_defs, 0, 4096);
    let writer_reg = EntryStoreWriterRegistry::<1, STORE_COUNT>::new(
        Arc::clone(&mem),
        tb_reg.clone(),
        defs,
        tb_reg.mem_end_offset(),
        0,
        [0; 1],
    );
    (tb_reg, writer_reg)
}

/// Constructs a writer registry and immediately converts to reader registry.
/// All stores on DEFAULT TB for simplicity.
fn make_reader_registry<const STORE_COUNT: usize>(
    defs: [EntryStoreDef; STORE_COUNT],
) -> (
    TripleBufferWriterRegistry<1>,
    EntryStoreWriterRegistry<1, STORE_COUNT>,
    EntryStoreReaderRegistry<1, STORE_COUNT>,
) {
    let (tb_reg, writer_reg) = make_writer_registry(defs);
    let reader_reg = writer_reg.to_reader();
    (tb_reg, writer_reg, reader_reg)
}

#[test]
fn bind_produces_valid_registry() {
    let defs = [sdef(0, 8, 0, 16, 4)];
    let (_tb, _wr, rr) = make_reader_registry(defs);
    assert!(rr.len() > 0);
    assert!(rr.mem_end_offset() > rr.mem_start_offset());
}

#[test]
fn get_returns_reader_for_valid_id() {
    let defs = [sdef(0, 8, 0, 16, 4)];
    let (_tb, wr, rr) = make_reader_registry(defs);
    assert_eq!(
        rr.get(EntryStoreId(0)).capacity(),
        wr.get(EntryStoreId(0)).capacity()
    );
    assert_eq!(
        rr.get(EntryStoreId(0)).mem_start_offset(),
        wr.get(EntryStoreId(0)).mem_start_offset()
    );
}

#[test]
fn get_identity_permutation() {
    let defs = [sdef(0, 8, 0, 16, 4), sdef(1, 4, 4, 8, 8)];
    let (_tb, _wr, rr) = make_reader_registry(defs);
    assert!(
        rr.get(EntryStoreId(0)).mem_start_offset() < rr.get(EntryStoreId(1)).mem_start_offset()
    );
}

#[test]
fn get_reversed_permutation() {
    let defs = [sdef(1, 8, 0, 16, 4), sdef(0, 4, 4, 8, 8)];
    let (_tb, _wr, rr) = make_reader_registry(defs);
    assert!(
        rr.get(EntryStoreId(1)).mem_start_offset() < rr.get(EntryStoreId(0)).mem_start_offset()
    );
}

#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "out of bounds")]
fn get_out_of_range_panics() {
    let defs = [sdef(0, 8, 0, 16, 4)];
    let (_tb, _wr, rr) = make_reader_registry(defs);
    let _ = rr.get(EntryStoreId(1));
}

#[test]
fn len_equals_mem_span() {
    let defs = [sdef(0, 8, 0, 16, 4)];
    let (_tb, _wr, rr) = make_reader_registry(defs);
    assert_eq!(rr.len(), rr.mem_end_offset() - rr.mem_start_offset());
}

#[test]
fn attr_read_visible_without_swap() {
    let defs = [sdef(0, 8, 0, 16, 4)];
    let (_tb_reg, writer_reg) = make_writer_registry(defs);
    let store = writer_reg.get(EntryStoreId(0));
    let slot = store.insert().unwrap();
    store.get(slot).attr_write(0, 424242);
    let reader_reg = writer_reg.to_reader();
    assert_eq!(
        reader_reg.get(EntryStoreId(0)).get(slot).attr_read(0),
        424242
    );
}

#[test]
fn core_read_visible_after_publish_swap() {
    let defs = [sdef(0, 8, 0, 16, 4)];
    let (tb_reg, writer_reg) = make_writer_registry(defs);
    let store = writer_reg.get(EntryStoreId(0));
    let slot = store.insert().unwrap();
    store.get(slot).core_write_all(&[5, 6, 7, 8, 9, 10, 11, 12]);
    tb_reg.get(TripleBufferId::DEFAULT).publish();
    let tb_rr = tb_reg.to_reader();
    tb_rr.get(TripleBufferId::DEFAULT).swap();
    let reader_reg = writer_reg.to_reader();
    let mut buf = [0i32; 8];
    reader_reg
        .get(EntryStoreId(0))
        .get(slot)
        .core_read_all(&mut buf);
    assert_eq!(buf, [5, 6, 7, 8, 9, 10, 11, 12]);
}
