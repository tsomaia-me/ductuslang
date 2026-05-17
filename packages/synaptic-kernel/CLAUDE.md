# synaptic-kernel — Agent instructions

## Read order

1. `ARCHITECTURE.md` — what the kernel is and how its parts relate. Read before any non-trivial change.
2. `CONVENTIONS.md` — how to write code that fits this kernel. Read before writing code.
3. Source — only after both above. Source comments document local invariants; they do not document architecture.

## Hard rules — violating these breaks the kernel

- **Consumer must enter the graph at a known-live slot every swap.** The kernel does not designate a root or maintain a
  registry of entry points — that's a user-side concern (typically a slot stored in `mem_metadata`). Within one cycle
  (between two `swap()` calls), pointers acquired by traversing from the entry slot via `next_ptr` / `prev_ptr` /
  synapse adjacency are safe. Across cycles, cached slot indices are unsafe — after `swap()`, the producer may have
  reclaimed deferred slots and reallocated them, so the same slot index may now reference a different entity.
  Re-enter from the user-designated entry slot every cycle.

- **Consumer must be quiesced before `Kernel` drop or `serialize()`.** Drop unconditionally frees the deferred-deletion
  queue. Serialize captures memory mid-flight. Either while the consumer is active is undefined behavior. In debug
  builds, `Kernel::drop` asserts `Arc::strong_count(&control_plane) == 1` and panics on violation; release skips the
  check. The recommended pattern is `consumer_thread.join()` before `drop(kernel)`, or declare the consumer after
  the kernel so reverse-declaration drop order handles it.

- **No allocation on the producer hot path.** Allocation is permitted only inside `Kernel::new`, `load_serialized`,
  `grow`, and the internal `Box`/`Box::from_raw` for mirror swap. Anywhere else, you've broken wait-freedom.

- **Every primitive that backs a memory region must support `bind`.** `new` zero-initializes; `bind` assumes valid
  existing state. Serialization replay needs both. Adding a primitive without `bind` breaks `load_serialized`.

- **User TB / store / LUT IDs must form a permutation of `[0, N-1]`.** No gaps, no duplicates.
  `TripleBufferId::DEFAULT` (`u16::MAX`) is reserved and must not appear in user `tb_defs`.

- **Atomic ordering: `Relaxed` is the default.** `Acquire`/`Release`/`AcqRel` appear only at the publish/swap fences,
  the staging-buffer generation handshake, and the `ControlPlane` epoch handshake. If you reach for `SeqCst` on a
  payload read, you've misunderstood the protocol — surface the question first.

- **Slots are typed as `SlotId(NonZeroU32)`.** Slot 0 is unrepresentable at the API layer — the type prevents it.
  "No slot" is `None` in `Option<SlotId>`, niche-optimized to a 0 wire value. The wire format (i32 cells in
  `AtomicBuffer`) still uses `0 = no slot`; conversion happens in `SlotId::from_i32` / `SlotId::option_to_i32` at
  the API boundary. Slot APIs are 1-based.

- **Producer thread / consumer thread separation is a contract, not enforced by types.** Every `*Writer` / `Epoch` /
  `Kernel` method is producer-only. Every `*Reader` / `EpochMirror` / `EpochConsumer` method is consumer-only. The
  `Arc<ControlPlane>` is the only legal cross-thread bridge.

- **`grow()` is monotonic and schema-preserving.** Every capacity dimension in the new config must be `>= current`
  (returns `KernelError::InsufficientCapacity` otherwise). Every schema field — strides, `tb_id` assignments,
  ID sets — must match exactly between old and new config (returns `KernelError::SchemaMismatch` otherwise).
  There is no shrink path. Only capacities may grow.

- **No domain concepts in the kernel.** The kernel is topology-agnostic. It does not know what a root, clip, group,
  component, or entry point is. Sub-chains and synaptic components are emergent from the link structure, not first-class
  kernel concepts. If you find yourself adding a "root pointer," "head registry," "clip ID," or anything that names a
  user-level structure, stop — that belongs in the consumer (e.g. SymphonyEngine), not here.

- **Chain links are acyclic; synapses are arbitrary.** `next_ptr` / `prev_ptr` form doubly-linked sub-chains and cannot
  form cycles (mutators preserve invariants; direct setters are `pub(crate)`). The synapse graph may contain cycles,
  multi-edges, and self-loops — the kernel does not check or care. If a higher layer needs acyclic synapses, it enforces
  that itself.

## Where to look

- Architecture overview, planes, generation stack: `ARCHITECTURE.md`
- Construction patterns, naming, sizing, atomic rules, terminology: `CONVENTIONS.md`
- Test harness shared config: `tests/common/mod.rs`
- Real-thread SPSC test patterns: `tests/kernel_concurrent_test.rs`, `tests/triple_buffer_test.rs`
- Property-test oracle pattern: `tests/staging_buffer_prop_test.rs`
- Hot-swap stress patterns: `tests/epoch_stress_test.rs`

## When in doubt

If a change touches memory layout, threading, the generation protocol, or slot-allocator behavior — stop. Surface the
design question before writing code. The kernel's invariants are tightly coupled; a "small fix" in one place often
invalidates a guarantee in another.
