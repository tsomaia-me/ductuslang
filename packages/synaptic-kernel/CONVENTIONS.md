# synaptic-kernel — Conventions

Read after `ARCHITECTURE.md`. This file documents *how* code is structured. It does not repeat *what* the kernel does.

## Constructor triplet: `new` / `bind` / `create`

Every primitive that backs a memory region exposes:

- `pub fn new(mem, ..., capacity) -> Self` — zero-initialize the region.
- `pub fn bind(mem, ..., capacity) -> Self` — assume the region holds valid existing state. Used during
  `load_serialized` and writer-side replay.
- `pub fn create(mem, ..., capacity, bind: bool) -> Self` — the actual constructor. `new` and `bind` are thin wrappers.

Why three: serialization replay needs to rebuild writers without zeroing live state. Both entry points share validation
and offset arithmetic via `create`. If you add a new primitive backed by `AtomicBuffer`, follow this exactly.

`bind` may need to perform an `Acquire` load to pick up the most recent published state and re-sync internal scratch.
See `TripleBufferWriter::create` (the `bind` branch loads `state` with `Acquire` and calls `sync`).

## Writer / Reader pairing

Every writer type has a corresponding reader type. Construction is asymmetric:

- Writer is built directly: `XWriter::new(mem, offsets, ...)`.
- Reader is built from the writer: `let reader = writer.to_reader();`.

The reader cannot be constructed independently. Its constructor is `pub(crate) fn bind(...)`. This guarantees the two
share the same memory layout by construction — there is no other way to get them into agreement.

Thread placement:

- Writer types: producer thread only.
- Reader types: consumer thread only.
- `to_reader()` is called on the producer thread. The resulting reader is `Send` and is moved to the consumer.

## Sizing functions

Every primitive that consumes memory exposes:

- `calculate_size_on_mem(...) -> usize` — units of `i32` on the MEM plane.
- `calculate_size_on_tb(...) -> usize` (when applicable) — units of `i32` on a triple buffer.

These are pure, called pre-construction by the kernel to compute total buffer sizes. They must agree exactly with what
`create` lays out. A mismatch panics in debug, corrupts in release.

## Cursor pattern

Constructors take `mem_start_offset` and compute `mem_end_offset = mem_start_offset + size`. Both are stored on the
struct and exposed via accessors. The next primitive in layout consumes the previous one's `mem_end_offset` as its
`mem_start_offset`. `Epoch::create` is the canonical example.

Same pattern for TB: `tb_start_offset` / `tb_end_offset`.

When laying out within a single TB across multiple stores (multi-store-per-TB), use a per-TB cursor variable. See
`EntryStoreWriterRegistry::create` (`extra_tb_cursors[index]`).

## Slot conventions

- Slots are 1-based. Slot `0` means "undefined" / "no slot."
- Internally, the free list and bitmap are 0-based. The `+ 1` / `- 1` conversion happens at the API boundary in
  `SimpleFreeList`.
- `is_active(slot)` checks both `is_allocated` and `!is_deferred`. Use `is_active` when you mean "safe to read." Use
  `is_allocated` when you mean "the slot has been handed out and not yet been returned to the free list."

## ID conventions for registries

- IDs occupy `[0, N-1]` where `N` is the registry's const generic.
- Each ID is unique. The user supplies definitions in any order; the registry validates uniqueness and range at
  construction.
- `TripleBufferId::DEFAULT == TripleBufferId(u16::MAX)` is reserved for the kernel-internal default TB. It must not
  appear in user `tb_defs`.
- Entry stores and LUTs declare which TB they live on via `tb_id`. They may target `DEFAULT` or any user TB.

## Plane / zone / buffer / registry — terminology

- **Plane** — visibility class. `MEM` (direct, immediately visible to consumer) or `TB` (triple-buffered,
  publish-gated). Two planes total.
- **Buffer** — a contiguous region on a plane. `AtomicBuffer` is the global one; each `TripleBufferWriter` owns three
  internal rotating buffers.
- **Zone** — a fixed-size view into a buffer at a specific offset. `MemZoneWriter`, `TbZoneWriter`, `TbZoneView`,
  `TbZoneReader`. Zones are short-lived, typically constructed per-call.
- **Entry** — a slotted record with three zones: `core` (TB, kernel-owned), `meta` (TB, user-owned), `attr` (MEM,
  user-owned). See `EntryWriter` / `EntryHandle` / `EntryReader`.
- **Registry** — fixed-size const-generic collection of writers indexed by typed ID. Always paired with a reader
  registry.

## Atomic ordering

- Default is `Relaxed`. Most reads and writes are protected by a coarser fence somewhere upstream.
- `Acquire`/`Release` appears at exactly four locations:
    - `TripleBufferWriter::publish` / `TripleBufferReader::swap` on `state` — the per-frame fence.
    - `RingBuffer` `pending_count` — push/read visibility.
    - `StagingBuffer` `reader_ack_generation` — generation handshake.
    - `ControlPlane` `writer_generation` / `reader_ack_generation` / `mirror_ptr` — the epoch handshake.
- `AcqRel` appears on the triple-buffer state swaps and on `ControlPlane::swap_epoch`.
- Never use `SeqCst`. If you think you need it, you've misunderstood the protocol — surface the question before adding.

## Visibility: `pub` vs `pub(crate)`

Structural setters on `NodeWriter` / `SynapseWriter` (`set_next_ptr`, `set_outgoing_synapse_head`, etc.) are
`pub(crate)`. Only the kernel mutates structural pointers — external mutation would break the cascade and link-list
invariants. User-domain setters (`set_meta`, `attr_*`) are `pub`. Preserve this split when adding new entity facades.

## Topology — what the kernel does and does not own

Working anywhere under `topology/`? Read this first.

- The kernel owns *primitives*: node slots, synapse slots, chain links (`next_ptr` / `prev_ptr`), synapse adjacency
  (`outgoing_*` / `incoming_*` heads/tails). It does **not** own *shapes*: there is no kernel concept of root, head,
  clip, group, component, or entry point. Sub-chains and synaptic components are emergent from the link structure.
- Two asymmetric organizing structures over the same node pool:
  - **Chain links** are acyclic by construction. The only mutators are `insert_node`, `insert_node_after`,
    `insert_node_before`, `remove_node`, and `remove_chain`. Direct setters are `pub(crate)` so users can't introduce
    cycles by hand.
  - **Synapse graph** is arbitrary. Cycles, multi-edges, and self-loops are all permitted. The kernel never checks.
- When adding to `NetworkWriter` / `NodeStoreWriter`: do not introduce a kernel-level "head registry," "root pointer,"
  or any structure that names a user-domain concept. If you find yourself typing `clip` or `root` in kernel code,
  stop. Push it to the consumer.
- `remove_node` cascades incident synapses; `remove_chain(head_slot)` is its sub-chain analogue, walking `next_ptr`
  from the given head and removing each. Both belong on `NetworkWriter`, not `NodeStoreWriter` — the cascade requires
  combined node+synapse access.
- The user-side entry point (e.g. SymphonyScript's "root clip") lives outside the kernel. Conventionally stored in
  `mem_metadata`. The consumer reads it there and calls `EpochMirror::get_node(slot)` to enter.

## Bit-packing

Node and synapse `kind` occupies the top 8 bits of `core[0]`; the lower 24 bits are reserved for internal flags. Range
is `[0, 256)`. Any change to the layout must update both `NodeWriter` / `NodeReader` / `NodeHandle` and
`SynapseWriter` / `SynapseReader` / `SynapseView` together.

## Asserts

Two-tier policy:

- **Constructors and `create` paths** use `assert!`. Caller-controlled invariants (capacity > 0, capacity is power of 2,
  ID range, layout fits in buffer). One-shot cost, fire loud, never optimized out.
- **Hot-path methods** (`read`, `write`, slot lookups, zone accessors) use `debug_assert!`. Caller is responsible for
  valid inputs in release.

The split is "called once per construction" vs "called per operation." When in doubt, follow the existing pattern in the
surrounding file.

## Const generics in registries

Registries use `const N: usize` plus an `id_index: [u16; N]` lookup. The internal sentinel for "unassigned slot" during
construction is `u16::MAX`. This collides nominally with `TripleBufferId::DEFAULT`'s value but never collides
functionally because registries short-circuit `DEFAULT` before indexing. If you add a new registry kind, follow the same
pattern; do not introduce a new sentinel scheme.

## Memory: `AtomicBuffer = Arc<[AtomicI32]>`

The `Arc` is the cross-thread bridge. Every primitive that holds a region clones the `Arc` (cheap — refcount bump, no
data copy). The buffer outlives the kernel struct precisely as long as something is still holding a clone — including
the deferred-deletion queue.

There is a long-standing TODO to switch to `Arc<[AtomicI32]>` to remove one indirection (Vec adds a heap hop). If you
change this type, every primitive has to be touched — that's why it hasn't happened yet.

## Tests

- Unit tests live next to source as `#[cfg(test)] mod tests`.
- Integration tests live under `tests/`. Per-primitive files (`bitmap_test.rs`, `ring_buffer_test.rs`) plus full-stack
  files (`kernel_test.rs`, `serialization_test.rs`, `epoch_stress_test.rs`).
- `tests/common/mod.rs` holds shared config builders. New tests should reuse it.
- Property tests use `proptest` and follow the **oracle pattern**: build a parallel naive model in safe Rust, fuzz a
  sequence of operations against both, assert state equality at every step. See `tests/staging_buffer_prop_test.rs` for
  the canonical example.
- Concurrent tests use real `std::thread`. Two-thread SPSC stress with thousands of iterations is the established
  pattern. See `tests/kernel_concurrent_test.rs`. (No `loom` yet — known gap, see `ARCHITECTURE.md`.)
- A new SPSC primitive needs at minimum: a single-threaded behavioral test, an oracle prop test, and a multi-threaded
  stress test.

### Test patterns

**Transient ack.** When a producer-side test needs to drive a generation
ack — usually to drain a deferred-free queue or close out a publish cycle
— but doesn't care about the mirror's contents and doesn't want to hold a
consumer across subsequent kernel mutations, use the one-line idiom:

```rust
TestConsumer::new(kernel.get_control_plane()).acquire_mirror();
```

This constructs the consumer, performs `acquire_mirror()` (which acks the
current generation), discards the returned `&EpochMirror`, and drops the
consumer at the end of the expression statement — all on a single line.
The consumer's `Arc<ControlPlane>` clone is released before the next
statement runs, so the kernel's debug-time Drop assert sees
`strong_count == 1` at scope end.

Use it inside the canonical two-cycle reclaim dance:

```rust
kernel.publish();
TestConsumer::new(kernel.get_control_plane()).acquire_mirror(); // ack
kernel.publish();
// deferred slots are now reclaimed
```

Don't use it when the test actually needs to read the mirror or sustain
the consumer across multiple kernel mutations — for those, declare the
consumer after the kernel as a normal binding so it lives for the rest of
the scope and drops first by reverse-declaration order.

## Files you'll touch together

When changing the shape of any of these, expect to touch the corresponding writer + reader + handle/view + their
registries. Skipping the reader side or the registry side is the most common partial-update bug.

- Node:    `node_writer.rs`, `node_reader.rs`, `node_handle.rs`, `node_store_writer.rs`, `node_store_reader.rs`
- Synapse: `synapse_writer.rs`, `synapse_reader.rs`, `synapse_handle.rs`
- Entry:   `entry_writer.rs`, `entry_reader.rs`, `entry_handle.rs`, `entry_store_*.rs`, `entry_store_*_registry.rs`
- TB:      `triple_buffer_writer.rs`, `triple_buffer_reader.rs`, `triple_buffer_*_registry.rs`, plus `tb_writer.rs` /
  `tb_reader.rs` / `tb_zone_*.rs`
- LUT:     `lut_writer.rs`, `lut_reader.rs`, `lut_*_registry.rs`

## Adding a new primitive — checklist

1. `XWriter::new` / `XWriter::bind` / `XWriter::create(... bind: bool)` triplet.
2. `XWriter::calculate_size_on_mem` (and `_on_tb` if applicable).
3. `XWriter::mem_start_offset` / `mem_end_offset` accessors.
4. `XWriter::to_reader() -> XReader`.
5. `XReader::bind` (`pub(crate)`).
6. `XWriter::copy_from(&Self)` if the primitive is involved in `grow()`.
7. Unit tests + oracle prop test + multi-threaded stress test.
8. If the primitive is registry-managed, add `XWriterRegistry<...>` / `XReaderRegistry<...>` following the existing
   registry pattern with `id_index: [u16; N]`.
