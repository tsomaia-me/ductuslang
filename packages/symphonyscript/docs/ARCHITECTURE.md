# SymphonyScript Architecture

This document describes how the implementation realizes `SPEC.md`. The SPEC defines what programs mean; this document defines the crates, data structures and interfaces that compute it. A conflict between the two is a defect the owner resolves.

## 1. Crates

```text
packages/symphonyscript/
  CLAUDE.md
  Cargo.toml
  docs/
  crates/
    core/        symphonyscript-core
    compiler/    symphonyscript
    tool/        symphonyscript-tool
    kernel/      symphonyscript-kernel
    fixtures/    symphonyscript-fixtures
  editors/
    vscode/
```

| Crate | Environment | Contents | Depends on |
|---|---|---|---|
| `symphonyscript-core` | `no_std` with `alloc` | values and types, the vocabulary and its builder, the Program, the runtime, the `Consumer` trait | `libm` |
| `symphonyscript` | `std` | the compiler, `compile`, `eval`, diagnostics, Rust code generation | core |
| `symphonyscript-tool` | `std` | the framework tool: CLI, watch loop, language server | compiler |
| `symphonyscript-kernel` | `std` | the `Consumer` for synaptic-kernel | core, synaptic-kernel |
| `symphonyscript-fixtures` | `std`, tests only | the example vocabulary of SPEC §1.3, the fixture projects (PLAN Phase 1) | core |

Only `symphonyscript-core` runs on devices. Everything in it builds without `std`, and its float functions come from `libm` on every target, so desktop and device compute bit-identical values (SPEC §12.5).

## 2. The vocabulary

A framework author builds a vocabulary in Rust. `v.node(name)` and `v.wire(name)` return a kind's typed handle at once, and `v.kind(handle)` and `v.wire_kind(handle)` configure it, so a kind can accept itself or a kind declared after it. The framework's consumer keeps the handles and matches them against the Program. A part of the SPEC §1.3 vocabulary:

```rust
let mut v = Vocabulary::builder("music");

let beats = v.unit_family("beats").base("beats").unit("bars", 4.0);
let wave = v.enumeration("Wave", ["Sine", "Saw", "Square"]);

let note = v.node("Note");
let clip = v.node("Clip");
let track = v.node("Track");
let osc = v.node("Osc");
let session = v.node("Session");
let plays = v.wire("Plays");
let starts = v.wire("Starts");

v.kind(note)
    .doc("A single pitched event inside a clip.")
    .attr::<i32>("pitch").default(60).default_attr()
    .attr::<f32>("velocity").default(1.0).doc("Loudness, 0 to 1.")
    .attr::<f32>("brightness").default(0.5)
    .attr::<f64>("length").default(0.25).family(beats, "beats")
    .attr::<bool>("accent").default(false).flag('\'');

v.kind(clip)
    .top_level(Count::Any)
    .children(note, Count::Any)
    .attr::<f32>("gain").default(1.0)
    .attr::<f32>("level").default(1.0);

v.kind(track)
    .top_level(Count::Any)
    .attr::<&'static str>("label").default("");

v.kind(osc)
    .top_level(Count::Any)
    .attr::<f64>("freq").default(440.0).default_attr()
    .attr_enum("wave", wave, "Sine")
    .attr::<f32>("rate").default(1.0);

v.kind(session)
    .top_level(Count::AtMost(1))
    .entry()
    .children(track, Count::Any)
    .attr::<f64>("tempo").default(120.0).default_attr();

v.wire_kind(plays)
    .connects(clip, clip)
    .cyclic()
    .attr::<f32>("gain").default(1.0).default_attr()
    .attr::<i32>("transpose").default(0);

v.wire_kind(starts)
    .connects(clip, clip)
    .connects(track, clip)
    .cyclic();

v.prelude("music.sym", include_str!("music.sym"));

let vocab = v.build()?;
```

- `attr::<T>` takes `i32`, `i64`, `f32`, `f64`, `bool`, fixed-length arrays of these nested to any depth (`[bool; 8]`, `[[i32; 3]; 2]`), and `&'static str` for string attributes. The Rust compiler checks the default against `T`. `attr_enum(name, enum, default_variant)` declares an enum attribute, and `attr_enum_array(name, enum, length, default_variant)` an array of one.
- `Count` is `Exactly(n)`, `AtLeast(n)`, `AtMost(n)`, `Between(n, m)` or `Any`.
- `.children(kind, count)` accepts a child kind. Each `.connects(from, to)` adds a source and destination pair to a wire kind. `.incoming(wire, count)` and `.outgoing(wire, count)` bound how many wires of one kind a node kind takes; a kind that states no bound takes any number.
- `.doc(text)` attaches documentation to kinds, attributes, wires, enums and units. The language server shows it.
- `build()` validates the whole vocabulary at once and returns every error SPEC §10.1 lists, except the prelude's, which the compiler reports (§7).
- A built vocabulary carries a stamp: its name and a 64-bit FNV-1a hash of a canonical serialization of its content, identical on every target. The serialization holds the kinds and wire kinds in declaration order, which fixes their indices, with their attributes (names, types, defaults, families, default units, flags and default-attribute marks), counts, connections, entry and cyclic marks; the enums and their variants; the unit families and scales; and the prelude's text with its line endings normalized to LF. Documentation is not part of it, so a `.doc` edit leaves the stamp unchanged.

The prelude is SymphonyScript source. `build()` cannot compile it, because the compiler is not in core; the compiler handles it (§7).

## 3. The Program

```rust
pub struct Program {
    pub vocabulary: VocabularyStamp,
    pub top_level: Vec<InstanceId>,
    pub instances: Vec<Instance>,
    pub wires: Vec<Wire>,
    pub inputs: Vec<Input>,
    pub cells: Vec<Cell>,
    pub strings: Vec<String>,
}

pub struct Instance {
    pub kind: NodeKind,
    pub path: Path,
    pub span: Span,
    pub expanded_from: Vec<Span>,
    pub attrs: Vec<(AttrId, Binding)>,
    pub gate: Option<Binding>,
    pub sequence: Vec<Entry>,
}

pub enum Entry {
    Child(InstanceId),
    Bundle(Vec<InstanceId>),
    Wire(WireId),
}

pub struct Wire {
    pub kind: WireKind,
    pub path: Path,
    pub span: Span,
    pub expanded_from: Vec<Span>,
    pub source: InstanceId,
    pub target: Target,
    pub attrs: Vec<(AttrId, Binding)>,
    pub gate: Option<Binding>,
}

pub enum Target {
    Fixed(InstanceId),
    Select { candidates: Vec<InstanceId>, index: Binding },
}

pub enum Binding {
    Const(Value),
    Input(InputSlot),
    Cell(CellId),
    Array(Vec<Binding>),
}

pub struct Input {
    pub name: String,
    pub ty: Ty,
    pub family: Option<FamilyId>,
    pub slots: Range<InputSlot>,
    pub initial: Vec<Value>,
    pub span: Span,
}

pub struct Cell {
    pub ty: Ty,
    pub expr: Expr,
    pub span: Span,
}
```

- `instances[i]` has `InstanceId(i)`; the same holds for wires, cells and input slots. Instances, wires and inputs are in the order of SPEC §11.
- An array input occupies one slot per element, numbered row by row (SPEC §12.1), so element writes address single slots. An array attribute binds `Binding::Array`.
- `cells` is in dependency order: a cell reads only inputs and cells with lower indices.
- An `AttrId` indexes its kind's attribute list.
- `gate` combines the item's own `when` with the `when` arm conditions around it in one binding (SPEC §11).
- `Select` keeps the candidates in the order of SPEC §8.2; `index` is an integer binding, clamped to them.
- `Expr` is a tree of constants, input and cell reads, typed unary and binary operators, `if`, `match`, `Pick` (indexing with a runtime index, clamped), conversions and built-in calls. Every operator node carries its operand type, and every read of a cell that its use reads in another type carries an explicit conversion (SPEC §4.4). Functions, patterns, units and compile-time values are already resolved; nothing in `Expr` names a source declaration.
- `Value` is `I32`, `I64`, `F32`, `F64`, `Bool`, `Enum(EnumId, u16)` or `Str(StrId)`. A `StrId` indexes `strings`, which holds each string attribute value once.
- `Span` is a file, named by its `Source.path`, and a byte range, used in diagnostics and tooling. `span` is where an item is written; `expanded_from` holds the pattern uses that produced it, innermost first, and is empty outside patterns.

`Program` implements structural equality that compares floats by their bits, which the canonical NaN of SPEC §12.5 keeps stable, so two Programs compiled from the same sources compare equal.

## 4. The runtime

```rust
let mut rt = Runtime::new(program)?;
let cutoff = rt.input("cutoff").unwrap();
let steps = rt.input("steps").unwrap();

rt.write(cutoff, Value::F32(0.7))?;
rt.write_element(steps, 3, Value::Bool(true))?;

rt.commit();
for change in rt.changes() {
    match change {
        Change::Attr { instance, attr, element } => { let v = rt.attr(instance, attr, element); }
        Change::WireAttr { wire, attr, element } => { let v = rt.wire_attr(wire, attr, element); }
        Change::Active(instance) => { let on = rt.active(instance); }
        Change::WireActive(wire) => { let on = rt.wire_active(wire); }
        Change::Target(wire) => { let to = rt.target(wire); }
    }
}
```

- `Runtime::new` validates the Program (ids in range, cells in dependency order, operand types consistent) and returns an error for one that fails. It then evaluates every cell, computes every activation and target, and allocates every buffer the runtime ever uses.
- `commit` never allocates. It walks the cells in index order, evaluating only cells marked dirty, and marks readers dirty when a value changes bit-for-bit. It fills the change buffer, whose capacity is the number of reportable items, in the order of SPEC §12.3. `changes` reads that buffer until the next commit.
- Each input, cell and gate has a precomputed list of the attributes, gates and targets that read it, so changes are reported without searching.
- Effective activation is computed parents first, so a gate change reaches every descendant in one pass.
- `Expr` is evaluated without recursion, on a stack sized in `Runtime::new` for the deepest expression, so no Program overflows the native stack.
- `write`, `write_element` and `write_array` return an error for a write SPEC §12.1 rejects and for a dead handle. Nothing in the runtime panics.
- `program` returns the loaded Program.
- The runtime is `Send` and is used from one thread. Consumers move changes to other threads themselves.

## 5. Reload

```rust
match rt.prepare_reload(new_program) {
    Ok(pending) => {
        let report = pending.report();
        for item in report.structure() {
            match item {
                Structural::Kept { old, new } => {}
                Structural::Added(new) => {}
                Structural::Removed(old) => {}
                Structural::Reordered(parent) => {}
            }
        }
        for write in report.dropped() {}
        for change in report.changes() {}
        rt.install(pending);
    }
    Err(Refused::Vocabulary | Refused::Invalid) => {}
}
```

`structure()` covers instances, bundles and wires (`Item::Instance`, `Item::Bundle` and `Item::Wire`); `Reordered` names a parent instance, a bundle or `Parent::Top`. `prepare_reload`:

1. Refuses a Program with a different vocabulary stamp (`Refused::Vocabulary`), or one that fails `Runtime::new`'s validation (`Refused::Invalid`).
2. Carries input values over by name, following SPEC §13.2. Each input element records whether the host has written it.
3. Matches instances, bundles and wires following SPEC §13.3, from the top level down: paths below matched parents for named items; then, per parent sequence, the common prefix and suffix, and Myers' difference algorithm over the keys of the remaining anonymous entries; then per-kind pairing in each gap.
4. Builds the new state and evaluates every cell.
5. Reports the structure, the staged writes it dropped, and the attribute, activation and target differences of matched items.

`install` replaces the running state with the pending one. Dropping a `PendingReload` instead leaves the runtime on the old Program. Preparing may allocate; installing swaps buffers and does not. Input handles are stable: the runtime keeps a registry from input names to handles across reloads, adds handles for new names, marks handles of removed inputs dead, and revives a dead handle when an input of its name returns.

## 6. The Consumer trait

```rust
pub trait Consumer {
    type Error;
    fn load(&mut self, runtime: &Runtime) -> Result<(), Self::Error>;
    fn apply(&mut self, runtime: &Runtime);
    fn reload(&mut self, pending: &PendingReload) -> Result<(), Self::Error>;
}
```

A consumer turns a Program into something: a synaptic-kernel graph, a MIDI file, a test trace, a printed dump. It reads the Program through `runtime.program()` and each commit's changes through `runtime.changes()`. The tool's watch loop drives any consumer through this trait. `load` refuses a Program the consumer cannot take, such as one whose vocabulary stamp differs from the vocabulary the consumer was built for. `reload` reads the new Program and the report from the pending reload and either applies all of it or changes nothing and returns an error. The watch loop installs a pending reload only after the consumer's `reload` succeeds, so a refusal leaves the runtime and the consumer on the old Program, and the loop reports the error.

## 7. The compiler

```rust
pub struct Compiler { /* the vocabulary and its parsed, resolved prelude */ }

impl Compiler {
    pub fn new(vocab: &Vocabulary) -> Result<Compiler, Failed>;
    pub fn compile(&self, sources: &[Source]) -> Result<Compiled, Failed>;
}

pub fn compile(sources: &[Source], vocab: &Vocabulary) -> Result<Compiled, Failed>;
pub fn eval(source: &str, vocab: &Vocabulary) -> Result<Runtime, Failed>;

pub struct Source { pub path: String, pub text: String }
pub struct Compiled { pub program: Program, pub warnings: Vec<Diagnostic> }
pub struct Failed { pub diagnostics: Vec<Diagnostic> }
```

`Compiler::new` parses and resolves the prelude once and reports its errors as vocabulary errors (SPEC §10.1). The tool and the language server keep one `Compiler`; `compile` and `eval` build one for a single call. `Source.path` is relative to the project's source root, with `/` separators.

The compiler performs no file I/O; the caller supplies every source. This keeps the compiler usable in editors, build scripts and WebAssembly. Stages:

1. **Lex.** Layout tokens (SPEC §2.2), literals, keywords, unit suffixes looked up in the vocabulary. Every byte belongs to a token or to trivia (whitespace and comments).
2. **Parse.** A lossless syntax tree that reproduces the source byte for byte, with a typed view over it. The parser recovers from errors: it records a diagnostic, skips to a synchronization point (the next line at the same or lower indentation) and continues, so a broken file still yields a tree. A placement whose kind is a wire kind of the vocabulary parses as a wire placement.
3. **Collect.** The top-level namespace across all files, the prelude and the vocabulary, with duplicate detection and prelude shadowing; the call graph of functions, which marks recursive ones, and the use graph of patterns.
4. **Expand.** Pattern uses are expanded and `for` loops unrolled. A value that structure needs (a range bound, an iterable, a recursive call, and any argument or `let` they read) is resolved, typed as SPEC §4.6 says and evaluated when expansion reaches it; such a value that reads a cell is an error. Every expanded item keeps the spans of its uses and of its definition.
5. **Resolve.** Names, paths and `to` in the expanded program; calls to non-recursive functions are inlined with their parameters bound (SPEC §6.3); each value is classified as compile-time or reactive by what it reads. Cycles among declarations (SPEC §6.2) are found here, before Infer. A bare name that names no declaration is left for Infer to resolve as a variant.
6. **Infer.** Types and unit families over the whole program (SPEC §3.6, §4), bare variants from their expected types, and then the remaining calls to recursive functions, evaluated in the types inference gives them. Recursive calls are memoized by argument values, argument types and expected type, which is what SPEC §4.6's limit counts.
7. **Check.** The vocabulary checks (SPEC §10.2), cycles among wires, fit checks, and the warnings of SPEC §4.4 and §10.2.
8. **Lower.** Constant folding with the canonical NaN, the cell graph in dependency order, paths, filled defaults, the string table, the Program.

Diagnostics carry a stable code in kebab-case (`unknown-attribute`, `count-exceeded`), a primary span, secondary spans (for a pattern use: the use and the line in the pattern), a plain-language message, and a hint when one exists. Every code is a variant of the compiler's `Code` enum, whose documentation names the SPEC section of its rule. The tests keep a table mapping each error rule of the SPEC, by section and sentence, to its code; a test checks that every code appears in the table and every code in the table exists. The compiler collects every diagnostic of a run instead of stopping at the first.

Each change recompiles the whole project.

## 8. Delivery

- **Library.** A host calls `compile` and `Runtime::new` in its own process. Edits reload live. `eval` compiles and loads a single source in one call.
- **Generated Rust.** `symphonyscript::codegen::generate(sources, &vocab) -> Result<String, Failed>` returns the text of a Rust module containing `pub fn program() -> Program`. A framework's `build.rs` reads the sources, calls it and writes the module into `OUT_DIR`; the firmware or application includes it and never contains the compiler. The vocabulary lives in its own crate, used by the framework both as a dependency and as a build-dependency, because a build script cannot call the crate it builds. Generated builds do not reload.

A differential test compiles every fixture project both ways and requires equal Programs and equal commit traces.

## 9. Tooling

A framework ships one binary, built from its vocabulary:

```rust
fn main() {
    symphonyscript_tool::Tool::new(music::vocabulary())
        .command("play", music::play)
        .command("export-midi", music::export_midi)
        .run()
}

fn play(project: &Project, args: &[String]) -> Result<(), ToolError> {
    symphonyscript_tool::watch(project, &music::vocabulary(), &mut music::Engine::new())
}
```

| Subcommand | Provided by | Does |
|---|---|---|
| `new <name>` | tool | creates a project: `symphony.toml` and `src/` with a starter file |
| `check` | tool | compiles and prints diagnostics |
| `dump` | tool | prints the Program |
| `lsp` | tool | runs the language server |
| any other | framework | playing, rendering, exporting |

- **Commands.** A framework command is `fn(&Project, &[String]) -> Result<(), ToolError>`, receiving the project and the command's arguments.
- **Projects.** A `Project` holds its root, the settings of its `symphony.toml`, and its sources: every `.sym` file under `source`, found recursively, each with its path relative to `source`. Those paths fix the top-level order (SPEC §2.1).
- **Single files.** `check`, `dump` and the framework's subcommands accept a lone `.sym` file in place of a project; it is a project of one source whose path is the file's name.
- **Watch.** `watch(project, vocab, consumer)` recompiles on save, prepares a reload, lets the consumer accept it, installs it, and drives the consumer (§5, §6). Frameworks call it from their own commands.
- **`symphony.toml`** holds `tool`, the name of the framework's binary, and `source`, the source root (default `src`).
- **Editor.** One VS Code extension: a static syntax grammar plus a language client that reads `symphony.toml` and starts `<tool> lsp`. For a lone `.sym` file it starts the tool named in its `symphonyscript.tool` setting; with no setting it stays inactive and says why. Other editors use their own language-client configuration.
- **Language server.** Diagnostics; completion of attributes after `|`, accepted child kinds in a body, valid destinations after a wire's `:`, enum variants where an enum is expected; hover with documentation and inferred types with their origin, where a compile-time value shows its value and the type each use chose, and a cell shows that it is reactive, its one type and the inputs it reads; go to definition; find references; rename of instances, patterns, functions, inputs and `let`s; an outline of top-level items; semantic tokens, which mark inputs and reactive `let`s apart from compile-time values.

## 10. The synaptic-kernel adapter

`symphonyscript-kernel` implements `Consumer` for synaptic-kernel. It is generic over vocabularies: it maps structure and values, and the engine on the kernel's consumer thread interprets kinds.

### 10.1 Structure

| Program | Kernel |
|---|---|
| Instance | a node; `kind` is the vocabulary's node-kind index |
| An instance's sequence | a sub-chain that starts with a head node and holds the children in sequence order; a `contains` synapse goes from the instance to the head |
| Bundle | a bundle node in the parent's chain; its members form a sub-chain with its own head node behind another `contains` synapse |
| Wire | a wire-point node at the wire's position in the chain, holding the wire's attributes, activation and serial; a synapse whose `kind` is the wire-kind index goes from it to the destination |
| Destination chosen at runtime | one synapse per distinct candidate, each holding a flag that marks the chosen one |
| Top-level instances | a sub-chain after a root sentinel node, in top-level order |
| Entry | the sentinel's slot, written to `mem_metadata` once at load and never changed |

`kind` is 8 bits. The adapter reserves four node kinds (sentinel, head, bundle and wire point) and one synapse kind (`contains`), so a vocabulary used with the adapter has at most 252 node kinds and 255 wire kinds. `load` refuses a larger vocabulary.

The engine enters at the sentinel on every cycle, walks the top-level chain, and follows `contains` synapses to the heads of sequences. A head never moves, so reordering a sequence never touches its `contains` synapse.

### 10.2 Values

Each node and synapse holds the adapter's own cells, followed by attribute cells:

- **Adapter cells**, always in meta: a node's active flag and a serial number unique to the instance's or wire's lifetime, and a candidate synapse's chosen flag. The engine keys its per-instance state (playheads, sounding notes, phases) by serial, because a freed slot is reused by another instance.
- **Attribute cells**, placed by type:

```rust
fn plane(ty: &Ty) -> Plane {
    match ty {
        Ty::Array(..) => Plane::Array,
        Ty::I64 | Ty::F64 | Ty::Str => Plane::Meta,
        _ => Plane::Attr,
    }
}
```

A `bool`, an enum value, an `i32`, an `f32` and a string's `StrId` each take one cell; an `i64` or `f64` takes two, low half first; an array takes its elements' cells in row order. A value that fits one cell is untorn in the attribute plane and needs no publish. synaptic-kernel's cells are 32-bit, so a 64-bit value spans two cells and goes to meta, where `publish()` makes it visible all at once.

Array attributes never sit in a node's stride. A node with array attributes keeps them in a separate, triple-buffered array region, reached through an index in its meta cells, so only nodes with arrays pay for them, and they become visible at the same publish as meta. The adapter computes a layout table from the vocabulary once, giving every attribute of every kind a plane and an offset. The engine reads through the same table, so the two never disagree.

`load` and `reload` write the Program's string table into a kernel LUT, as UTF-8 bytes packed four to a cell after a table of offsets, and size the LUT from the table, so the engine reads strings without the Program. `StrId` cells live in meta, so a reload's new ids and its new table appear at the same publish.

`apply` writes each change to its location and calls `publish()` only when a change landed in meta or in the array region. The guarantee to engines, which is this consumer's statement under SPEC §12.2: every single value is consistent; meta-plane and array values are consistent with each other at every publish; independent attribute-plane values changed by the commits between two engine reads may be seen mixed in that read, and an attribute-plane value may be seen before a meta-plane value of the same commit, such as an activation, is published.

Strides are uniform per store: the meta stride and the attribute stride are the largest any kind's scalar attributes need.

### 10.3 Capacity and reload

`load` sizes the kernel from the Program's counts plus headroom equal to those counts. A removed slot counts until the engine has acknowledged the publishes that follow its removal, so `reload` publishes first, which lets the slots the previous reload freed be reclaimed, and then inserts added nodes, removes removed ones together with their children's chains, writes changes and publishes. A reload that replaces every node and synapse fits without `grow` once the engine has acknowledged the previous reload's publishes; a reload that outgrows capacity calls `grow`, and capacity only grows. Kept and reordered nodes move to their new chain positions with `move_node_after` and `move_node_before`, which keep a node's slot, synapses and cells and preserve the kernel's invariants. The adapter requires these from synaptic-kernel, and PLAN Phase 8 adds them.
