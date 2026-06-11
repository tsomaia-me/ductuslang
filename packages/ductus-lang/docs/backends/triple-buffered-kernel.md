# The Triple-Buffered Kernel — a Reference Backend

This document specifies the reference **triple-buffered SPSC kernel**: one
concrete runtime that satisfies the abstract contracts in `SPEC.md` — the
runtime interface (§13.14) and the Ductus IR (§15.4). Other backends may
realize those contracts by entirely different mechanisms. Nothing here is
normative for the *language*; it is normative only for this one concrete
backend.

This document's own sections use a `B.x` namespace, so they do not collide
with `SPEC.md`'s section numbers. References written as `§…` (without the `B`
prefix) point into `SPEC.md`; references written in the `B.x` form point to
sections within this document.

---
#### B.1 Triple-buffering

The reactive state buffer is **triple-buffered** to provide:

- Snapshot consistency across multiple cells for multi-cell values.
- Batched publication: writes accumulated in the back buffer commit
  atomically when the producer publishes.
- Wait-free reads from the consumer.

The arrangement is **single-producer, single-consumer (SPSC)**: one
*producer role* writes, one *consumer role* reads, mediated by three
buffer copies and an atomic current-pointer swap. The mapping of
these roles to physical threads, and the trigger that initiates a
publish, are implementation-defined; this document specifies only the
mechanism. Typical native deployments map the producer role to the
host's main thread and the consumer role to one or more application
threads.

The kernel maintains three copies of the buffer:

- **Current**: the most recently published snapshot. Read by the
  consumer. Not written while serving as current.
- **Back**: actively being written by the producer. Not read by the
  consumer.
- **Pending**: a third buffer used to allow the producer to begin
  writing the next batch of state without waiting for the consumer.
  Rotation among the three is producer-managed.

##### B.1.1 Publish operation

To publish accumulated writes, the producer performs:

1. Finalizes writes to the back buffer.
2. Atomically swaps the "current" pointer to point at the
   newly-written back buffer. The previous "current" rotates to
   become the next available back/pending buffer.

The publish operation runs on the producer role's thread. Its cost
is O(N) where N is the buffer size — the producer copies the
publishable state into the back buffer before the swap. The atomic
swap itself is O(1).

The "copy" here refers to the producer's carry-forward of unchanged
cells from the previous-current buffer to the back buffer, not a
re-copy of dirty cells. Dirty cells were written into the back buffer
incrementally between publishes per §13.10.1; the publish operation
copies forward only the cells the producer did not touch (so the new
back buffer is a complete snapshot). This carry-forward is O(N) in the
buffer size; the atomic swap itself is O(1).

The producer's per-publish cost is therefore O(N) memcpy + one
atomic operation. This cost is paid on the producer side, not on
the consumer side; consumers are unaffected.

When the producer chooses to publish (the trigger is specified in
§13.10 — the kernel's evaluation cycle) is outside the scope of this
section.

##### B.1.2 Swap operation

The consumer, when it wants to read the latest published state,
performs **swap**:

1. Atomically load the current pointer.
2. Read cells from the buffer it points to.

The swap operation runs on the consumer role's thread. Its cost is
O(1) — one atomic load. The consumer never copies data; it reads in
place from the buffer the current pointer points to.

##### B.1.3 Why three buffers

A two-buffer ping-pong would force the producer to wait for the
consumer to finish reading before publishing the next state. With
three buffers, the producer always has a buffer available to write
to that the consumer is not currently reading, even when the
consumer holds its reference into a snapshot for an extended period.
This preserves wait-free reads on the consumer side without
producer-side blocking.

##### B.1.4 Multiple cross-thread observers

If a deployment requires multiple cross-thread observers (multiple
consumers reading the same producer's published state), the SPSC
triple buffer can be replicated — each observer maintains its own
SPSC channel against the producer. SPMC variants are possible but
not required for the language's basic operation; the specification
defines SPSC as the canonical mechanism.


#### B.2 Wide-atomic optimization (optional)

On platforms with hardware support for 128-bit atomic operations
(x86_64 with `CMPXCHG16B`, ARM64 with `LDXP`/`STXP`), the
implementation may use in-place 128-bit atomic updates for `i128` and
`u128` cells rather than relying on the triple-buffer publish cycle.
This is an optimization, not a correctness requirement; the
triple-buffer mechanism provides correct semantics on all platforms.

Platforms without wide-atomic support (WebAssembly, ARM32, etc.) rely
exclusively on the triple-buffer mechanism. Programs using `i128` or
`u128` reactive cells on such platforms function correctly; they pay
the full per-publish cost for those cells.


### B.3 Producer and Consumer Roles

The triple-buffer mechanism (B.1) operates in terms of two roles:

- **Producer**: the role that writes the back buffer, runs publish
  cycles (evaluation + atomic swap). There is exactly one producer
  per kernel instance (SPSC). The producer may also read the back
  buffer it is writing; such reads are local to the producer and
  do not go through the triple-buffer pointer swap. What the
  producer writes (signal/attr updates from host API, derived and
  recurrent expression results) and what triggers it to publish
  are specified in §13.10.
- **Consumer**: the role that reads the current buffer via the swap
  operation. Loads the current pointer and reads cells from the
  buffer it points to. Never writes; never invokes behaviors. There
  is one consumer per SPSC channel; if multiple cross-thread
  observers are needed, each maintains its own SPSC channel
  (B.1.4).

This document specifies only the mechanism of these roles — what each role is
permitted to do, how the two coordinate via the triple buffer, and
the costs of the swap and publish operations. The mapping of roles
to physical threads and the choreography of what the producer does
between publishes are implementation-defined; the trigger that
initiates a publish is specified in §13.10 (the kernel's evaluation
cycle).

#### B.3.1 Thread-safety properties of the mechanism

By construction of the SPSC triple buffer:

- The producer writes the back buffer without interference; the
  consumer never touches it.
- The consumer reads the current buffer without interference; the
  producer never touches it.
- The atomic current-pointer swap is the synchronization point
  between producer and consumer.
- No locks are required, no spin-wait is required, and reads are
  wait-free.

These properties hold regardless of the role-to-thread mapping, which
is implementation-defined: in typical native deployments the host's
main thread plays the producer role and one or more application threads
play the consumer role; other deployments may assign a kernel-configured
thread to the producer role. The mechanism (B.1, B.3) does not
depend on the mapping choice.

#### B.3.2 Behaviors invoked by the mechanism

Reactive behaviors (derived expression bodies and recurrent
expression bodies) are invoked by the producer. Functions called from
reactive contexts are reactive-transparent per §13.12.2 and reached
transitively from registered behaviors; they are not themselves
separately invoked by the producer. The trigger, the selection of
which behaviors are invoked, and the ordering of invocations within
a publish cycle are all specified in §13.10.

The behavior ABI (§14.6) is the contract between the producer and
each invoked behavior. Each invocation receives a kernel handle
and an instance ID; behavior bodies read from and write to cells
via the handle. Behaviors are thread-safe by construction
(B.3.3).

#### B.3.3 Why Ductus behaviors are thread-safe by construction

Regardless of the role-to-thread mapping (implementation-defined per
B.3.1), Ductus source code never sees cross-thread concerns:

- No shared mutable state outside reactive cells.
- Reactive cells are coordinated through the triple-buffer
  mechanism above.
- Local `mut` bindings (§11) are stack-allocated and per-invocation.
- Closure captures are by-value Copy (§11.10), no shared mutability.

A Ductus program does not declare thread affinity; it does not
need to. The kernel determines (implementation-defined per B.3.1)
which thread plays which role.


#### B.4 Drop and triple-buffer eviction for dynamic-size cells

Dynamic-size cells (per §13.12.4 and §14.3.3) require eviction
ordering across triple-buffer rotation. When the kernel commits a
new value for a dynamic-size cell, the previous value is still
referenced by the rotating-out buffer slot until rotation makes
that slot the next back buffer.

**Rotation rule:**

A pool slot for a dynamic-size cell becomes eligible for `drop`
when no buffer references its index. Concretely:

1. Producer commits new value → new pool slot allocated → back
   buffer's index updated to new slot.
2. Atomic swap → back becomes current; previous-current's index
   still points at the old slot.
3. Consumer eventually reads the new current (catches up).
4. Next publish → previous-current rotates to back. At this point
   the back buffer's slot reference is replaced; the old slot is
   now unreferenced from any buffer.
5. The kernel runs `drop` on the old slot's value, then releases
   the slot to the pool's free list.

**For persistent data structures (e.g., `Vec[T]` as persistent
trie):** drop runs at the trie-node level rather than the whole-Vec
level. Internal nodes shared across versions remain alive until all
referencing versions have been evicted. The pool tracks per-node
refcounts; nodes drop when their refcount reaches zero.

**Drop ordering invariants:**

- A drop runs only after the value is unreferenced by any buffer.
- Drops run on the producer thread (or a dedicated reclamation
  thread), never on the consumer thread.
- Drops complete before the slot is reused — no in-place reuse of
  a slot whose drop hasn't finished.

The synchronization between rotation-out and drop+reclamation is
provided by the kernel's per-pool reclamation epoch: the slot enters
a quarantine state when its index is replaced; the pool's
reclamation thread (or the producer thread, depending on
implementation) advances the epoch atomically and runs drops on
quarantined slots before releasing them to the free list. No drop
runs while any buffer still references the slot's index.

**Drop and panic:** if `drop` panics on a dynamic-size cell value,
process abort applies per §14.7.4. The pool slot is leaked but the
process is terminating anyway.


### B.5 Lowering (Ductus → Rust)

The native-mode Rust emitter (§14.1.3) lowers the typed IR to Rust
source per the rules below. Interpreter-mode bytecode emission is
implementation-defined and out of scope for this section.

#### B.5.1 Type lowering

| Ductus                 | Rust                                                        |
|------------------------|-------------------------------------------------------------|
| `i8`–`i64`, `u8`–`u64` | Same Rust types.                                            |
| `i128`, `u128`         | Same Rust types (on supporting targets).                    |
| `f32`, `f64`           | Same Rust types.                                            |
| `bool`, `char`         | `bool`, `char`.                                             |
| `string`               | A newtype wrapping a kernel string-pool index (see B.5.1.1).|
| Tuples                 | Rust tuples.                                                |
| Arrays `T[N]`          | Rust arrays `[T; N]`.                                       |
| Records                | Rust structs with same field order.                         |
| Enums                  | Rust enums with same variant order.                         |
| Newtypes (§6.3)        | Rust newtype structs.                                       |

##### B.5.1.1 String storage uniformity

The `string` type lowers to the same Rust representation regardless
of whether the binding is reactive or non-reactive: a newtype around
a u64 index into the kernel's string pool (§14.5).

Reactive context (signal/attr/recurrent/derived value of type
`string`): the index lives in a reactive cell. The pool entry's
refcount tracks how many cells reference the string across all
buffer copies.

Non-reactive context (local `let s = "hello"`, function parameter,
record field outside reactive declaration): the index lives in
ordinary Rust memory. The pool entry is still refcounted; ownership
of the index increments the refcount, dropping the index (per
§14.7) decrements it. Strings created in non-reactive scopes are
reclaimed when their last index is dropped — typically when the
function returns and locals go out of scope.

This uniformity means: all `string` values share one storage backend
(the kernel pool), regardless of where their indices are held.
There is no separate "Rust-local string" representation distinct
from the "kernel string" representation; the only difference is
*where the index is stored* (cell vs ordinary memory), not what
the index points to.

The §11.6 "refcount-shared immutable backing" model maps directly
onto the kernel pool. The pool *is* the shared backing.

#### B.5.2 Function and trait lowering

Ductus resolves all generic instantiations and trait dispatch during
frontend processing (§14.1.1). Emitted Rust is fully monomorphic and
trait-free:

- A generic Ductus function `fn f[T](...)` becomes multiple
  monomorphic Rust functions, one per instantiation: `f_i32`,
  `f_f64`, etc.
- Trait method calls dispatch in Ductus to a specific function; the
  emitted Rust call is direct, not through a trait.
- Ductus traits are not declared in the emitted Rust. No `trait` or
  `impl` blocks appear (with the exception below).
- Operator overloading on Ductus numeric primitives uses Rust's
  built-in operators (`+`, `-`, etc.) directly; no trait emission
  needed.

The one exception: when a Ductus record overloads a Ductus operator
(e.g., a user-defined `Vec3` with `Add`), the emitter generates an
explicit `impl std::ops::Add for Vec3` block in Rust so that `+`
works on the type at the Rust level. This is a narrow mechanical
emission, not a full trait export.

#### B.5.3 Ownership lowering

Ductus's ownership rules map directly to Rust's:

| Ductus                            | Rust                      |
|-----------------------------------|---------------------------|
| `let x = e`                       | `let x = e;`              |
| `mut x = e`                       | `let mut x = e;`          |
| Default parameter (`v: T`)        | `v: &T` parameter.        |
| `own` parameter (`own v: T`)      | `v: T` parameter (move).  |
| `move v` at call site             | Pass by value (move).     |
| Default return (`fn f(v: T) -> T`) | `-> &T` rooted in input (lowerer infers from body's contributing inputs). |
| `own` return (`fn f(v: T) -> own T`) | `-> T` owned (Copy/Clone anchoring emitted by the lowerer when sourced from a cluster member). |
| `for x in v:` (default)           | `for x in &v` (borrows).  |
| `for own x in v:` (`for own`)     | `for x in v` (consumes).  |
| `Copy` types                      | `Copy` trait derived.     |
| `Clone`                           | `Clone` trait derived.    |

Ductus's user-facing surface omits the `&T` machinery; the lowerer
inserts the corresponding Rust borrow form when emitting code for
default-convention parameters and default-form for-loop iteration.
Rust's borrow checker enforces the same rules that Ductus's frontend
already verified through the cluster analysis (§11.3.4) and Rule (P)
(§11.3.5); any code that passed Ductus's checks passes Rust's.

#### B.5.4 Iterator lowering

Ductus's `Iterator` trait (§12.7) under the P3 design has signature:

```
fn next(own iter: Subject, source: Source) -> (Option[Item], Subject)
```

with two associated types (`Item`, `Source`). This shape does not
correspond directly to Rust's standard `Iterator` trait
(`fn next(&mut self) -> Option<Self::Item>`), because Ductus's
`Source` parameter is part of the trait contract — there is no
"hidden source field" inside the iterator.

The emitter generates a custom Rust trait per monomorphization that
mirrors Ductus's shape:

```rust
// emitted Rust (per Ductus iterator type)
trait DuctusIteratorXYZ {
    type Item;
    type Source;
    fn next(self, source: &Self::Source) -> (Option<Self::Item>, Self);
}
```

The `source` parameter is emitted as `&Source` (Rust borrow) under
Ductus's borrow-default convention. The `(Option<Item>, Self)`
tuple return is preserved at the trait level; the linear-ownership
optimization (§12.7.2) is applied by the lowerer when the for-loop
desugaring satisfies its three conditions, producing machine code
equivalent to in-place cursor mutation. The compiler MAY additionally
generate a Rust `std::iter::Iterator` wrapper for interop with
Rust-standard combinators, but the native form is the custom trait.

For-loop emission threads the source through each call:

```
for x in v:                      -- Ductus
  body
```

becomes:

```rust
// emitted Rust
let _src = v;                    // owns the source binding
let mut _iter = Iterable::iterator(&_src);
loop {
    let (opt, new_iter) = DuctusIteratorXYZ::next(_iter, &_src);
    _iter = new_iter;
    match opt {
        Some(value) => {
            let x = value;
            { body }
        },
        None => break,
    }
}
```

For the `for own` form, `_src` is moved into the iterator via
`IntoIterable::consuming_iterator(_src)`, and the `next` calls pass
`()` as the source parameter (per `Iter.Source = ()` constraint in
§12.9).

This translation is invisible to Ductus source code. Ductus users
never see `&` or `&mut` in their code or in error messages; the
emitted Rust is an implementation detail of the lowerer. The custom
trait approach is preferred over emitting against Rust's standard
`Iterator` because the standard trait cannot express the
`(Source, Item)` slot relationships natively without compiler tricks.

#### B.5.5 Reactive primitive lowering

Ductus's `signal`, `attr`, `recurrent`, and `derived` declarations
do not lower to Rust types directly. They lower to:

- Cell allocations in the kernel state buffer, described in the
  graph specification (§15.4).
- Behavior registrations (the body of a `derived` expression OR the
  body of a `recurrent`'s expression becomes a Rust function matching the
  behavior ABI, §14.6).
- Dependency edges in the graph specification.

The lowered Rust code contains no syntactic trace of `signal` /
`attr` / `recurrent` / `derived` keywords. They are pure
graph-construction directives, encoded into the graph specification
and behavior table.


#### B.6 Diff algorithm

The diff is computed entry-by-entry across each artifact field of
the graph specification:

- **Cells.** Matched by `id` (the fully-qualified declaration path
  of §15.4.1.1, identical to §13.15.2). Outcomes:
    - Same `id`, same `type` → **carried over** (kernel preserves cell
      value).
    - Same `id`, different `type` → **changed** (drop + re-allocate;
      initial value from `new_spec`).
    - `id` in `old_spec` only → **removed** (kernel drops per §14.7).
    - `id` in `new_spec` only → **added** (kernel allocates and
      initializes).

- **Connections.** Matched by `(from, to, connection_type)`. Diff
  semantics as for cells: matched → carried over; otherwise removed
    + added. Attr value changes on a matched connection are value
      changes, not identity changes; the connection itself is carried
      over and its attrs updated per the kernel mechanics of §14.8.

- **Behaviors.** Matched by content-addressed `behavior_id` per
  §14.6.3. Same ID → carried over (no rebinding needed). Different
  ID → removed + added (kernel rebinds function-pointer table).

- **Derived dependency edges, recurrent dependency edges, when-gates.**
  Set diff by their respective keys (derived cell ID, recurrent cell
  ID, gated instance ID).

